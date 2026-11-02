use mytools_core::capability::{
    Capability, CapabilityCall, CapabilityError, CapabilityRegistry, Permission,
};
use serde_json::Value;
use tauri::Manager;
use tauri::{AppHandle, Runtime};
const MAIN_WINDOW: &str = "main";

pub(super) fn register<R: Runtime>(
    app: &AppHandle<R>,
    registry: &mut CapabilityRegistry,
) -> Result<(), CapabilityError> {
    let show_app = app.clone();
    registry.register(Capability::new(
        "window.show",
        Permission::Write,
        "Show the main window",
        Box::new(move |_call: &CapabilityCall| {
            if let Some(window) = show_app.get_webview_window(MAIN_WINDOW) {
                window
                    .show()
                    .map_err(|e| CapabilityError::handler_failed("window.show", e.to_string()))?;
            }
            Ok(Value::Null)
        }),
    ))?;
    let hide_app = app.clone();
    registry.register(Capability::new(
        "window.hide",
        Permission::Write,
        "Hide the main window",
        Box::new(move |_call: &CapabilityCall| {
            if let Some(window) = hide_app.get_webview_window(MAIN_WINDOW) {
                window
                    .hide()
                    .map_err(|e| CapabilityError::handler_failed("window.hide", e.to_string()))?;
            }
            Ok(Value::Null)
        }),
    ))?;
    let resize_app = app.clone();
    registry.register(Capability::new(
        "window.resize",
        Permission::Write,
        "Resize the main window",
        Box::new(move |call: &CapabilityCall| {
            let height = call
                .args
                .get("height")
                .and_then(Value::as_f64)
                .ok_or_else(|| CapabilityError::invalid_args("window.resize expects { height }"))?;
            if let Some(window) = resize_app.get_webview_window(MAIN_WINDOW) {
                let size = window
                    .outer_size()
                    .map_err(|e| CapabilityError::handler_failed("window.resize", e.to_string()))?;
                let scale = window.scale_factor().unwrap_or(1.0);
                window
                    .set_size(tauri::PhysicalSize::new(
                        size.width,
                        (height * scale).round().max(1.0) as u32,
                    ))
                    .map_err(|e| CapabilityError::handler_failed("window.resize", e.to_string()))?;
            }
            Ok(Value::Null)
        }),
    ))?;
    Ok(())
}
