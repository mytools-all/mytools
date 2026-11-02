use crate::services::apps::launcher::AppEntry;

#[tauri::command]
pub async fn app_launcher_scan() -> Vec<AppEntry> {
    crate::services::apps::launcher::scan().await
}

#[tauri::command]
pub async fn app_launcher_search(query: String) -> Vec<AppEntry> {
    crate::services::apps::launcher::search(query).await
}

#[tauri::command]
pub async fn app_launcher_open(app: tauri::AppHandle, path: String) -> Result<(), String> {
    crate::services::apps::launcher::launch(app, path).await
}
