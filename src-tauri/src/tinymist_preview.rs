use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tauri::Emitter;
use tungstenite::Message;

use crate::project::paths::{join_under_root, MAIN_TYP};
use crate::AppState;

const DATA_PLANE_PREFIX: &str = "Data plane server listening on: ";
const CONTROL_PANEL_PREFIX: &str = "Control panel server listening on: ";

/// Prefer explicit `TINYMIST_PATH`, then the binary shipped in `resources/bin/`, then `PATH`.
///
/// The bundled path lives under `target/` during `tauri dev`. If we executed it in place, Linux
/// would keep the binary mapped (`ETXTBSY`) and `cargo build` could not overwrite it on rebuild.
/// We copy into the app config dir and run that path instead.
fn tinymist_executable(state: &AppState) -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("TINYMIST_PATH") {
        return Ok(PathBuf::from(p));
    }
    if let Some(p) = state.bundled_tinymist.as_ref() {
        if p.is_file() {
            let cache_dir = state.app_config_dir.join("tinymist-cache");
            return materialized_bundled_tinymist(p, &cache_dir).map_err(|e| e.to_string());
        }
    }
    Ok(PathBuf::from("tinymist"))
}

fn bundled_cache_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "tinymist.exe"
    } else {
        "tinymist"
    }
}

fn needs_tinymist_cache_refresh(bundled: &Path, cached: &Path) -> io::Result<bool> {
    let b = fs::metadata(bundled)?;
    let c = match fs::metadata(cached) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(e) => return Err(e),
    };
    if b.len() != c.len() {
        return Ok(true);
    }
    match (b.modified(), c.modified()) {
        (Ok(bt), Ok(ct)) => Ok(bt > ct),
        _ => Ok(true),
    }
}

fn replace_file_atomically(tmp: &Path, dest: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        fs::rename(tmp, dest)?;
    }
    #[cfg(not(unix))]
    {
        let _ = fs::remove_file(dest);
        fs::rename(tmp, dest)?;
    }
    Ok(())
}

fn materialized_bundled_tinymist(bundled: &Path, cache_dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(cache_dir)?;
    let dest = cache_dir.join(bundled_cache_name());
    if needs_tinymist_cache_refresh(bundled, &dest)? {
        let tmp = cache_dir.join(if cfg!(target_os = "windows") {
            ".tinymist.exe.part"
        } else {
            ".tinymist.part"
        });
        let _ = fs::remove_file(&tmp);
        fs::copy(bundled, &tmp)?;
        replace_file_atomically(&tmp, &dest)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dest, perms)?;
        }
    }
    Ok(dest)
}

fn parse_data_plane_url(line: &str) -> Option<String> {
    let idx = line.find(DATA_PLANE_PREFIX)?;
    let host = line[idx + DATA_PLANE_PREFIX.len()..].trim();
    if host.is_empty() {
        return None;
    }
    Some(format!("http://{host}/"))
}

fn parse_control_plane_ws_url(line: &str) -> Option<String> {
    let idx = line.find(CONTROL_PANEL_PREFIX)?;
    let host = line[idx + CONTROL_PANEL_PREFIX.len()..].trim();
    if host.is_empty() {
        return None;
    }
    Some(format!("ws://{host}/"))
}

#[derive(Clone, serde::Serialize)]
struct PreviewScrollToSource {
    filepath: String,
    line0: u32,
    column0: u32,
}

fn parse_editor_scroll_payload(txt: &str) -> Option<PreviewScrollToSource> {
    let v: serde_json::Value = serde_json::from_str(txt).ok()?;
    if v.get("event").and_then(|x| x.as_str())? != "editorScrollTo" {
        return None;
    }
    let filepath = v.get("filepath").and_then(|x| x.as_str())?.to_string();
    let (line0, column0) = match v.get("start") {
        Some(serde_json::Value::Array(a)) if a.len() >= 2 => {
            let l = a.first()?.as_u64()? as u32;
            let c = a.get(1)?.as_u64()? as u32;
            (l, c)
        }
        _ => (0, 0),
    };
    Some(PreviewScrollToSource {
        filepath,
        line0,
        column0,
    })
}

fn spawn_control_plane_listener(app: tauri::AppHandle, ws_url: String) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let Ok((mut socket, _)) = tungstenite::connect(ws_url.as_str()) else {
            return;
        };
        loop {
            match socket.read() {
                Ok(Message::Text(t)) => {
                    if let Some(p) = parse_editor_scroll_payload(&t) {
                        let _ = app.emit("preview-scroll-to-source", p);
                    }
                }
                Ok(Message::Ping(p)) => {
                    let _ = socket.send(Message::Pong(p));
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    })
}

/// Kill any running preview process and clear state.
///
/// Kill the tinymist child **before** joining the control-plane listener thread. Joining first
/// could deadlock: the listener blocks on `socket.read()` until the server closes the connection.
pub fn stop(state: &AppState) -> Result<(), String> {
    let mut slot = state.tinymist.lock().map_err(|e| e.to_string())?;
    if let Some(mut session) = slot.take() {
        let listener = session.control_plane_listener.take();
        let _ = session.child.kill();
        let _ = session.child.wait();
        if let Some(h) = listener {
            let _ = h.join();
        }
    }
    Ok(())
}

