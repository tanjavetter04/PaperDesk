//! PaperDesk-managed Git checkpoints under `refs/paperdesk/history` (does not move `HEAD`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use git2::{Commit, IndexAddOption, Oid, Repository, Signature};

use crate::AppState;

pub const HISTORY_REF: &str = "refs/paperdesk/history";
const SETTINGS_FILE: &str = "project_history.json";
const STORE_VERSION: u32 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHistoryEntry {
    #[serde(default)]
    pub asked_enable: bool,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub asked_existing_git: bool,
    #[serde(default)]
    pub use_existing_git: Option<bool>,
}

impl Default for ProjectHistoryEntry {
    fn default() -> Self {
        Self {
            asked_enable: false,
            enabled: false,
            asked_existing_git: false,
            use_existing_git: None,
        }
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct HistoryStore {
    #[serde(default = "store_version_default")]
    version: u32,
    #[serde(default)]
    projects: HashMap<String, ProjectHistoryEntry>,
}

fn store_version_default() -> u32 {
    STORE_VERSION
}

fn settings_path(state: &AppState) -> PathBuf {
    state.app_config_dir.join(SETTINGS_FILE)
}

fn load_store(state: &AppState) -> Result<HistoryStore, String> {
    let p = settings_path(state);
    let Ok(raw) = fs::read_to_string(&p) else {
        return Ok(HistoryStore::default());
    };
    serde_json::from_str(&raw).map_err(|e| format!("project history settings: {e}"))
}

fn save_store(state: &AppState, store: &HistoryStore) -> Result<(), String> {
    fs::create_dir_all(&state.app_config_dir).map_err(|e| e.to_string())?;
    let data =
        serde_json::to_string_pretty(store).map_err(|e| format!("serialize history store: {e}"))?;
    fs::write(settings_path(state), data).map_err(|e| e.to_string())
}

fn project_key(root: &Path) -> String {
    root.to_string_lossy().to_string()
}

pub fn get_entry(state: &AppState, root: &Path) -> Result<ProjectHistoryEntry, String> {
    let store = load_store(state)?;
    Ok(store
        .projects
        .get(&project_key(root))
        .cloned()
        .unwrap_or_default())
}

/// Move `project_history.json` entry from the old project root path to the new one (after `fs::rename` of the project folder).
pub fn migrate_path_after_rename(
    state: &AppState,
    old_canonical_root: &Path,
    new_canonical_root: &Path,
) -> Result<(), String> {
    let old_key = project_key(old_canonical_root);
    let new_key = project_key(new_canonical_root);
    if old_key == new_key {
        return Ok(());
    }
    let mut store = load_store(state)?;
    if let Some(entry) = store.projects.remove(&old_key) {
        store.projects.insert(new_key, entry);
        save_store(state, &store)?;
    }
    Ok(())
}

/// Drop PaperDesk history preferences for a project root (e.g. after the folder was deleted).
pub fn remove_store_entry_for_root(state: &AppState, root: &Path) -> Result<(), String> {
    let key = project_key(root);
    let mut store = load_store(state)?;
    if store.projects.remove(&key).is_some() {
        save_store(state, &store)?;
    }
    Ok(())
}

fn update_entry<F>(state: &AppState, root: &Path, f: F) -> Result<ProjectHistoryEntry, String>
where
    F: FnOnce(&mut ProjectHistoryEntry),
{
    let mut store = load_store(state)?;
    let key = project_key(root);
    let e = store.projects.entry(key).or_default();
    f(e);
    let out = e.clone();
    save_store(state, &store)?;
    Ok(out)
}

pub fn has_git_dir(root: &Path) -> bool {
    root.join(".git").exists()
}

pub fn note_dirty(state: &AppState) {
    state.history_dirty.store(true, Ordering::Release);
}

pub fn take_dirty(state: &AppState) -> bool {
    state.history_dirty.swap(false, Ordering::AcqRel)
}

/// Whether PaperDesk should show the "enable history?" prompt (first decision only).
pub fn should_prompt_enable(entry: &ProjectHistoryEntry) -> bool {
    !entry.asked_enable
}

/// Whether to ask about using an existing `.git` (after user enabled history).
pub fn should_prompt_existing_git(root: &Path, entry: &ProjectHistoryEntry) -> bool {
    entry.enabled && has_git_dir(root) && !entry.asked_existing_git
}

/// History is active only when enabled, a `.git` directory exists, and the user chose to use Git
/// (including repos created by PaperDesk via `git init`, where we set `use_existing_git: true`).
pub fn history_active(root: &Path, entry: &ProjectHistoryEntry) -> bool {
    entry.enabled && has_git_dir(root) && matches!(entry.use_existing_git, Some(true))
}

pub fn respond_enable(state: &AppState, root: PathBuf, enable: bool) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("project path: {e}"))?;
    let had_git = has_git_dir(&root);
    update_entry(state, &root, |e| {
        e.asked_enable = true;
        e.enabled = enable;
        if !enable {
            e.asked_existing_git = false;
            e.use_existing_git = None;
        }
    })?;

