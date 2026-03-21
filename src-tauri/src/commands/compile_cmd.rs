use std::fs;
use std::path::Path;

use base64::Engine;
use serde::Deserialize;

use crate::typst_engine::{self, PaperDeskWorld};
use crate::typst_engine::CompileOutcome;
use crate::AppState;

#[derive(Deserialize)]
struct PaperDeskMeta {
    entry: Option<String>,
}

fn resolve_entry(root: &Path, entry: Option<String>) -> String {
    if let Some(e) = entry {
        let t = e.trim();
        if !t.is_empty() {
            return t.replace('\\', "/");
        }
    }
    let meta_path = root.join("paperdesk.json");
    if let Ok(raw) = fs::read_to_string(&meta_path) {
        if let Ok(m) = serde_json::from_str::<PaperDeskMeta>(&raw) {
            if let Some(e) = m.entry.filter(|s| !s.is_empty()) {
                return e.replace('\\', "/");
            }
        }
    }
    "main.typ".into()
}

#[tauri::command]
pub fn compile_project(
    state: tauri::State<'_, AppState>,
    entry: Option<String>,
) -> Result<CompileOutcome, String> {
    let root = state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let rel = resolve_entry(&root, entry);
    let mut world = PaperDeskWorld::new(
        root,
        &rel,
        state.typst_package_cache.clone(),
    )
    .map_err(|e| e.to_string())?;

    match typst_engine::compile_to_pdf(&mut world) {
        Ok((pdf, diagnostics)) => Ok(CompileOutcome {
            ok: true,
            pdf_base64: Some(
                base64::engine::general_purpose::STANDARD.encode(pdf),
            ),
            diagnostics,
        }),
        Err(diagnostics) => Ok(CompileOutcome {
            ok: false,
            pdf_base64: None,
            diagnostics,
        }),
    }
}

/// Writes compiled PDF to `path` (absolute path from the native save dialog on the frontend).
#[tauri::command]
pub fn export_pdf_to_path(
    state: tauri::State<'_, AppState>,
    path: String,
    entry: Option<String>,
) -> Result<(), String> {
    let root = state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let rel = resolve_entry(&root, entry);
    let mut world = PaperDeskWorld::new(
        root,
        &rel,
        state.typst_package_cache.clone(),
    )
    .map_err(|e| e.to_string())?;

    let (pdf, _) = typst_engine::compile_to_pdf(&mut world).map_err(|diags| {
        diags
            .into_iter()
            .map(|d| d.message)
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    let out = std::path::PathBuf::from(path.trim());
    fs::write(&out, pdf).map_err(|e| e.to_string())
}
