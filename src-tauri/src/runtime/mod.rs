mod executor;
mod dropped_files;
pub use dropped_files::DroppedFiles;
pub use executor::CapabilityExecutor;

pub use mytools_core::plugin_session::PluginSessions;

use mytools_core::capability::{CallContext, CapabilityError, Origin};
use mytools_core::plugin_loader::LoadedPlugin;

pub fn authorize_call(
    sessions: &PluginSessions,
    loaded: &[LoadedPlugin],
    window: &str,
    scope: Option<&str>,
    session: Option<&str>,
    capability: &str,
) -> Result<CallContext, CapabilityError> {
    let deny = |reason: &str| CapabilityError::Denied {
        capability: capability.to_string(),
        reason: reason.to_string(),
    };
    let Some(plugin_id) = scope else {
        return if window == "main" && session.is_none() {
            Ok(CallContext::new(Origin::Official).with_window(window))
        } else {
            Err(deny("missing plugin identity"))
        };
    };
    if !session.is_some_and(|token| sessions.matches(window, token, plugin_id)) {
        return Err(deny("invalid plugin session or caller identity"));
    }
    let plugin = loaded
        .iter()
        .find(|p| p.enabled && p.manifest.name == plugin_id)
        .ok_or_else(|| deny("plugin is not installed or enabled"))?;
    if plugin.incompatible.is_some() {
        return Err(deny("plugin API version is incompatible"));
    }
    if plugin.restricts_capabilities() && !plugin.declares_capability(capability) {
        return Err(deny("capability is not declared by this plugin"));
    }
    Ok(CallContext::new(plugin.origin)
        .with_window(window)
        .with_plugin(plugin_id))
}

pub struct CapabilityRequest {
    pub capability: String,
    pub args: serde_json::Value,
    pub scope: Option<String>,
    pub session: Option<String>,
}

pub async fn dispatch(
    app: tauri::AppHandle,
    window: String,
    request: CapabilityRequest,
) -> Result<serde_json::Value, CapabilityError> {
    use crate::services::plugins::PluginRegistry;
    use mytools_core::capability::CapabilityRegistry;
    use std::sync::Arc;
    use tauri::Manager;
    let registry = Arc::clone(&app.state::<Arc<CapabilityRegistry>>());
    let plugins = Arc::clone(&app.state::<Arc<PluginRegistry>>());
    let capability = request.capability.clone();
    let executor = app.state::<CapabilityExecutor>();
    let handle = app.clone();
    executor
        .execute(&capability, move || {
            let ctx = authorize_call(
                &handle.state::<PluginSessions>(),
                &plugins.list(),
                &window,
                request.scope.as_deref(),
                request.session.as_deref(),
                &request.capability,
            )?;
            let result = registry.call(&request.capability, &request.args, &ctx);
            if let Err(err) = &result {
                tracing::debug!(capability = %request.capability, plugin = ?ctx.plugin_id,
                code = err.code(), "capability call rejected");
            }
            result
        })
        .await
}

pub fn is_plugin_window(label: &str) -> bool {
    label.starts_with("plugin-")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin() -> LoadedPlugin {
        LoadedPlugin {
            manifest: serde_json::from_value(serde_json::json!({
                "name": "demo", "version": "1.0.0", "main": "index.html", "features": []
            }))
            .expect("manifest"),
            dir: String::new(),
            main_url: String::new(),
            preload_url: None,
            icon: None,
            enabled: true,
            origin: Origin::Custom,
            ecosystem: None,
            declared_capabilities: vec!["storage.get".into()],
            incompatible: None,
        }
    }

    #[test]
    fn recognizes_only_plugin_window_labels() {
        assert!(is_plugin_window("plugin-demo"));
        assert!(is_plugin_window("plugin-mytools-hello-1"));
        assert!(!is_plugin_window("main"));
        assert!(!is_plugin_window("floatball"));
        assert!(!is_plugin_window(""));
    }

    #[test]
    fn authorizes_only_the_bound_enabled_plugin_and_declared_capability() {
        let sessions = PluginSessions::default();
        let token = sessions.open("main", "demo").expect("session");
        let mut loaded = vec![plugin()];
        let call = |plugins: &[LoadedPlugin], capability: &str| {
            authorize_call(
                &sessions,
                plugins,
                "main",
                Some("demo"),
                Some(&token),
                capability,
            )
        };
        let ctx = call(&loaded, "storage.get").expect("authorized");
        assert_eq!(ctx.origin, Origin::Custom);
        assert_eq!(ctx.plugin_id.as_deref(), Some("demo"));
        assert!(call(&loaded, "storage.set").is_err());
        assert!(call(&[], "storage.get").is_err());
        loaded[0].enabled = false;
        assert!(call(&loaded, "storage.get").is_err());
        loaded[0].enabled = true;
        loaded[0].incompatible = Some("newer API".into());
        assert!(call(&loaded, "storage.get").is_err());
    }

    #[test]
    fn rejects_missing_forged_and_cross_window_sessions() {
        let sessions = PluginSessions::default();
        let token = sessions.open("main", "demo").expect("session");
        let loaded = vec![plugin()];
        for (window, scope, session) in [
            ("main", Some("demo"), None),
            ("main", Some("demo"), Some("forged")),
            ("plugin-demo-1", Some("demo"), Some(token.as_str())),
            ("main", Some("other"), Some(token.as_str())),
            ("plugin-demo-1", None, None),
            ("floatball", None, None),
        ] {
            assert!(
                authorize_call(&sessions, &loaded, window, scope, session, "storage.get").is_err()
            );
        }
        assert!(authorize_call(&sessions, &loaded, "main", None, None, "storage.get").is_ok());
    }
}
