use std::collections::HashSet;
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

fn dedupe_recent_paths_preserve_order(paths: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for p in paths {
        if seen.insert(p.clone()) {
            out.push(p);
        }
    }
    out
}

fn validate_project_folder_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("name is empty".into());
    }
    if name == "." || name == ".." {
        return Err("invalid name".into());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("name must not contain path separators".into());
    }
    #[cfg(windows)]
    {
        const FORBIDDEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
        if name.chars().any(|c| FORBIDDEN.contains(&c)) {
            return Err("invalid character in name".into());
        }
    }
    if name.chars().any(|c| c.is_control()) {
        return Err("invalid character in name".into());
    }
    Ok(name)
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
    let main_path = p.join(MAIN_TYP);
    if !main_path.is_file() {
        return Err(format!(
            "not a PaperDesk project folder: missing {MAIN_TYP} in the project root (expected {})",
            main_path.display()
        ));
    }
    *state.project_root.lock().map_err(|e| e.to_string())? = Some(p.clone());
    let mut paths = load_recent(&state);
    let s = p.to_string_lossy().to_string();
    paths.retain(|x| x != &s);
    paths.insert(0, s);
    paths.truncate(RECENT_MAX);
    save_recent(&state, paths)
}

/// Rename the project directory (last path segment). Updates recent list and open `project_root` when it matches.
#[tauri::command]
pub fn rename_project(
    state: tauri::State<'_, AppState>,
    path: String,
    new_name: String,
) -> Result<String, String> {
    let trimmed = validate_project_folder_name(&new_name)?;
    let old = PathBuf::from(path.trim());
    let old_canon = old
        .canonicalize()
        .map_err(|e| format!("invalid project path: {e}"))?;
    if !old_canon.is_dir() {
        return Err("project path must be a directory".into());
    }
    if !old_canon.join(MAIN_TYP).is_file() {
        return Err(format!(
            "cannot rename: missing {MAIN_TYP} in the project root"
        ));
    }

    let parent = old_canon
        .parent()
        .ok_or_else(|| "cannot rename filesystem root".to_string())?;
    let dest = parent.join(trimmed);
    if old_canon.file_name().and_then(|n| n.to_str()) == Some(trimmed) {
        return Ok(old_canon.to_string_lossy().into_owned());
    }
    if dest.exists() {
        return Err("a file or folder with that name already exists".into());
    }

    let open_matches = {
        let g = state.project_root.lock().map_err(|e| e.to_string())?;
        match g.as_ref() {
            Some(r) => r
                .canonicalize()
                .ok()
                .as_ref()
                .is_some_and(|c| c == &old_canon),
            None => false,
        }
    };

    fs::rename(&old_canon, &dest).map_err(|e| e.to_string())?;
    let new_canon = dest
        .canonicalize()
        .map_err(|e| format!("renamed project but could not resolve path: {e}"))?;

    let old_s = old_canon.to_string_lossy().to_string();
    let new_s = new_canon.to_string_lossy().to_string();
    let mut paths = load_recent(&state);
    for p in &mut paths {
        let same = p == &old_s
            || PathBuf::from(p.as_str())
                .canonicalize()
                .ok()
                .as_ref()
                .is_some_and(|c| c == &old_canon);
        if same {
            *p = new_s.clone();
        }
    }
    paths = dedupe_recent_paths_preserve_order(paths);
    save_recent(&state, paths)?;

    crate::project::history::migrate_path_after_rename(&state, &old_canon, &new_canon)?;

    if open_matches {
        *state.project_root.lock().map_err(|e| e.to_string())? = Some(new_canon.clone());
        tinymist_preview::stop(&state)?;
    }

    Ok(new_s)
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
        let _ = crate::project::history::try_checkpoint(&state, r, "close", false);
    }
    bib_watch::stop(&state);
    tinymist_preview::stop(&state)?;
    *state.project_root.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

/// Creates `parent_dir/project_name` as a new empty directory (must not exist yet).
fn prepare_new_project_in_parent(parent_dir: &str, project_name: &str) -> Result<PathBuf, String> {
    let name = validate_project_folder_name(project_name)?;
    let parent = PathBuf::from(parent_dir.trim());
    let parent = parent
        .canonicalize()
        .map_err(|e| format!("invalid parent folder: {e}"))?;
    if !parent.is_dir() {
        return Err("parent folder must be a directory".into());
    }
    let target = parent.join(name);
    if target.exists() {
        return Err("a project folder with that name already exists in the chosen location".into());
    }
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    target
        .canonicalize()
        .map_err(|e| format!("could not canonicalize new project folder: {e}"))
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

/// `template_id` must be `thesis`. Creates `parent_dir/project_name` and copies the template into it.
#[tauri::command]
pub fn create_from_template(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    template_id: String,
    parent_dir: String,
    project_name: String,
) -> Result<String, String> {
    if template_id.trim() != "thesis" {
        return Err("unknown template (only thesis is available)".into());
    }
    let target = prepare_new_project_in_parent(&parent_dir, &project_name)?;

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

/// Empty project: creates `parent_dir/project_name` with an empty root `main.typ`.
#[tauri::command]
pub fn create_empty_project(
    state: tauri::State<'_, AppState>,
    parent_dir: String,
    project_name: String,
) -> Result<String, String> {
    let target = prepare_new_project_in_parent(&parent_dir, &project_name)?;
    let main_path = target.join(MAIN_TYP);
    fs::write(&main_path, []).map_err(|e| format!("failed to write {MAIN_TYP}: {e}"))?;
    activate_new_project(&state, target)
}

