use crate::services::backups::{self, Backups};
use mytools_core::backup::{RestoreOptions, Selection};
use tauri::Manager;

fn trusted(window: &tauri::Window) -> Result<(), String> {
    if matches!(window.label(), "main" | "center") {
        Ok(())
    } else {
        Err("备份与恢复仅限主程序设置页面".into())
    }
}

#[tauri::command]
pub async fn backup_info(
    window: tauri::Window,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    trusted(&window)?;
    tauri::async_runtime::spawn_blocking(move || backups::info(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn backup_last_result(
    window: tauri::Window,
    app: tauri::AppHandle,
) -> Result<Option<serde_json::Value>, String> {
    trusted(&window)?;
    Ok(backups::last_result(&app))
}

#[tauri::command]
pub async fn backup_export(
    window: tauri::Window,
    app: tauri::AppHandle,
    selection: Selection,
    path: String,
) -> Result<serde_json::Value, String> {
    trusted(&window)?;
    let state = app.state::<Backups>();
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "正在进行其他备份操作")?;
    let sessions = app.state::<crate::runtime::PluginSessions>();
    let _lease = sessions.begin_maintenance()?;
    let executor = app.state::<crate::runtime::CapabilityExecutor>();
    let _barrier = executor.quiesce().await?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move||{let manifest=backups::export(&handle,selection,std::path::PathBuf::from(&path))?;Ok(serde_json::json!({"path":path,"files":manifest.files.len(),"plugins":manifest.plugins.len(),"bytes":manifest.files.iter().map(|file|file.size).sum::<u64>()}))}).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn backup_preview(
    window: tauri::Window,
    app: tauri::AppHandle,
    path: String,
) -> Result<serde_json::Value, String> {
    trusted(&window)?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<Backups>();
        let _operation = state
            .operation
            .try_lock()
            .map_err(|_| "正在进行其他备份操作")?;
        state.preview(&handle, std::path::Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn backup_cancel_preview(
    window: tauri::Window,
    app: tauri::AppHandle,
    token: String,
) -> Result<(), String> {
    trusted(&window)?;
    app.state::<Backups>().cancel(&token);
    Ok(())
}

#[tauri::command]
pub async fn backup_restore(
    window: tauri::Window,
    app: tauri::AppHandle,
    token: String,
    options: RestoreOptions,
    confirmed: bool,
) -> Result<(), String> {
    trusted(&window)?;
    if !confirmed {
        return Err("恢复前必须确认覆盖并保存编辑内容".into());
    }
    let state = app.state::<Backups>();
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "正在进行其他备份操作")?;
    let sessions = app.state::<crate::runtime::PluginSessions>();
    let _lease = sessions.begin_maintenance()?;
    let executor = app.state::<crate::runtime::CapabilityExecutor>();
    let _barrier = executor.quiesce().await?;
    state.queue(&app, &token, options)?;
    app.restart();
}

#[tauri::command]
pub fn backup_open_logs(window: tauri::Window, app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    trusted(&window)?;
    let path = crate::infrastructure::paths::data_root(&app).join("logs");
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn backup_diagnostics(
    window: tauri::Window,
    app: tauri::AppHandle,
    path: String,
) -> Result<(), String> {
    trusted(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        backups::diagnostics(&app, std::path::Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}
