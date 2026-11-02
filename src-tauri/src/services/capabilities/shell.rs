use mytools_core::capability::{
    Capability, CapabilityCall, CapabilityError, CapabilityRegistry, Permission,
};
use serde_json::Value;
use tauri::{AppHandle, Runtime};
use tauri_plugin_opener::OpenerExt;

pub(super) fn register<R: Runtime>(
    app: &AppHandle<R>,
    registry: &mut CapabilityRegistry,
) -> Result<(), CapabilityError> {
    let open_app = app.clone();
    registry.register(Capability::new(
        "shell.open",
        Permission::Write,
        "Open a URL or a file with the default handler",
        Box::new(move |call: &CapabilityCall| {
            if let Some(url) = call.arg_str("url") {
                open_app
                    .opener()
                    .open_url(url.to_string(), None::<&str>)
                    .map_err(|e| CapabilityError::handler_failed("shell.open", e.to_string()))?;
                return Ok(Value::Null);
            }
            if let Some(path) = call.arg_str("path") {
                open_app
                    .opener()
                    .open_path(path.to_string(), None::<&str>)
                    .map_err(|e| CapabilityError::handler_failed("shell.open", e.to_string()))?;
                return Ok(Value::Null);
            }
            Err(CapabilityError::invalid_args(
                "shell.open expects { url } or { path }",
            ))
        }),
    ))?;
    let reveal_app = app.clone();
    registry.register(Capability::new(
        "shell.reveal",
        Permission::Write,
        "Reveal a file in the system file manager",
        Box::new(move |call: &CapabilityCall| {
            let path = call
                .arg_str("path")
                .ok_or_else(|| CapabilityError::invalid_args("shell.reveal expects { path }"))?;
            reveal_app
                .opener()
                .reveal_item_in_dir(path)
                .map_err(|e| CapabilityError::handler_failed("shell.reveal", e.to_string()))?;
            Ok(Value::Null)
        }),
    ))?;
    Ok(())
}
