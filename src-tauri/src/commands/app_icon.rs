#[tauri::command]
pub async fn app_icon(path: String) -> Option<String> {
    crate::services::apps::resolve_icon(path).await
}
