use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn resources_snapshot(app: AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move ||{
  let telemetry=crate::services::resources::snapshot(&app)?;
  let sessions=app.state::<crate::runtime::PluginSessions>().active_windows();
  Ok(json!({"telemetry":telemetry,"sessions":sessions.into_iter().map(|(window,plugin)|json!({"window":window,"plugin":plugin})).collect::<Vec<_>>()}))
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn resources_storage(app: AppHandle, force: Option<bool>) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::resource_storage::snapshot(&app, force.unwrap_or(false))
    })
    .await
    .map_err(|e| e.to_string())?
}