    if enable && !had_git {
        let _lock = state.history_git_lock.lock().map_err(|e| e.to_string())?;
        Repository::init(&root).map_err(|e| format!("git init: {e}"))?;
        // Skip the "use existing repo?" prompt for repos we just created.
        update_entry(state, &root, |e| {
            e.asked_existing_git = true;
            e.use_existing_git = Some(true);
        })?;
    }
    Ok(())
}

pub fn respond_existing_git(
    state: &AppState,
    root: PathBuf,
    use_existing: bool,
) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("project path: {e}"))?;
    if !has_git_dir(&root) {
        return Err("no git repository in project".into());
    }
    update_entry(state, &root, |e| {
        e.asked_existing_git = true;
        e.use_existing_git = Some(use_existing);
        if !use_existing {
            e.enabled = false;
        }
    })?;
    Ok(())
}

fn paperdesk_sig(_repo: &Repository) -> Result<Signature<'_>, git2::Error> {
    Signature::now("PaperDesk", "paperdesk@local")
}

fn restore_index_after_checkpoint(
    index: &mut git2::Index,
    head_tree: &Option<git2::Tree<'_>>,
) -> Result<(), String> {
    if let Some(tree) = head_tree {
        index.read_tree(tree).map_err(|e| e.to_string())?;
        index.write().map_err(|e| e.to_string())?;
    } else {
        index.clear().map_err(|e| e.to_string())?;
        index.write().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Create a checkpoint commit on `refs/paperdesk/history` from the current working tree.
/// Restores the Git index to match `HEAD` afterwards so the user's staging area is unchanged.
///
/// Returns `None` when the snapshot tree matches the current tip of `refs/paperdesk/history`
/// (no new commit — avoids empty / duplicate checkpoints).
pub fn create_checkpoint(repo: &Repository, message: &str) -> Result<Option<Oid>, String> {
    let mut index = repo.index().map_err(|e| e.to_string())?;

    let head_tree = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_tree().ok());

    if let Some(ref tree) = head_tree {
        index.read_tree(tree).map_err(|e| e.to_string())?;
    } else {
        index.clear().map_err(|e| e.to_string())?;
    }

    index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .map_err(|e| e.to_string())?;

    let tree_id = index
        .write_tree_to(repo)
        .map_err(|e| e.to_string())?;

    let parent: Option<Commit<'_>> = repo
        .find_reference(HISTORY_REF)
        .ok()
        .and_then(|r| r.peel_to_commit().ok());

    if let Some(p) = &parent {
        let parent_tree = p.tree().map_err(|e| e.to_string())?;
        if parent_tree.id() == tree_id {
            restore_index_after_checkpoint(&mut index, &head_tree)?;
            return Ok(None);
        }
    }

    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;

    let sig = paperdesk_sig(repo).map_err(|e| e.to_string())?;

    let parents: Vec<&Commit<'_>> = parent.as_ref().into_iter().collect();

    let commit_id = repo
        .commit(None, &sig, &sig, message, &tree, &parents)
        .map_err(|e| e.to_string())?;

    repo.reference(HISTORY_REF, commit_id, true, "PaperDesk checkpoint")
        .map_err(|e| e.to_string())?;

    restore_index_after_checkpoint(&mut index, &head_tree)?;

    Ok(Some(commit_id))
}

pub fn open_repo(root: &Path) -> Result<Repository, String> {
    Repository::open(root).map_err(|e| format!("git open: {e}"))
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCommitSummary {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub time_unix: i64,
}

/// Lists checkpoints from newest to oldest along the first-parent chain of
/// `refs/paperdesk/history` (deterministic order; avoids time-sorted revwalk quirks).
pub fn list_commits(repo: &Repository, limit: usize) -> Result<Vec<HistoryCommitSummary>, String> {
    let Ok(history_ref) = repo.find_reference(HISTORY_REF) else {
        return Ok(Vec::new());
    };
    let Some(mut oid) = history_ref.target() else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for _ in 0..limit {
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => break,
        };
        let msg = commit
            .message()
            .map(|m| m.trim().to_string())
            .unwrap_or_default();
        let time = commit.time().seconds();
        let full = oid.to_string();
        let short = full.chars().take(8).collect::<String>();
        out.push(HistoryCommitSummary {
            id: full,
            short_id: short,
            message: msg,
            time_unix: time,
        });
        if commit.parent_count() == 0 {
            break;
        }
        oid = match commit.parent(0) {
            Ok(parent) => parent.id(),
            Err(_) => break,
        };
    }
    Ok(out)
}

pub fn diff_commit_to_workdir(repo: &Repository, commit_id: &str) -> Result<String, String> {
    let oid = Oid::from_str(commit_id).map_err(|e| e.to_string())?;
    let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
    let tree = commit.tree().map_err(|e| e.to_string())?;

    let diff = repo
        .diff_tree_to_workdir(Some(&tree), None)
        .map_err(|e| e.to_string())?;

    const MAX_OUT: usize = 512 * 1024;
    let mut out = String::new();
    // Only prefix context/add/delete lines with their marker. libgit2 uses other origins
    // (e.g. 'F' file header, 'H' hunk header) where `content` already contains the full line;
    // prepending those chars would corrupt the patch ("Fdiff --git …").
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        if out.len() < MAX_OUT {
            let origin = line.origin();
            let content = String::from_utf8_lossy(line.content());
            match origin {
                ' ' | '+' | '-' => {
                    out.push(origin);
                    out.push_str(&content);
                }
                _ => out.push_str(&content),
            }
        }
        true
    })
    .map_err(|e| e.to_string())?;

    if out.len() >= MAX_OUT {
        out.push_str("\n\n[diff truncated]\n");
    }
    Ok(out)
}

