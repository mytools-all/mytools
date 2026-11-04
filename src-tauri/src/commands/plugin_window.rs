//! Detached plugin webview windows.
//!
//! `plugin_open_window` spins up a new top-level `WebviewWindow` that loads
//! the app itself at `#/plugin/<id>/<code>` (see `detached_app_url`).
//! Rendering the plugin route — instead of the plugin's raw `mainUrl` —
//! keeps `PluginView` alive in the detached window, so the bridge relay,
//! plugin lifecycle events and the error panel all keep working.
//! Each window is independent of the launcher: its own WebView2 process,
//! its own GPU surface, its own resize / decorate semantics. A crash in a
//! plugin window does not affect the launcher window or any other plugin
//! window.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::runtime::is_plugin_window;
use crate::services::plugins::SharedPluginRegistry;

const DEFAULT_WIDTH: u32 = 1024;
const DEFAULT_HEIGHT: u32 = 720;
const MIN_WIDTH: u32 = 640;
const MIN_HEIGHT: u32 = 400;
const POSITION_OFFSET_PX: i32 = 30;

static NEXT_LABEL_SEQ: AtomicU32 = AtomicU32::new(1);

#[tauri::command]
pub async fn plugin_center_open_window(app: tauri::AppHandle, section: String) -> Result<String, String> {
    if !matches!(section.as_str(), "store" | "installed" | "dev" | "settings") {
        return Err("invalid plugin center section".into());
    }
    let label = "center";
    if let Some(window) = app.get_webview_window(label) {
        crate::infrastructure::app_identity::apply(&window)?;
        window.eval(format!("location.hash='#/plugins?section={section}'")).map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    } else {
        let window = WebviewWindowBuilder::new(&app, label, WebviewUrl::App(format!("index.html#/plugins?section={section}").into()))
            .icon(crate::infrastructure::app_identity::icon()?).map_err(|error| error.to_string())?
            .title("MyTools · 插件中心").inner_size(1024.0, 760.0)
            .min_inner_size(720.0, 480.0).decorations(true).resizable(true)
            .visible(false).disable_drag_drop_handler().build().map_err(|e| e.to_string())?;
        crate::infrastructure::app_identity::apply(&window)?;
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
    }
    Ok(label.into())
}

#[derive(Serialize, Clone, Debug)]
pub struct PluginWindowInfo {
    pub label: String,
    pub plugin_id: String,
    pub title: String,
}

pub fn plugin_label(plugin_id: &str, feature: &str) -> String {
    format!("plugin-{plugin_id}-{feature}")
}

pub fn next_label(plugin_id: &str) -> String {
    let seq = NEXT_LABEL_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("plugin-{plugin_id}-{seq}")
}

#[derive(Debug, Clone, Copy)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Pure function: given the launcher's monitor info + a custom size,
/// compute the (x, y) of the new window (offset diagonally from the
/// launcher) and the size to use.
pub fn compute_window_rect(
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: u32,
    scale: f64,
    custom: Option<(u32, u32)>,
    offset_index: u32,
) -> WindowRect {
    let (w, h) = custom.unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT));
    let logical_w = monitor_w as f64 / scale;
    let center_x = monitor_x + ((logical_w - w as f64) / 2.0).round() as i32;
    let top_y = monitor_y + 80 + (offset_index as i32 * POSITION_OFFSET_PX);
    WindowRect {
        x: center_x,
        y: top_y,
        width: w,
        height: h,
    }
}

/// Percent-encode a single path segment for use in a URL path.
fn encode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Detached windows load the **app** at the plugin route rather than the
/// plugin's own HTML. That keeps `PluginView` alive in the new window, so
/// the bridge relay, plugin lifecycle events and the error panel all
/// keep working — loading `mainUrl` directly would leave `window.mytools`
/// calls pointing at `window.parent` (itself) with no relay listening.
pub fn detached_app_url(plugin_id: &str, feature: &str) -> String {
    format!(
        "index.html#/plugin/{}/{}",
        encode_segment(plugin_id),
        encode_segment(feature)
    )
}

