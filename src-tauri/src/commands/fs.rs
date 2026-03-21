use std::fs;
use std::path::Path;

use crate::project::paths::join_under_root;
use crate::AppState;

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

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        if meta.is_dir() {
            // Skip hidden / common noise
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(true)
            {
                continue;
            }
            collect_files(root, &path, out)?;
        } else if meta.is_file() && is_listed_file(&path) {
            let rel = path.strip_prefix(root).map_err(|_| "prefix".to_string())?;
            let s = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.push(s);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn list_project_files(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let guard = state.project_root.lock().map_err(|e| e.to_string())?;
    let root = guard
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files)?;
    files.sort();
    Ok(files)
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
    fs::write(&path, content).map_err(|e| e.to_string())
}
