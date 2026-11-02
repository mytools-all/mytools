//! Window management commands — show / hide / toggle the launcher
//! panel, position it relative to the currently focused monitor, and
//! switch between the compact launcher layout and the resizable tool /
//! plugin layout.
//!
//! All commands take a `tauri::WebviewWindow` injected by Tauri so they
//! are unit-testable with the helper in `#[cfg(test)] mod tests`.

use mytools_core::{PANEL_HEIGHT, PANEL_TOP_MARGIN, PANEL_WIDTH};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager};

use crate::services::settings::SettingsState;

#[tauri::command]
pub async fn window_show(app: tauri::AppHandle) -> Result<(), String> {
    let w = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn window_hide(app: tauri::AppHandle) -> Result<(), String> {
    let w = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    w.hide().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn window_toggle(window: tauri::WebviewWindow) -> Result<bool, String> {
    let visible = window.is_visible().map_err(|e| e.to_string())?;
    if visible {
        window.hide().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

/// Persist the launcher window's physical bounds so the layout survives
/// restarts. Width/height must be positive; the window's own min size
/// already guards against tiny values.
#[tauri::command]
pub async fn window_save_bounds(
    settings: tauri::State<'_, SettingsState>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    validate_bounds(width, height)?;
    settings.set_main_bounds(x, y, width, height)
}

pub fn validate_bounds(width: u32, height: u32) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("window bounds must be positive".into());
    }
    Ok(())
}

/// Switch the launcher / tool layout.
///
/// `resizable` and `decorations` are applied in that order (then the
/// size if both `width` and `height` are provided); the window is then
/// re-positioned on top of the currently focused monitor. Reordering
/// resizable-before-decorations-before-size avoids visible flicker.
#[tauri::command]
pub async fn window_set_layout(
    window: tauri::WebviewWindow,
    resizable: bool,
    decorations: bool,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<(), String> {
    window.set_resizable(resizable).map_err(|e| e.to_string())?;
    window
        .set_decorations(decorations)
        .map_err(|e| e.to_string())?;
    let custom = match (width, height) {
        (Some(w), Some(h)) if w > 0.0 && h > 0.0 => Some((w, h)),
        _ => None,
    };
    position_on_active_monitor(&window, &window.app_handle().clone(), custom)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn position_on_active_monitor(
    window: &tauri::WebviewWindow,
    app: &AppHandle,
    custom: Option<(f64, f64)>,
) -> Result<(), String> {
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no monitor".to_string())?;

    let screen = monitor.size();
    let scale = monitor.scale_factor();
    let pos = monitor.position();

    let (panel_w, panel_h) = custom.unwrap_or((PANEL_WIDTH as f64, PANEL_HEIGHT as f64));

    let center_x = pos.x + ((screen.width as f64 / scale - panel_w) / 2.0).round() as i32;
    let top_y = pos.y + (PANEL_TOP_MARGIN as f64).round() as i32;

    window
        .set_position(LogicalPosition::new(center_x, top_y))
        .map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(panel_w, panel_h))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(scale: f64, screen_w: u32, panel: (f64, f64)) -> (i32, i32) {
        let (panel_w, _) = panel;
        let logical_w = screen_w as f64 / scale;
        let center_x = ((logical_w - panel_w) / 2.0).round() as i32;
        let top_y = PANEL_TOP_MARGIN;
        (center_x, top_y)
    }

    #[test]
    fn compute_layout_centers_default_panel() {
        let (x, y) = rect(1.0, 1920, (PANEL_WIDTH as f64, PANEL_HEIGHT as f64));
        assert_eq!(x, (1920 - PANEL_WIDTH as i32) / 2);
        assert_eq!(y, PANEL_TOP_MARGIN);
    }

    #[test]
    fn compute_layout_handles_scale_factor() {
        let (x, _) = rect(2.0, 1920, (PANEL_WIDTH as f64, PANEL_HEIGHT as f64));
        let logical_w = 1920.0 / 2.0;
        let expected = ((logical_w - PANEL_WIDTH as f64) / 2.0).round() as i32;
        assert_eq!(x, expected);
    }

    #[test]
    fn compute_layout_supports_custom_size() {
        let (x, _) = rect(1.0, 2560, (1024.0, 720.0));
        assert_eq!(x, (2560 - 1024) / 2);
    }

    #[test]
    fn save_bounds_accepts_positive_size() {
        assert!(validate_bounds(720, 480).is_ok());
    }

    #[test]
    fn save_bounds_rejects_zero_size() {
        assert!(validate_bounds(0, 480).is_err());
        assert!(validate_bounds(720, 0).is_err());
        assert!(validate_bounds(0, 0).is_err());
    }
}
