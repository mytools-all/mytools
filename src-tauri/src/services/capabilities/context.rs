use mytools_core::{
    capability::{Capability, CapabilityError, CapabilityRegistry, Permission},
    dir_grants::DirGrantStore,
};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tauri::{AppHandle, Manager, Runtime};

pub(super) fn register<R: Runtime>(
    app: &AppHandle<R>,
    registry: &mut CapabilityRegistry,
    grants: Arc<DirGrantStore>,
) -> Result<(), CapabilityError> {
    let app = app.clone();
    registry.register(Capability::new("context.read", Permission::Exec, "Read the one-shot context explicitly transferred from My Panel", Box::new(move |call| {
        let types: Vec<String> = serde_json::from_value(call.args.get("types").cloned().unwrap_or(Value::Null))
            .map_err(|_| CapabilityError::invalid_args("expected { types: ['files' | 'image' | 'text'] }"))?;
        if types.is_empty() || types.iter().any(|kind| !matches!(kind.as_str(), "files" | "image" | "text")) {
            return Err(CapabilityError::invalid_args("unsupported context type"));
        }
        let window = call.ctx.window_label.as_deref().ok_or_else(|| CapabilityError::invalid_args("missing window"))?;
        let payload = app.state::<crate::runtime::LaunchContexts>().take(window, call.ctx.scope())
            .map_err(|error| CapabilityError::handler_failed("context.read", error))?;
        if payload.is_null() { return Ok(Value::Null); }
        let mut result = json!({"version":1,"source":payload["source"]});
        for kind in &types {
            if kind != "files" {
                if let Some(value) = payload.get(kind) { result[kind] = value.clone(); }
                continue;
            }
            if let Some(files) = payload["files"].as_array() {
                let mut entries = Vec::new();
                for path in files.iter().filter_map(Value::as_str).take(1000) {
                    let path = Path::new(path);
                    let Ok(metadata) = std::fs::symlink_metadata(path) else { continue; };
                    if metadata.file_type().is_symlink() || !(metadata.is_file() || metadata.is_dir()) { continue; }
                    let Some(parent) = path.parent() else { continue; };
                    grants.grant(call.ctx.scope(), parent).map_err(|error| CapabilityError::handler_failed("context.read", error))?;
                    entries.push(json!({"path":path,"name":path.file_name().unwrap_or_default().to_string_lossy(),"directory":metadata.is_dir(),"size":metadata.len(),"modified":metadata.modified().ok().and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok()).map(|time|time.as_secs()).unwrap_or(0)}));
                }
                result["files"] = json!(entries);
            }
        }
        Ok(result)
    })))?;
    Ok(())
}
