use std::fs;
use std::path::Path;

use crate::project::history;
use crate::project::paths::{join_under_root, MAIN_TYP};
use crate::AppState;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntry {
    pub path: String,
    pub is_dir: bool,
}

const TEXT_EXTENSIONS: &[&str] = &["typ", "bib", "txt", "md", "yml", "yaml", "json"];

fn is_listed_file(path: &Path) -> bool {
    path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            TEXT_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(ext))
                || matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "pdf"
                )
        })
        .unwrap_or(false)
}

fn rel_path_str(root: &Path, path: &Path) -> Result<String, String> {
    let rel = path.strip_prefix(root).map_err(|_| "prefix".to_string())?;
    Ok(rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn collect_entries(root: &Path, dir: &Path, out: &mut Vec<ProjectEntry>) -> Result<(), String> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(true)
        {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        if meta.is_dir() {
            let rel = rel_path_str(root, &path)?;
            out.push(ProjectEntry {
                path: rel,
                is_dir: true,
            });
            collect_entries(root, &path, out)?;
        } else if meta.is_file() && is_listed_file(&path) {
            let rel = rel_path_str(root, &path)?;
            out.push(ProjectEntry {
                path: rel,
                is_dir: false,
            });
        }
    }
    Ok(())
}

#[tauri::command]
pub fn list_project_files(state: tauri::State<'_, AppState>) -> Result<Vec<ProjectEntry>, String> {
    let guard = state.project_root.lock().map_err(|e| e.to_string())?;
    let root = guard
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let mut entries = Vec::new();
    collect_entries(&root, &root, &mut entries)?;
    Ok(entries)
}

/// Create a directory under the project root (`relative_path` uses `/`). Fails if it already exists.
#[tauri::command]
pub fn create_project_dir(
    state: tauri::State<'_, AppState>,
    relative_path: String,
) -> Result<(), String> {
    let relative_path = relative_path.trim().replace('\\', "/");
    if relative_path.is_empty() {
        return Err("path is empty".into());
    }
    let guard = state.project_root.lock().map_err(|e| e.to_string())?;
    let root = guard
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let path = join_under_root(&root, &relative_path)?;
    if path.exists() {
        return Err("a file or folder with that path already exists".into());
    }
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    drop(guard);
    let _ = history::try_checkpoint(&state, &root, "paperdesk: new folder", true);
    Ok(())
}

/// Move or rename a file or folder under the project root. Parent directories of `to` are created.
#[tauri::command]
pub fn move_project_path(
    state: tauri::State<'_, AppState>,
    from: String,
    to: String,
) -> Result<(), String> {
    let from_rel = from.trim().replace('\\', "/");
    let to_rel = to.trim().replace('\\', "/");
    if from_rel.is_empty() || to_rel.is_empty() {
        return Err("path is empty".into());
    }
    if from_rel == MAIN_TYP {
        return Err("cannot move main.typ (project entry file)".into());
    }
    let guard = state.project_root.lock().map_err(|e| e.to_string())?;
    let root = guard
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let from_path = join_under_root(&root, &from_rel)?;
    let to_path = join_under_root(&root, &to_rel)?;
    if !from_path.exists() {
        return Err("source does not exist".into());
    }
    if to_path.exists() {
        return Err("destination already exists".into());
    }
    if let Some(parent) = to_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::rename(&from_path, &to_path).map_err(|e| e.to_string())?;
    drop(guard);
    let _ = history::try_checkpoint(&state, &root, "paperdesk: move/rename", true);
    Ok(())
}

#[tauri::command]
pub fn read_text_file(state: tauri::State<'_, AppState>, relative_path: String) -> Result<String, String> {
    let guard = state.project_root.lock().map_err(|e| e.to_string())?;
    let root = guard
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let path = join_under_root(&root, &relative_path)?;
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_text_file(
    state: tauri::State<'_, AppState>,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    let guard = state.project_root.lock().map_err(|e| e.to_string())?;
    let root = guard
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let path = join_under_root(&root, &relative_path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| e.to_string())?;
    history::note_dirty(&state);
    Ok(())
}
