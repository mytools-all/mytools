use crate::services::settings::{AppSettings, SettingsState, Theme};
use tauri::{Emitter, Manager};

#[tauri::command]
pub fn settings_set_ui_state(
    window: tauri::Window,
    state: tauri::State<'_, SettingsState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    if !matches!(window.label(), "main" | "center" | "panel") {
        return Err("此操作仅限主程序页面".into());
    }
    state.set_ui_state(key.clone(), value.clone())?;
    for label in ["main", "center", "panel"] {
        let _ = window.emit_to(
            label,
            "settings:ui-state",
            serde_json::json!({"key":key,"value":value}),
        );
    }
    Ok(())
}

#[tauri::command]
pub fn settings_get(state: tauri::State<'_, SettingsState>) -> AppSettings {
    state.get()
}

#[tauri::command]
pub fn settings_set_theme(
    window: tauri::Window,
    state: tauri::State<'_, SettingsState>,
    theme: Theme,
) -> Result<(), String> {
    state.set_theme(theme)?;
    for label in ["main", "center", "panel", "floatball"] {
        let _ = window.emit_to(label, "settings:theme", theme);
    }
    crate::services::backups::audit(window.app_handle(), "settings.theme", None, true);
    Ok(())
}

#[tauri::command]
pub fn settings_set_usage(
    state: tauri::State<'_, SettingsState>,
    pinned: Vec<String>,
    recent: Vec<String>,
) -> Result<(), String> {
    state.set_usage(pinned, recent)
}
