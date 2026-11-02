//! Clipboard commands — store + retrieve text history. The polling
//! task that fills the store is launched from `lib.rs::run`.

use mytools_core::clipboard::ClipboardStore;
use std::path::PathBuf;
use std::sync::Arc;

pub type SharedClipboardStore = Arc<ClipboardStore>;

pub fn open_store(path: PathBuf) -> Result<SharedClipboardStore, String> {
    ClipboardStore::open(&path)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

pub fn open_in_memory_store() -> Result<SharedClipboardStore, String> {
    ClipboardStore::open_in_memory()
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

pub fn read_text<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().read_text().map_err(|e| e.to_string())
}

pub fn write_text<R: tauri::Runtime>(app: &tauri::AppHandle<R>, text: &str) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(text.to_string())
        .map_err(|e| e.to_string())
}

pub fn write_image<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    image: &tauri::image::Image<'_>,
) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_image(image)
        .map_err(|e| e.to_string())
}
