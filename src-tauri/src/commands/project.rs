use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::AppState;

const RECENT_MAX: usize = 12;
const RECENT_FILE: &str = "recent_projects.json";

#[derive(Serialize, Deserialize, Default)]
struct RecentFile {
    #[serde(default)]
    paths: Vec<String>,
}

fn recent_path(state: &AppState) -> PathBuf {
    state.app_config_dir.join(RECENT_FILE)
}

fn load_recent(state: &AppState) -> Vec<String> {
    let p = recent_path(state);
    let Ok(raw) = fs::read_to_string(&p) else {
        return Vec::new();
    };
    serde_json::from_str::<RecentFile>(&raw)
        .map(|r| r.paths)
        .unwrap_or_default()
}

fn save_recent(state: &AppState, paths: Vec<String>) -> Result<(), String> {
    fs::create_dir_all(&state.app_config_dir).map_err(|e| e.to_string())?;
    let data = RecentFile { paths };
    fs::write(
        recent_path(state),
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recent_projects(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(load_recent(&state))
}

#[tauri::command]
pub fn add_recent_project(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let mut paths = load_recent(&state);
    paths.retain(|p| p != &path);
    paths.insert(0, path);
    paths.truncate(RECENT_MAX);
    save_recent(&state, paths)
}

#[tauri::command]
pub fn open_project(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let p = PathBuf::from(path.trim());
    let p = p
        .canonicalize()
        .map_err(|e| format!("invalid project path: {e}"))?;
    if !p.is_dir() {
        return Err("project path must be a directory".into());
    }
    *state.project_root.lock().map_err(|e| e.to_string())? = Some(p.clone());
    let mut paths = load_recent(&state);
    let s = p.to_string_lossy().to_string();
    paths.retain(|x| x != &s);
    paths.insert(0, s);
    paths.truncate(RECENT_MAX);
    save_recent(&state, paths)
}

#[tauri::command]
pub fn get_open_project(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn close_project(state: tauri::State<'_, AppState>) -> Result<(), String> {
    *state.project_root.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// `template_id`: `article` or `thesis`. `target_dir` is the new project folder (created if missing; must be empty if exists).
#[tauri::command]
pub fn create_from_template(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    template_id: String,
    target_dir: String,
) -> Result<String, String> {
    let target = PathBuf::from(target_dir.trim());
    if target.exists() {
        let mut it = fs::read_dir(&target).map_err(|e| e.to_string())?;
        if it.next().is_some() {
            return Err("target folder must be empty".into());
        }
    } else {
        fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    }
    let target = target
        .canonicalize()
        .map_err(|e| format!("could not canonicalize target: {e}"))?;

    let rel = format!("templates/{template_id}");
    let src = app
        .path()
        .resolve(&rel, tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("could not resolve bundled template ({rel}): {e}"))?;
    if !src.is_dir() {
        return Err(format!("template not found: {template_id}"));
    }

    copy_dir_all(&src, &target).map_err(|e| format!("failed to copy template: {e}"))?;

    *state.project_root.lock().map_err(|e| e.to_string())? = Some(target.clone());
    let s = target.to_string_lossy().to_string();
    let mut paths = load_recent(&state);
    paths.retain(|p| p != &s);
    paths.insert(0, s.clone());
    paths.truncate(RECENT_MAX);
    save_recent(&state, paths)?;

    Ok(s)
}

