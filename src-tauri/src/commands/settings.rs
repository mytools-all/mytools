use crate::services::settings::{AppSettings, SettingsState, Theme};

#[tauri::command]
pub fn settings_get(state: tauri::State<'_, SettingsState>) -> AppSettings {
    state.get()
}

#[tauri::command]
pub fn settings_set_theme(
    state: tauri::State<'_, SettingsState>,
    theme: Theme,
) -> Result<(), String> {
    state.set_theme(theme)
}

#[tauri::command]
pub fn settings_set_usage(
    state: tauri::State<'_, SettingsState>,
    pinned: Vec<String>,
    recent: Vec<String>,
) -> Result<(), String> {
    state.set_usage(pinned, recent)
}
