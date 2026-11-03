#[tauri::command]
pub async fn plugin_catalog_fetch() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(crate::services::catalog::fetch_catalog)
        .await
        .map_err(|e| e.to_string())?
}
