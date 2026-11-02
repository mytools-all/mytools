pub mod index;
pub mod launcher;

pub async fn resolve_icon(path: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::infrastructure::app_icons::resolve_icon(&path)
    })
    .await
    .ok()
    .flatten()
}
