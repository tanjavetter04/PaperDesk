use std::fs;
use std::path::PathBuf;

use base64::Engine;

use crate::project::paths::MAIN_TYP;
use crate::typst_engine::{self, PaperDeskWorld};
use crate::typst_engine::CompileOutcome;
use crate::AppState;

/// Open editor buffer for live preview (compiled instead of on-disk text for that path).
#[derive(serde::Deserialize)]
pub struct PreviewSource {
    pub path: String,
    pub text: String,
    #[serde(default, alias = "cursorByteOffset")]
    pub cursor_byte_offset: Option<u32>,
}

#[tauri::command]
pub fn compile_project(
    state: tauri::State<'_, AppState>,
    preview_source: Option<PreviewSource>,
) -> Result<CompileOutcome, String> {
    let root = state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let preview_cursor = preview_source.as_ref().and_then(|p| {
        p.cursor_byte_offset
            .map(|o| (p.path.clone(), o as usize))
    });
    let overrides = preview_source
        .map(|p| vec![(p.path, p.text)])
        .unwrap_or_default();
    let preview_jump = preview_cursor
        .as_ref()
        .map(|(path, off)| (path.as_str(), *off));
    let mut world = PaperDeskWorld::new(
        root,
        MAIN_TYP,
        state.typst_package_cache.clone(),
        overrides,
        state.resource_fonts_dir.clone(),
    )
    .map_err(|e| e.to_string())?;

    match typst_engine::compile_to_pdf(&mut world, preview_jump) {
        Ok((pdf, diagnostics, preview_page)) => Ok(CompileOutcome {
            ok: true,
            pdf_base64: Some(
                base64::engine::general_purpose::STANDARD.encode(pdf),
            ),
            diagnostics,
            preview_page,
        }),
        Err(diagnostics) => Ok(CompileOutcome {
            ok: false,
            pdf_base64: None,
            diagnostics,
            preview_page: None,
        }),
    }
}

/// Compile a project at an arbitrary directory without changing the open project (hub thumbnails).
#[tauri::command]
pub fn compile_project_at_path(
    state: tauri::State<'_, AppState>,
    project_path: String,
) -> Result<CompileOutcome, String> {
    let root = PathBuf::from(project_path.trim());
    let root = root
        .canonicalize()
        .map_err(|e| format!("invalid project path: {e}"))?;
    if !root.is_dir() {
        return Err("project path must be a directory".into());
    }
    let mut world = PaperDeskWorld::new(
        root,
        MAIN_TYP,
        state.typst_package_cache.clone(),
        vec![],
        state.resource_fonts_dir.clone(),
    )
    .map_err(|e| e.to_string())?;

    match typst_engine::compile_to_pdf(&mut world, None) {
        Ok((pdf, diagnostics, preview_page)) => Ok(CompileOutcome {
            ok: true,
            pdf_base64: Some(
                base64::engine::general_purpose::STANDARD.encode(pdf),
            ),
            diagnostics,
            preview_page,
        }),
        Err(diagnostics) => Ok(CompileOutcome {
            ok: false,
            pdf_base64: None,
            diagnostics,
            preview_page: None,
        }),
    }
}

/// Writes compiled PDF to `path` (absolute path from the native save dialog on the frontend).
#[tauri::command]
pub fn export_pdf_to_path(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let root = state
        .project_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())?;
    let mut world = PaperDeskWorld::new(
        root,
        MAIN_TYP,
        state.typst_package_cache.clone(),
        vec![],
        state.resource_fonts_dir.clone(),
    )
    .map_err(|e| e.to_string())?;

    let (pdf, _, _) = typst_engine::compile_to_pdf(&mut world, None).map_err(|diags| {
        diags
            .into_iter()
            .map(|d| d.message)
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    let out = std::path::PathBuf::from(path.trim());
    fs::write(&out, pdf).map_err(|e| e.to_string())
}
