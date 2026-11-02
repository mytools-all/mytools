use mytools_core::capability::{
    Capability, CapabilityCall, CapabilityError, CapabilityRegistry, Permission,
};
use serde_json::Value;
use tauri::{AppHandle, Runtime};

pub(super) fn register<R: Runtime>(
    app: &AppHandle<R>,
    registry: &mut CapabilityRegistry,
) -> Result<(), CapabilityError> {
    registry.register(Capability::new(
        "app.list",
        Permission::Read,
        "List indexed applications",
        Box::new(|_call: &CapabilityCall| {
            let apps = crate::services::apps::index::listing();
            serde_json::to_value(apps)
                .map_err(|e| CapabilityError::handler_failed("app.list", e.to_string()))
        }),
    ))?;
    let launch_app = app.clone();
    registry.register(Capability::new(
        "app.launch",
        Permission::Write,
        "Launch an indexed application",
        Box::new(move |call: &CapabilityCall| {
            let target = call
                .arg_str("target")
                .ok_or_else(|| CapabilityError::invalid_args("app.launch expects { target }"))?;
            crate::services::apps::launcher::open_target(&launch_app, target)
                .map_err(|e| CapabilityError::handler_failed("app.launch", e))?;
            Ok(Value::Null)
        }),
    ))?;
    Ok(())
}
