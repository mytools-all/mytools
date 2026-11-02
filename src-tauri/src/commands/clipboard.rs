use crate::services::clipboard::SharedClipboardStore;

#[tauri::command]
pub async fn clipboard_list(
    store: tauri::State<'_, SharedClipboardStore>,
    limit: Option<i64>,
) -> Result<Vec<mytools_core::ClipboardEntry>, String> {
    store.list(limit.unwrap_or(100)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clipboard_pin(
    store: tauri::State<'_, SharedClipboardStore>,
    id: i64,
    pinned: bool,
) -> Result<(), String> {
    store.pin(id, pinned).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clipboard_delete(
    store: tauri::State<'_, SharedClipboardStore>,
    id: i64,
) -> Result<(), String> {
    store.delete(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clipboard_clear(store: tauri::State<'_, SharedClipboardStore>) -> Result<(), String> {
    store.clear().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clipboard_write(app: tauri::AppHandle, text: String) -> Result<(), String> {
    crate::services::clipboard::write_text(&app, &text)
}
