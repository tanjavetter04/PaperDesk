use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::bib_watch;
use crate::project::paths::MAIN_TYP;
use crate::tinymist_preview;
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
    let root = state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    if let Some(ref r) = root {
        let _ = crate::project::history::try_checkpoint(&state, r, "paperdesk: close", false);
    }
    bib_watch::stop(&state);
    tinymist_preview::stop(&state)?;
    *state.project_root.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

fn prepare_new_project_dir(target_dir: &str) -> Result<PathBuf, String> {
    let target = PathBuf::from(target_dir.trim());
    if target.exists() {
        let mut it = fs::read_dir(&target).map_err(|e| e.to_string())?;
        if it.next().is_some() {
            return Err("target folder must be empty".into());
        }
    } else {
        fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    }
    target
        .canonicalize()
        .map_err(|e| format!("could not canonicalize target: {e}"))
}

fn activate_new_project(state: &AppState, target: PathBuf) -> Result<String, String> {
    *state.project_root.lock().map_err(|e| e.to_string())? = Some(target.clone());
    let s = target.to_string_lossy().to_string();
    let mut paths = load_recent(&state);
    paths.retain(|p| p != &s);
    paths.insert(0, s.clone());
    paths.truncate(RECENT_MAX);
    save_recent(&state, paths)?;
    Ok(s)
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

/// `template_id` must be `thesis`. `target_dir` is the new project folder (created if missing; must be empty if exists).
#[tauri::command]
pub fn create_from_template(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    template_id: String,
    target_dir: String,
) -> Result<String, String> {
    if template_id.trim() != "thesis" {
        return Err("unknown template (only thesis is available)".into());
    }
    let target = prepare_new_project_dir(&target_dir)?;

    let rel = format!("templates/{template_id}");
    let src = app
        .path()
        .resolve(&rel, tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("could not resolve bundled template ({rel}): {e}"))?;
    if !src.is_dir() {
        return Err(format!("template not found: {template_id}"));
    }

    copy_dir_all(&src, &target).map_err(|e| format!("failed to copy template: {e}"))?;

    activate_new_project(&state, target)
}

/// Empty project: only `main.typ` (empty file). `target_dir` rules match [`create_from_template`].
#[tauri::command]
pub fn create_empty_project(
    state: tauri::State<'_, AppState>,
    target_dir: String,
) -> Result<String, String> {
    let target = prepare_new_project_dir(&target_dir)?;
    let main_path = target.join(MAIN_TYP);
    fs::write(&main_path, []).map_err(|e| format!("failed to write {MAIN_TYP}: {e}"))?;
    activate_new_project(&state, target)
}

