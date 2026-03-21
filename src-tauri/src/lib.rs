mod commands;
mod project;
mod typst_engine;

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::Manager;

use commands::compile_cmd::{compile_project, export_pdf_to_path};
use commands::fs::{list_project_files, read_text_file, write_text_file};
use commands::project::{
    add_recent_project, close_project, create_from_template, get_open_project, get_recent_projects,
    open_project,
};

/// Shared application state (current project + paths).
pub struct AppState {
    pub project_root: Mutex<Option<PathBuf>>,
    pub typst_package_cache: PathBuf,
    pub app_config_dir: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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

            app.manage(AppState {
                project_root: Mutex::new(None),
                typst_package_cache,
                app_config_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_recent_projects,
            add_recent_project,
            open_project,
            get_open_project,
            close_project,
            list_project_files,
            read_text_file,
            write_text_file,
            compile_project,
            export_pdf_to_path,
            create_from_template,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
