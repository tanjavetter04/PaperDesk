use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::project::paths::join_under_root;
use crate::AppState;

pub struct BibWatchSession {
    _watcher: RecommendedWatcher,
}

fn normalize_bib_relative(raw: &str) -> Result<String, String> {
    let s = raw.trim().replace('\\', "/");
    if s.is_empty() {
        return Err("bibliography path is empty".into());
    }
    if s.starts_with('/') || s.contains("..") {
        return Err("invalid bibliography path".into());
    }
    Ok(s)
}

fn paths_match(event_path: &Path, bib: &Path) -> bool {
    if event_path == bib {
        return true;
    }
    match (event_path.canonicalize(), bib.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => event_path.file_name() == bib.file_name()
            && match (event_path.parent(), bib.parent()) {
                (Some(ep), Some(bp)) => ep == bp,
                _ => false,
            },
    }
}

/// Stop watching; safe to call when no watcher is active.
pub fn stop(state: &AppState) {
    if let Ok(mut slot) = state.bib_watch.lock() {
        *slot = None;
    }
}

/// Watch the parent directory of `relative_bib` (non-recursive) and emit `bib-externally-updated`
/// when that file is created, modified, or removed. Debounced ~300 ms.
pub fn start(app: &AppHandle, state: &AppState, relative_bib: &str) -> Result<(), String> {
    stop(state);

    let rel = normalize_bib_relative(relative_bib)?;
    let root = state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())?;

    let bib_path = join_under_root(&root, &rel)?;
    let watch_dir = bib_path
        .parent()
        .filter(|p| p.as_os_str().len() > 0)
        .unwrap_or(&root)
        .to_path_buf();

    if !watch_dir.is_dir() {
        return Err("bibliography parent folder is missing".into());
    }

    let bib_target = bib_path;
    let rel_emit = rel.clone();
    let app_handle = app.clone();
    let gen = Arc::new(AtomicU64::new(0));

    let gen_cb = Arc::clone(&gen);
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            let Ok(event) = res else {
                return;
            };
            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                return;
            }
            for p in event.paths {
                if paths_match(&p, &bib_target) {
                    let g = gen_cb.fetch_add(1, Ordering::SeqCst) + 1;
                    let app2 = app_handle.clone();
                    let rel2 = rel_emit.clone();
                    let gen2 = Arc::clone(&gen_cb);
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(300));
                        if gen2.load(Ordering::SeqCst) == g {
                            let payload = serde_json::json!({ "relativePath": rel2 });
                            let _ = app2.emit("bib-externally-updated", payload);
                        }
                    });
                    break;
                }
            }
        },
        Config::default(),
    )
    .map_err(|e| format!("bib watcher: {e}"))?;

    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("bib watcher watch: {e}"))?;

    *state.bib_watch.lock().map_err(|e| e.to_string())? = Some(BibWatchSession { _watcher: watcher });

    Ok(())
}

#[tauri::command]
pub fn restart_bib_watcher(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    relative_path: String,
) -> Result<(), String> {
    start(&app, &state, &relative_path)
}
