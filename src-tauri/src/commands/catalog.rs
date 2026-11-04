use crate::services::{catalog::CatalogSources, settings::SettingsState};

#[tauri::command]
pub async fn plugin_catalog_fetch(url: String) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || crate::services::catalog::fetch_catalog(&url))
        .await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn plugin_catalog_sources_get(state: tauri::State<'_, SettingsState>) -> CatalogSources {
    state.get().catalog_sources
}

#[tauri::command]
pub fn plugin_catalog_sources_set(state: tauri::State<'_, SettingsState>, sources: CatalogSources) -> Result<(), String> {
    state.set_catalog_sources(sources)
}
