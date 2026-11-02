use crate::runtime::{CapabilityRequest, PluginSessions};
use crate::services::plugins::PluginRegistry;
use mytools_core::capability::CapabilityError;
use mytools_core::permission::{Grant, GrantStore};
use serde_json::Value;
use std::sync::Arc;
use tauri::{Manager, State, Window};

#[tauri::command]
pub fn plugin_session_open(
    window: Window,
    sessions: State<'_, PluginSessions>,
    plugins: State<'_, Arc<PluginRegistry>>,
    plugin: String,
) -> Result<String, String> {
    if window.label() != "main" && !window.label().starts_with("plugin-") {
        return Err("window cannot host plugins".into());
    }
    if !plugins
        .list()
        .iter()
        .any(|p| p.enabled && p.manifest.name == plugin)
    {
        return Err("plugin is not installed or enabled".into());
    }
    let _ = window.state::<crate::runtime::DroppedFiles>().take(window.label());
    sessions.open(window.label(), &plugin)
}

#[tauri::command]
pub fn plugin_session_close(window: Window, sessions: State<'_, PluginSessions>, session: String) {
    sessions.close(window.label(), &session);
}

#[tauri::command]
pub async fn plugin_capability_call(
    window: Window,
    capability: String,
    args: Option<Value>,
    scope: Option<String>,
    session: Option<String>,
) -> Result<Value, CapabilityError> {
    crate::runtime::dispatch(
        window.app_handle().clone(),
        window.label().to_string(),
        CapabilityRequest {
            capability,
            args: args.unwrap_or(Value::Null),
            scope,
            session,
        },
    )
    .await
}

#[tauri::command]
pub async fn plugin_grant_capability(
    grants: State<'_, Arc<GrantStore>>,
    plugin: String,
    capability: String,
) -> Result<(), String> {
    grants.grant(&plugin, &capability)
}

#[tauri::command]
pub async fn plugin_revoke_capability(
    grants: State<'_, Arc<GrantStore>>,
    plugin: String,
    capability: String,
) -> Result<bool, String> {
    grants.revoke(&plugin, &capability)
}

#[tauri::command]
pub async fn plugin_capability_grants(
    grants: State<'_, Arc<GrantStore>>,
) -> Result<Vec<Grant>, String> {
    Ok(grants.list())
}
