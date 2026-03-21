use std::fs;
use std::io::Cursor;

use arboard::Clipboard;
use chrono::Local;
use image::{ImageBuffer, ImageFormat, Rgba};
use serde::Serialize;

use crate::project::history;
use crate::project::paths::join_under_root;
use crate::AppState;

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum ClipboardPasteForTypstResult {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "image")]
    Image { relative_path: String },
}

/// Reads the system clipboard: saves a bitmap as `assets/image-<local-timestamp>.png` or returns plain text.
#[tauri::command]
pub fn clipboard_paste_for_typst(
    state: tauri::State<'_, AppState>,
) -> Result<ClipboardPasteForTypstResult, String> {
    let mut clip = Clipboard::new().map_err(|e| format!("clipboard: {e}"))?;

    if let Ok(img) = clip.get_image() {
        let w = img.width;
        let h = img.height;
        if w == 0 || h == 0 {
            return Err("clipboard image has zero size".into());
        }
        let pixel_bytes = w
            .checked_mul(h)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| "clipboard image too large".to_string())?;
        if img.bytes.len() < pixel_bytes {
            return Err("incomplete clipboard image data".into());
        }
        let rgba: Vec<u8> = img.bytes[..pixel_bytes].to_vec();
        let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(w as u32, h as u32, rgba)
                .ok_or_else(|| "invalid clipboard image buffer".to_string())?;

        let mut png_bytes: Vec<u8> = Vec::new();
        image::DynamicImage::ImageRgba8(buffer)
            .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
            .map_err(|e| format!("png encode: {e}"))?;

        let now = Local::now();
        let name = format!(
            "image-{}-{:03}.png",
            now.format("%Y%m%d-%H%M%S"),
            now.timestamp_subsec_millis()
        );
        let relative_path = format!("assets/{name}");

        let guard = state.project_root.lock().map_err(|e| e.to_string())?;
        let root = guard
            .clone()
            .ok_or_else(|| "no project open".to_string())?;
        let path = join_under_root(&root, &relative_path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, &png_bytes).map_err(|e| e.to_string())?;
        drop(guard);
        history::note_dirty(&state);
        return Ok(ClipboardPasteForTypstResult::Image { relative_path });
    }

    match clip.get_text() {
        Ok(content) => Ok(ClipboardPasteForTypstResult::Text { content }),
        Err(_) => Ok(ClipboardPasteForTypstResult::None),
    }
}
