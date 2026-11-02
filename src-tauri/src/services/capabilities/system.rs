use crate::services::system::get_path;
use mytools_core::capability::{
    Capability, CapabilityCall, CapabilityError, CapabilityRegistry, Permission,
};
use serde_json::Value;
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

pub(super) fn register<R: Runtime>(
    app: &AppHandle<R>,
    registry: &mut CapabilityRegistry,
) -> Result<(), CapabilityError> {
    registry.register(Capability::new("system.networkdetails", Permission::Read, "Read the complete Windows ipconfig /all report", Box::new(|_| {
        crate::infrastructure::net_ifaces::configuration_report().map(Value::String)
            .map_err(|e| CapabilityError::handler_failed("system.networkdetails", e))
    })))?;
    let path_app = app.clone();
    registry.register(Capability::new(
        "system.path",
        Permission::Read,
        "Resolve a well known directory",
        Box::new(move |call: &CapabilityCall| {
            let name = call
                .arg_str("name")
                .ok_or_else(|| CapabilityError::invalid_args("system.path expects { name }"))?;
            let path = get_path(&path_app, name)
                .map_err(|e| CapabilityError::handler_failed("system.path", e))?;
            Ok(Value::String(path))
        }),
    ))?;
    let notify_app = app.clone();
    registry.register(Capability::new(
        "system.notify",
        Permission::Write,
        "Show a system notification",
        Box::new(move |call: &CapabilityCall| {
            let title = call
                .arg_str("title")
                .ok_or_else(|| CapabilityError::invalid_args("system.notify expects { title }"))?;
            let body = call.arg_str("body").unwrap_or_default();
            notify_app
                .notification()
                .builder()
                .title(title)
                .body(body)
                .show()
                .map_err(|e| CapabilityError::handler_failed("system.notify", e.to_string()))?;
            Ok(Value::Null)
        }),
    ))?;
    registry.register(Capability::new(
        "system.netinterfaces",
        Permission::Read,
        "List the network adapters of the machine",
        Box::new(|_call: &CapabilityCall| {
            let interfaces = crate::infrastructure::net_ifaces::list_interfaces();
            serde_json::to_value(interfaces)
                .map_err(|e| CapabilityError::handler_failed("system.netinterfaces", e.to_string()))
        }),
    ))?;
    Ok(())
}
