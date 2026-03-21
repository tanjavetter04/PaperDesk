use std::path::PathBuf;

use crate::project::history::{
    self, diff_commit_to_workdir, get_entry, history_active, list_commits, open_repo,
    respond_enable, respond_existing_git, restore_paths, should_prompt_enable,
    should_prompt_existing_git, try_checkpoint, HISTORY_REF,
};
use crate::AppState;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryStatus {
    pub has_git_dir: bool,
    pub enabled: bool,
    pub prompt_enable: bool,
    pub prompt_existing_git: bool,
    pub use_existing_git: Option<bool>,
    pub history_ref_exists: bool,
    pub tip_short: Option<String>,
}

fn require_root(state: &AppState) -> Result<PathBuf, String> {
    state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())
}

#[tauri::command]
pub fn history_get_status(state: tauri::State<'_, AppState>) -> Result<HistoryStatus, String> {
    let root = require_root(&state)?;
    let entry = get_entry(&state, &root)?;
    let has_git = history::has_git_dir(&root);
    let (history_ref_exists, tip_short) = if has_git {
        match open_repo(&root) {
            Ok(repo) => match repo.find_reference(HISTORY_REF) {
                Ok(r) => {
                    let short = r
                        .target()
                        .map(|oid| oid.to_string().chars().take(8).collect::<String>());
                    (true, short)
                }
                Err(_) => (false, None),
            },
            Err(_) => (false, None),
        }
    } else {
        (false, None)
    };

    Ok(HistoryStatus {
        has_git_dir: has_git,
        enabled: entry.enabled,
        prompt_enable: should_prompt_enable(&entry),
        prompt_existing_git: should_prompt_existing_git(&root, &entry),
        use_existing_git: entry.use_existing_git,
        history_ref_exists,
        tip_short,
    })
}

#[tauri::command]
pub fn history_respond_enable(
    state: tauri::State<'_, AppState>,
    enable: bool,
) -> Result<(), String> {
    let root = require_root(&state)?;
    respond_enable(&state, root, enable)
}

#[tauri::command]
pub fn history_respond_existing_git(
    state: tauri::State<'_, AppState>,
    use_existing: bool,
) -> Result<(), String> {
    let root = require_root(&state)?;
    respond_existing_git(&state, root, use_existing)
}

#[tauri::command]
pub fn history_checkpoint(
    state: tauri::State<'_, AppState>,
    reason: String,
    force: bool,
) -> Result<Option<String>, String> {
    let root = require_root(&state)?;
    let msg = format!("paperdesk: {}", reason.trim());
    try_checkpoint(&state, &root, &msg, force)
}

#[tauri::command]
pub fn history_list_commits(
    state: tauri::State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<history::HistoryCommitSummary>, String> {
    let root = require_root(&state)?;
    let entry = get_entry(&state, &root)?;
    if !history_active(&root, &entry) {
        return Err("project history is not active".into());
    }
    let repo = open_repo(&root)?;
    list_commits(&repo, limit.unwrap_or(50))
}

#[tauri::command]
pub fn history_diff_workdir(
    state: tauri::State<'_, AppState>,
    commit_id: String,
) -> Result<String, String> {
    let root = require_root(&state)?;
    let entry = get_entry(&state, &root)?;
    if !history_active(&root, &entry) {
        return Err("project history is not active".into());
    }
    let repo = open_repo(&root)?;
    diff_commit_to_workdir(&repo, &commit_id)
}

#[tauri::command]
pub fn history_restore(
    state: tauri::State<'_, AppState>,
    commit_id: String,
    paths: Option<Vec<String>>,
) -> Result<(), String> {
    let root = require_root(&state)?;
    let entry = get_entry(&state, &root)?;
    if !history_active(&root, &entry) {
        return Err("project history is not active".into());
    }
    let _lock = state.history_git_lock.lock().map_err(|e| e.to_string())?;
    let repo = open_repo(&root)?;
    let paths = paths.unwrap_or_default();
    restore_paths(&repo, &commit_id, &paths)?;
    Ok(())
}
