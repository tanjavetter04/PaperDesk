mod bib_watch;
mod commands;
mod project;
mod tinymist_preview;
mod typst_engine;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use tauri::Manager;

use commands::ai::{ai_chat, ai_get_status, ai_set_config};
use commands::compile_cmd::{compile_project, compile_project_at_path, export_pdf_to_path};
use commands::fs::{
    create_project_dir, list_project_files, move_project_path, read_text_file, write_text_file,
};
use commands::history_cmd::{
    history_checkpoint, history_diff_workdir, history_get_status, history_list_commits,
    history_respond_enable, history_respond_existing_git, history_restore,
};
use commands::project::{
    add_recent_project, close_project, create_empty_project, create_from_template, delete_project,
    duplicate_project, get_open_project, get_recent_projects, open_project, rename_project,
};
use bib_watch::restart_bib_watcher;
use tinymist_preview::{
    restart_tinymist_preview, start_tinymist_preview, tinymist_panel_scroll_to_source,
    TinymistSession,
};

/// Shared application state (current project + paths).
pub struct AppState {
    pub project_root: Mutex<Option<PathBuf>>,
    pub typst_package_cache: PathBuf,
    pub app_config_dir: PathBuf,
    /// Bundled `resources/fonts` for Typst (`FontSearcher::search_with`), if present.
    pub resource_fonts_dir: Option<PathBuf>,
    /// `resources/bin/tinymist` from build.rs, when present (packaged app or after local build).
    pub bundled_tinymist: Option<PathBuf>,
    pub tinymist: Mutex<Option<TinymistSession>>,
    /// Set when project files change on disk; consumed by history checkpoints.
    pub history_dirty: AtomicBool,
    /// Serialize Git history operations (commit / restore).
    pub history_git_lock: Mutex<()>,
    pub bib_watch: Mutex<Option<bib_watch::BibWatchSession>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let resolver = app.path();
            let app_config_dir = resolver
                .app_config_dir()
                .map_err(|e| format!("app_config_dir: {e}"))?;
            let typst_package_cache = app_config_dir.join("typst-packages");
            std::fs::create_dir_all(&typst_package_cache)
                .map_err(|e| format!("create typst cache: {e}"))?;
            std::fs::create_dir_all(&app_config_dir)
                .map_err(|e| format!("create config dir: {e}"))?;

            let resource_fonts_dir = resolver
                .resolve("fonts", tauri::path::BaseDirectory::Resource)
                .ok()
                .filter(|p| p.is_dir());

            let bundled_tinymist = {
                let name = if cfg!(target_os = "windows") {
                    "bin/tinymist.exe"
                } else {
                    "bin/tinymist"
                };
                resolver
                    .resolve(name, tauri::path::BaseDirectory::Resource)
                    .ok()
                    .filter(|p| p.is_file())
            };

            app.manage(AppState {
                project_root: Mutex::new(None),
                typst_package_cache,
                app_config_dir,
                resource_fonts_dir,
                bundled_tinymist,
                tinymist: Mutex::new(None),
                history_dirty: AtomicBool::new(false),
                history_git_lock: Mutex::new(()),
                bib_watch: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_recent_projects,
            add_recent_project,
            open_project,
            rename_project,
            duplicate_project,
            delete_project,
            get_open_project,
            close_project,
            list_project_files,
            create_project_dir,
            move_project_path,
            read_text_file,
            write_text_file,
            compile_project,
            compile_project_at_path,
            export_pdf_to_path,
            create_from_template,
            create_empty_project,
            start_tinymist_preview,
            restart_tinymist_preview,
            tinymist_panel_scroll_to_source,
            history_get_status,
            history_respond_enable,
            history_respond_existing_git,
            history_checkpoint,
            history_list_commits,
            history_diff_workdir,
            history_restore,
            restart_bib_watcher,
            ai_get_status,
            ai_set_config,
            ai_chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