/// Restore paths from `commit_id` into the working tree (and index for those paths).
/// If `paths` is empty, restores the entire tree from that commit.
pub fn restore_paths(
    repo: &Repository,
    commit_id: &str,
    paths: &[String],
) -> Result<(), String> {
    let oid = Oid::from_str(commit_id).map_err(|e| e.to_string())?;
    let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
    let tree = commit.tree().map_err(|e| e.to_string())?;

    let mut opts = git2::build::CheckoutBuilder::new();
    opts.force();
    if paths.is_empty() {
        // full tree
    } else {
        for p in paths {
            opts.path(p.replace('\\', "/"));
        }
    }

    repo
        .checkout_tree(tree.as_object(), Some(&mut opts))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// If history is active and `force` or the editor marked the tree dirty, write a checkpoint.
pub fn try_checkpoint(
    state: &AppState,
    root: &Path,
    message: &str,
    force: bool,
) -> Result<Option<String>, String> {
    let entry = get_entry(state, root)?;
    if !history_active(root, &entry) {
        return Ok(None);
    }

    let had_dirty = if force {
        false
    } else {
        take_dirty(state)
    };
    let should_commit = force || had_dirty;
    if !should_commit {
        return Ok(None);
    }

    let _lock = state.history_git_lock.lock().map_err(|e| e.to_string())?;
    let repo = match open_repo(root) {
        Ok(r) => r,
        Err(e) => {
            if had_dirty {
                note_dirty(state);
            }
            return Err(e);
        }
    };
    let msg = message.trim();
    let full_message = if msg.to_ascii_lowercase().starts_with("paperdesk:") {
        msg.to_string()
    } else {
        format!("paperdesk: {msg}")
    };
    let id = match create_checkpoint(&repo, &full_message) {
        Ok(id) => id,
        Err(e) => {
            if had_dirty {
                note_dirty(state);
            }
            return Err(e);
        }
    };
    Ok(id.map(|oid| oid.to_string()))
}
