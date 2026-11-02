#[tauri::command]
pub async fn system_get_path(app: tauri::AppHandle, name: String) -> Result<String, String> {
    crate::services::system::get_path(&app, &name)
}