/// Drains tinymist stderr: notify the main thread as soon as the HTTP preview URL is known
/// (do not wait for the control-plane WebSocket line — it may arrive later or be absent).
fn spawn_stderr_reader(
    stderr: std::process::ChildStderr,
    tx: mpsc::Sender<Result<(String, Option<String>), String>>,
    tx_late_control_ws: mpsc::Sender<String>,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        let mut preview_url: Option<String> = None;
        let mut control_ws: Option<String> = None;
        let mut preview_reported = false;
        // First `tx` had `control_ws: None`; if the control line appears later, forward on `tx_late`.
        let mut waiting_for_late_control = false;
        let mut late_control_sent = false;

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let t = line.trim_end();
                    if preview_url.is_none() {
                        preview_url = parse_data_plane_url(t);
                    }
                    if control_ws.is_none() {
                        control_ws = parse_control_plane_ws_url(t);
                    }
                    if preview_url.is_some() && !preview_reported {
                        let cw = control_ws.clone();
                        waiting_for_late_control = cw.is_none();
                        let _ = tx.send(Ok((preview_url.clone().unwrap(), cw)));
                        preview_reported = true;
                    }
                    if waiting_for_late_control
                        && !late_control_sent
                        && control_ws.as_ref().is_some_and(|s| !s.is_empty())
                    {
                        if let Some(ws) = control_ws.clone() {
                            let _ = tx_late_control_ws.send(ws);
                            late_control_sent = true;
                            waiting_for_late_control = false;
                        }
                    }
                }
                Err(_) => break,
            }
        }

        if !preview_reported {
            match preview_url {
                Some(p) => {
                    let _ = tx.send(Ok((p, control_ws)));
                }
                None => {
                    let _ = tx.send(Err(
                        "tinymist ended before the preview server started (is tinymist installed?)"
                            .into(),
                    ));
                }
            }
        }

        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
        }
    });
}

pub struct TinymistSession {
    pub child: Child,
    pub preview_url: String,
    pub root: PathBuf,
    pub entry_rel: String,
    control_plane_listener: Option<thread::JoinHandle<()>>,
}

/// Start tinymist preview for the open project, or return the existing session URL if unchanged.
pub fn ensure_running(app: &tauri::AppHandle, state: &AppState) -> Result<String, String> {
    let root = state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())?;

    let input_path = join_under_root(&root, MAIN_TYP)?;
    if !input_path.is_file() {
        return Err(format!("entry file not found: {MAIN_TYP}"));
    }

    {
        let slot = state.tinymist.lock().map_err(|e| e.to_string())?;
        if let Some(session) = slot.as_ref() {
            if session.root == root && session.entry_rel == MAIN_TYP {
                return Ok(session.preview_url.clone());
            }
        }
    }

    stop(state)?;

    let root_s = root.to_str().ok_or("project path is not valid UTF-8")?;
    let input_s = input_path
        .to_str()
        .ok_or("entry path is not valid UTF-8")?;
    let cache_s = state
        .typst_package_cache
        .to_str()
        .ok_or("package cache path is not valid UTF-8")?;

    let mut cmd = Command::new(tinymist_executable(state)?);
    cmd.current_dir(&root)
        .arg("preview")
        .arg(input_s)
        .arg("--no-open")
        // Incremental patches can flicker and throw off in-preview scroll; full passes are steadier.
        .arg("--partial-rendering")
        .arg("false")
        .arg("--data-plane-host")
        .arg("127.0.0.1:0")
        .arg("--control-plane-host")
        .arg("127.0.0.1:0")
        .arg("--root")
        .arg(root_s)
        .arg("--package-cache-path")
        .arg(cache_s)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        format!(
            "failed to start tinymist ({e}); run a full `cargo build` to fetch the bundled binary, install tinymist, or set TINYMIST_PATH"
        )
    })?;

    let stderr = child.stderr.take().ok_or("tinymist: no stderr pipe")?;
    let (tx, rx) = mpsc::channel();
    let (tx_late_ws, rx_late_ws) = mpsc::channel::<String>();
    spawn_stderr_reader(stderr, tx, tx_late_ws);

    let (preview_url, control_ws) = match rx.recv_timeout(Duration::from_secs(45)) {
        Ok(Ok(pair)) => pair,
        Ok(Err(msg)) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(msg);
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err("timed out waiting for tinymist preview (45s)".into());
        }
    };

    let app_handle = app.clone();
    let mut control_plane_listener = control_ws
        .filter(|s| !s.is_empty())
        .map(|ws| spawn_control_plane_listener(app_handle.clone(), ws));

    if control_plane_listener.is_none() {
        if let Ok(ws) = rx_late_ws.recv_timeout(Duration::from_secs(10)) {
            if !ws.is_empty() {
                control_plane_listener = Some(spawn_control_plane_listener(app_handle, ws));
            }
        }
    }

    *state.tinymist.lock().map_err(|e| e.to_string())? = Some(TinymistSession {
        child,
        preview_url: preview_url.clone(),
        root,
        entry_rel: MAIN_TYP.into(),
        control_plane_listener,
    });

    Ok(preview_url)
}

#[tauri::command]
pub fn start_tinymist_preview(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_running(&app, &state)
}

#[tauri::command]
pub fn restart_tinymist_preview(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    stop(&state)?;
    ensure_running(&app, &state)
}