#[tauri::command]
pub async fn plugin_open_window(
    window: tauri::Window,
    from_panel: Option<bool>,
    plugin_id: String,
    feature: String,
    registry: tauri::State<'_, SharedPluginRegistry>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let existing_label = plugin_label(&plugin_id, &feature);
    if let Some(existing) = app.get_webview_window(&existing_label) {
        crate::infrastructure::app_identity::apply(&existing)?;
        existing.show().map_err(|e| e.to_string())?;
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(existing_label);
    }

    let label = next_label(&plugin_id);
    let mut app_url = detached_app_url(&plugin_id, &feature);
    let manifest = {
        registry
            .list()
            .into_iter()
            .find(|p| p.manifest.name == plugin_id)
            .map(|p| p.manifest)
            .ok_or_else(|| format!("plugin '{plugin_id}' not found"))?
    };
    let title = manifest
        .plugin_name
        .clone()
        .unwrap_or(manifest.name.clone());
    if from_panel.unwrap_or(false) {
        if window.label() != "panel" { return Err("only the panel can transfer launch context".into()); }
        app.state::<crate::runtime::LaunchContexts>().bind(&label, &plugin_id)?;
        app_url.push_str("?context=panel");
    }

    let rect = if let Some(window) = app.get_webview_window("main") {
        if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
            let scale = monitor.scale_factor();
            let pos = monitor.position();
            let size = monitor.size();
            compute_window_rect(pos.x, pos.y, size.width, scale, None, 1)
        } else {
            WindowRect {
                x: 100,
                y: 100,
                width: DEFAULT_WIDTH,
                height: DEFAULT_HEIGHT,
            }
        }
    } else {
        WindowRect {
            x: 100,
            y: 100,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
        }
    };

    tracing::info!(
        "plugin_open_window: id={} feature={} label={} url={} rect=({}, {}, {}x{})",
        plugin_id,
        feature,
        label,
        app_url,
        rect.x,
        rect.y,
        rect.width,
        rect.height
    );

    let window = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(PathBuf::from(app_url)))
        .icon(crate::infrastructure::app_identity::icon()?).map_err(|error| error.to_string())?
        .title(&title)
        .inner_size(rect.width as f64, rect.height as f64)
        .min_inner_size(MIN_WIDTH as f64, MIN_HEIGHT as f64)
        .position(rect.x as f64, rect.y as f64)
        .resizable(true)
        .decorations(true)
        .always_on_top(false)
        .skip_taskbar(false)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|e| format!("failed to build webview window: {e}"))?;
    crate::infrastructure::app_identity::apply(&window)?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;

    let app_handle = app.clone();
    let label_for_event = label.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            tracing::info!("plugin window destroyed: {}", label_for_event);
            let _ = app_handle.emit(
                "plugin-window-closed",
                serde_json::json!({ "label": label_for_event }),
            );
        }
    });

    Ok(label)
}

#[tauri::command]
pub fn plugin_close_window(label: String, app: tauri::AppHandle) -> Result<(), String> {
    if !is_plugin_window(&label) {
        return Err(format!("{label} is not a plugin window"));
    }
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("plugin window '{label}' not found"))
    }
}

#[tauri::command]
pub fn plugin_list_windows(app: tauri::AppHandle) -> Vec<PluginWindowInfo> {
    let mut out = Vec::new();
    for (label, window) in app.webview_windows() {
        if !is_plugin_window(&label) {
            continue;
        }
        let title = window.title().unwrap_or_default();
        let plugin_id = label
            .strip_prefix("plugin-")
            .and_then(|rest| rest.split_once('-').map(|(id, _)| id.to_string()))
            .unwrap_or_default();
        out.push(PluginWindowInfo {
            label,
            plugin_id,
            title,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_window_rect_centers_and_offsets() {
        let r = compute_window_rect(0, 0, 2560, 1.0, None, 0);
        assert_eq!(r.x, (2560 - DEFAULT_WIDTH as i32) / 2);
        assert_eq!(r.y, 80);
        assert_eq!(r.width, DEFAULT_WIDTH);
        assert_eq!(r.height, DEFAULT_HEIGHT);
    }

    #[test]
    fn compute_window_rect_respects_scale() {
        let r = compute_window_rect(0, 0, 3840, 2.0, None, 0);
        let expected_x = ((3840.0 / 2.0 - DEFAULT_WIDTH as f64) / 2.0).round() as i32;
        assert_eq!(r.x, expected_x);
    }

    #[test]
    fn compute_window_rect_honors_custom_size() {
        let r = compute_window_rect(0, 0, 1920, 1.0, Some((800, 600)), 0);
        assert_eq!(r.width, 800);
        assert_eq!(r.height, 600);
    }

    #[test]
    fn compute_window_rect_offsets_with_index() {
        let r0 = compute_window_rect(0, 0, 1920, 1.0, None, 0);
        let r2 = compute_window_rect(0, 0, 1920, 1.0, None, 2);
        assert_eq!(r2.y - r0.y, 2 * POSITION_OFFSET_PX);
    }

    #[test]
    fn plugin_label_format() {
        assert_eq!(plugin_label("demo", "run"), "plugin-demo-run");
    }

    #[test]
    fn detached_app_url_points_at_the_plugin_route() {
        assert_eq!(
            detached_app_url("rubick-markdown", "editor"),
            "index.html#/plugin/rubick-markdown/editor"
        );
    }

    #[test]
    fn detached_app_url_encodes_scoped_and_special_ids() {
        // npm scoped names contain `@` and `/`, which would otherwise split
        // into extra router segments and resolve to the wrong plugin.
        assert_eq!(
            detached_app_url("@lucky-tools/excel-to-json", "main"),
            "index.html#/plugin/%40lucky-tools%2Fexcel-to-json/main"
        );
        assert_eq!(
            detached_app_url("a b", "c#d"),
            "index.html#/plugin/a%20b/c%23d"
        );
    }

    #[test]
    fn encode_segment_leaves_unreserved_characters_alone() {
        assert_eq!(encode_segment("aZ0-_.~"), "aZ0-_.~");
        assert_eq!(encode_segment(""), "");
    }

    #[test]
    fn next_label_is_unique() {
        let a = next_label("demo");
        let b = next_label("demo");
        assert_ne!(a, b);
        assert!(a.starts_with("plugin-demo-"));
    }
}
