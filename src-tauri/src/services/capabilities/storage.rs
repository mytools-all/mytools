use mytools_core::capability::{
    Capability, CapabilityCall, CapabilityError, CapabilityRegistry, Permission,
};
use serde_json::Value;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;
const CAPABILITY_STORE_FILE: &str = "plugin-store.json";

pub(super) fn register<R: Runtime>(
    app: &AppHandle<R>,
    registry: &mut CapabilityRegistry,
) -> Result<(), CapabilityError> {
    let store_path = crate::infrastructure::paths::config_root(app).join(CAPABILITY_STORE_FILE);
    let read_app = app.clone();
    let read_store_path = store_path.clone();
    registry.register(Capability::new(
        "storage.get",
        Permission::Read,
        "Read a key from the plugin private store",
        Box::new(move |call: &CapabilityCall| {
            let key = call
                .arg_str("key")
                .ok_or_else(|| CapabilityError::invalid_args("storage.get expects { key }"))?;
            let store = read_app
                .store(&read_store_path)
                .map_err(|e| CapabilityError::handler_failed("storage.get", e.to_string()))?;
            Ok(store.get(storage_key(call, key)).unwrap_or(Value::Null))
        }),
    ))?;
    let write_app = app.clone();
    let write_store_path = store_path.clone();
    registry.register(Capability::new(
        "storage.set",
        Permission::Write,
        "Write a key into the plugin private store",
        Box::new(move |call: &CapabilityCall| {
            let key = call
                .arg_str("key")
                .ok_or_else(|| CapabilityError::invalid_args("storage.set expects { key }"))?;
            let value = call.args.get("value").cloned().unwrap_or(Value::Null);
            let store = write_app
                .store(&write_store_path)
                .map_err(|e| CapabilityError::handler_failed("storage.set", e.to_string()))?;
            store.set(storage_key(call, key), value);
            store
                .save()
                .map_err(|e| CapabilityError::handler_failed("storage.set", e.to_string()))?;
            Ok(Value::Null)
        }),
    ))?;
    let delete_app = app.clone();
    let delete_store_path = store_path.clone();
    registry.register(Capability::new(
        "storage.del",
        Permission::Write,
        "Delete a key from the plugin private store",
        Box::new(move |call: &CapabilityCall| {
            let key = call
                .arg_str("key")
                .ok_or_else(|| CapabilityError::invalid_args("storage.del expects { key }"))?;
            let store = delete_app
                .store(&delete_store_path)
                .map_err(|e| CapabilityError::handler_failed("storage.del", e.to_string()))?;
            let existed = store.get(storage_key(call, key)).is_some();
            store.delete(storage_key(call, key));
            store
                .save()
                .map_err(|e| CapabilityError::handler_failed("storage.del", e.to_string()))?;
            Ok(Value::Bool(existed))
        }),
    ))?;
    Ok(())
}

pub fn storage_key(call: &CapabilityCall, key: &str) -> String {
    let namespace = call.arg_str("namespace").unwrap_or(call.ctx.scope());
    format!("{namespace}:{key}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mytools_core::capability::{CallContext, Origin};
    use serde_json::json;
    fn call_with(args: Value, scope: &str) -> CapabilityCall {
        CapabilityCall {
            id: "storage.get".into(),
            args,
            ctx: CallContext::new(Origin::Custom).with_plugin(scope),
        }
    }
    #[test]
    fn storage_key_uses_namespace_when_given() {
        let call = call_with(json!({ "key": "a", "namespace": "rubick:demo" }), "demo");
        assert_eq!(storage_key(&call, "a"), "rubick:demo:a");
    }

    #[test]
    fn storage_key_falls_back_to_scope() {
        let call = call_with(json!({ "key": "a" }), "demo");
        assert_eq!(storage_key(&call, "a"), "demo:a");
    }
}
