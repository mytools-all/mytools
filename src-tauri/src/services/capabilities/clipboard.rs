use mytools_core::capability::{
    Capability, CapabilityCall, CapabilityError, CapabilityRegistry, Permission,
};
use serde_json::Value;
use tauri::Manager;
use tauri::{AppHandle, Runtime};

pub(super) fn register<R: Runtime>(
    app: &AppHandle<R>,
    registry: &mut CapabilityRegistry,
) -> Result<(), CapabilityError> {
    let clipboard_app = app.clone();
    registry.register(Capability::new(
        "clipboard.write",
        Permission::Write,
        "Write text or an image to the system clipboard",
        Box::new(move |call: &CapabilityCall| {
            if let Some(text) = call.arg_str("text") {
                crate::services::clipboard::write_text(&clipboard_app, text).map_err(|e| {
                    CapabilityError::handler_failed("clipboard.write", e.to_string())
                })?;
                return Ok(Value::Null);
            }
            if let Some(data_url) = call.arg_str("dataUrl") {
                let bytes = decode_data_url(data_url)
                    .map_err(|e| CapabilityError::handler_failed("clipboard.write", e))?;
                let image = image_from_bytes(&bytes)
                    .map_err(|e| CapabilityError::handler_failed("clipboard.write", e))?;
                crate::services::clipboard::write_image(&clipboard_app, &image).map_err(|e| {
                    CapabilityError::handler_failed("clipboard.write", e.to_string())
                })?;
                return Ok(Value::Null);
            }
            Err(CapabilityError::invalid_args(
                "clipboard.write expects { text } or { dataUrl }",
            ))
        }),
    ))?;
    let read_clipboard_app = app.clone();
    registry.register(Capability::new(
        "clipboard.read",
        Permission::Exec,
        "Read text from the system clipboard",
        Box::new(move |_call: &CapabilityCall| {
            let text = crate::services::clipboard::read_text(&read_clipboard_app)
                .map_err(|e| CapabilityError::handler_failed("clipboard.read", e.to_string()))?;
            Ok(serde_json::json!({ "text": text }))
        }),
    ))?;
    let history_app = app.clone();
    registry.register(Capability::new(
        "clipboard.history",
        Permission::Exec,
        "Read the clipboard history",
        Box::new(move |call: &CapabilityCall| {
            let store = history_app
                .try_state::<crate::services::clipboard::SharedClipboardStore>()
                .ok_or_else(|| {
                    CapabilityError::handler_failed("clipboard.history", "no clipboard store")
                })?;
            let limit = call
                .args
                .get("limit")
                .and_then(Value::as_i64)
                .unwrap_or(100)
                .clamp(1, 500);
            let entries = store.list(limit).map_err(|e| {
                CapabilityError::handler_failed("clipboard.history", format!("{e:?}"))
            })?;
            serde_json::to_value(entries)
                .map_err(|e| CapabilityError::handler_failed("clipboard.history", e.to_string()))
        }),
    ))?;
    let remove_app = app.clone();
    registry.register(Capability::new(
        "clipboard.remove",
        Permission::Write,
        "Delete one entry from the clipboard history",
        Box::new(move |call: &CapabilityCall| {
            let id =
                call.args.get("id").and_then(Value::as_i64).ok_or_else(|| {
                    CapabilityError::invalid_args("clipboard.remove expects { id }")
                })?;
            let store = remove_app
                .try_state::<crate::services::clipboard::SharedClipboardStore>()
                .ok_or_else(|| {
                    CapabilityError::handler_failed("clipboard.remove", "no clipboard store")
                })?;
            store.delete(id).map_err(|e| {
                CapabilityError::handler_failed("clipboard.remove", format!("{e:?}"))
            })?;
            Ok(Value::Bool(true))
        }),
    ))?;
    Ok(())
}

pub fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let comma = data_url
        .find(',')
        .ok_or_else(|| "data URL missing comma".to_string())?;
    let header = &data_url[..comma];
    if !header.contains(";base64") {
        return Err("only base64 data URLs are supported".to_string());
    }
    let body = &data_url[comma + 1..];
    base64_decode(body)
}

pub fn image_from_bytes(bytes: &[u8]) -> Result<tauri::image::Image<'_>, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("decode image: {e}"))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(tauri::image::Image::new_owned(rgba.into_raw(), w, h))
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for ch in input.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => continue,
            _ => return Err(format!("invalid base64 char: {ch}")),
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mytools_core::capability::{CallContext, Origin};
    #[test]
    fn clipboard_capabilities_are_not_reserved_for_official_plugins() {
        use mytools_core::capability::{Decision, Policy};
        let handler: mytools_core::capability::Handler =
            Box::new(|_call: &CapabilityCall| Ok(Value::Null));
        let read = Capability::new("clipboard.read", Permission::Exec, "d", handler);
        let policy = mytools_core::capability::DefaultPolicy;
        assert_eq!(
            policy.decide(&read, &CallContext::new(Origin::Custom)),
            Decision::Prompt
        );
        let handler: mytools_core::capability::Handler =
            Box::new(|_call: &CapabilityCall| Ok(Value::Null));
        let history = Capability::new("clipboard.history", Permission::Exec, "d", handler);
        assert_eq!(
            policy.decide(&history, &CallContext::new(Origin::Custom)),
            Decision::Prompt
        );
        let handler: mytools_core::capability::Handler =
            Box::new(|_call: &CapabilityCall| Ok(Value::Null));
        let remove = Capability::new("clipboard.remove", Permission::Write, "d", handler);
        assert_eq!(
            policy.decide(&remove, &CallContext::new(Origin::Custom)),
            Decision::Prompt
        );
        assert_eq!(
            policy.decide(&read, &CallContext::new(Origin::Official)),
            Decision::Allow
        );
    }

    #[test]
    fn decodes_base64_data_url() {
        let bytes = decode_data_url("data:image/png;base64,aGVsbG8=").unwrap();
        assert_eq!(bytes, b"hello");
    }

    #[test]
    fn rejects_non_base64_data_url() {
        assert!(decode_data_url("data:text/plain,hello").is_err());
        assert!(decode_data_url("not-a-data-url").is_err());
    }
}
