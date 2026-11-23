use tauri::Manager;

use crate::services::app_updates::{Snapshot, Updates};
use crate::services::settings::UpdateMode;

fn trusted(window: &tauri::Window) -> Result<(), String> {
    if matches!(window.label(), "main" | "center") {
        Ok(())
    } else {
        Err("此操作仅限主程序设置页面".into())
    }
}

#[tauri::command]
pub fn app_update_status(window: tauri::Window) -> Result<Snapshot, String> {
    trusted(&window)?;
    window.state::<Updates>().snapshot()
}

#[tauri::command]
pub async fn app_update_check(window: tauri::Window) -> Result<Snapshot, String> {
    trusted(&window)?;
    window.state::<Updates>().check(window.app_handle()).await
}

#[tauri::command]
pub fn app_update_set_mode(window: tauri::Window, mode: UpdateMode) -> Result<Snapshot, String> {
    trusted(&window)?;
    window
        .state::<Updates>()
        .set_mode(window.app_handle(), mode)
}

#[tauri::command]
pub async fn app_update_download(window: tauri::Window) -> Result<Snapshot, String> {
    trusted(&window)?;
    window
        .state::<Updates>()
        .download(window.app_handle())
        .await
}

#[tauri::command]
pub async fn app_update_install(
    window: tauri::Window,
    saved_and_confirmed: bool,
) -> Result<(), String> {
    trusted(&window)?;
    if !saved_and_confirmed {
        return Err("请先保存内容并确认安装更新".into());
    }
    window.state::<Updates>().install(window.app_handle()).await
}

#[tauri::command]
pub fn app_autostart_status(window: tauri::Window) -> Result<bool, String> {
    trusted(&window)?;
    #[cfg(windows)]
    {
        use tauri_plugin_autostart::ManagerExt;
        window.autolaunch().is_enabled().map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    Err("当前平台暂不支持开机自启".into())
}

#[tauri::command]
pub fn app_autostart_set(window: tauri::Window, enabled: bool) -> Result<bool, String> {
    trusted(&window)?;
    #[cfg(windows)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let manager = window.autolaunch();
        if enabled {
            manager.enable()
        } else {
            manager.disable()
        }
        .map_err(|e| e.to_string())?;
        manager.is_enabled().map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("当前平台暂不支持开机自启".into())
    }
}
