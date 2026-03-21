use std::io::{BufRead, BufReader};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::project::paths::join_under_root;
use crate::AppState;

#[derive(serde::Deserialize)]
struct PaperDeskMeta {
    entry: Option<String>,
}

const DATA_PLANE_PREFIX: &str = "Data plane server listening on: ";

fn resolve_entry(root: &std::path::Path, entry: Option<String>) -> String {
    if let Some(entry) = entry {
        let trimmed = entry.trim();
        if !trimmed.is_empty() {
            return trimmed.replace('\\', "/");
        }
    }

    let meta_path = root.join("paperdesk.json");
    if let Ok(raw) = fs::read_to_string(&meta_path) {
        if let Ok(meta) = serde_json::from_str::<PaperDeskMeta>(&raw) {
            if let Some(entry) = meta.entry.filter(|value| !value.is_empty()) {
                return entry.replace('\\', "/");
            }
        }
    }

    "main.typ".into()
}

/// Prefer explicit `TINYMIST_PATH`, then the binary shipped in `resources/bin/`, then `PATH`.
fn tinymist_executable(state: &AppState) -> PathBuf {
    if let Ok(p) = std::env::var("TINYMIST_PATH") {
        return PathBuf::from(p);
    }
    if let Some(p) = state.bundled_tinymist.as_ref() {
        if p.is_file() {
            return p.clone();
        }
    }
    PathBuf::from("tinymist")
}

fn parse_data_plane_url(line: &str) -> Option<String> {
    let idx = line.find(DATA_PLANE_PREFIX)?;
    let host = line[idx + DATA_PLANE_PREFIX.len()..].trim();
    if host.is_empty() {
        return None;
    }
    Some(format!("http://{host}/"))
}

/// Kill any running preview process and clear state.
pub fn stop(state: &AppState) -> Result<(), String> {
    let mut slot = state.tinymist.lock().map_err(|e| e.to_string())?;
    if let Some(mut session) = slot.take() {
        let _ = session.child.kill();
        let _ = session.child.wait();
    }
    Ok(())
}

fn spawn_reader_thread(
    stderr: std::process::ChildStderr,
    tx: mpsc::Sender<Result<String, String>>,
) {
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut sent = false;
        for line in reader.lines().map_while(Result::ok) {
            if !sent {
                if let Some(url) = parse_data_plane_url(&line) {
                    sent = true;
                    let _ = tx.send(Ok(url));
                }
            }
        }
        if !sent {
            let _ = tx.send(Err(
                "tinymist ended before the preview server started (is tinymist installed?)".into(),
            ));
        }
    });
}

pub struct TinymistSession {
    pub child: Child,
    pub preview_url: String,
    pub root: PathBuf,
    pub entry_rel: String,
}

/// Start tinymist preview for the open project, or return the existing session URL if unchanged.
pub fn ensure_running(state: &AppState, entry: Option<String>) -> Result<String, String> {
    let root = state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())?;

    let rel = resolve_entry(&root, entry);
    let input_path = join_under_root(&root, &rel)?;
    if !input_path.is_file() {
        return Err(format!("entry file not found: {rel}"));
    }

    {
        let slot = state.tinymist.lock().map_err(|e| e.to_string())?;
        if let Some(session) = slot.as_ref() {
            if session.root == root && session.entry_rel == rel {
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

    let mut cmd = Command::new(tinymist_executable(state));
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
    spawn_reader_thread(stderr, tx);

    let preview_url = match rx.recv_timeout(Duration::from_secs(45)) {
        Ok(Ok(url)) => url,
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

    *state.tinymist.lock().map_err(|e| e.to_string())? = Some(TinymistSession {
        child,
        preview_url: preview_url.clone(),
        root,
        entry_rel: rel,
    });

    Ok(preview_url)
}

#[tauri::command]
pub fn start_tinymist_preview(
    state: tauri::State<'_, AppState>,
    entry: Option<String>,
) -> Result<String, String> {
    ensure_running(&state, entry)
}

#[tauri::command]
pub fn restart_tinymist_preview(
    state: tauri::State<'_, AppState>,
    entry: Option<String>,
) -> Result<String, String> {
    stop(&state)?;
    ensure_running(&state, entry)
}
