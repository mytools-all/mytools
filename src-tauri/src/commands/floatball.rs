//! Floating ball window.
//!
//! A small transparent, borderless, always-on-top window that sits near
//! the bottom-right corner of the screen. It hosts a ball view
//! (`FloatballView.vue` inside `App.vue`'s per-label branch) that can be
//! dragged around and single-clicked to show / hide the launcher.
//!
//! Mirrors Rubick's super-panel approach: a runtime-created frameless
//! topmost window controlled from the main process. Positioning is done
//! with pointer tracking from the renderer (`FloatballView.vue`); the
//! final position is persisted via `floatball_save_position` so it
//! survives restarts.
//!
//! All coordinates are PHYSICAL pixels (consistent with what the renderer
//! gets from `outerPosition()`), so monitor math needs no scale factor.

use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::services::settings::SettingsState;

pub const FLOATBALL_LABEL: &str = "floatball";
pub const BALL_WIN_WIDTH: f64 = 64.0;
pub const BALL_WIN_HEIGHT: f64 = 64.0;
pub const BALL_MARGIN: f64 = 12.0;

/// Pure function: default physical (x, y) for the floatball on a monitor
/// of the given physical size (bottom-right, inset by `BALL_MARGIN`).
pub fn default_ball_pos(monitor_w: u32, monitor_h: u32) -> (i32, i32) {
    (
        (monitor_w as i32 - BALL_WIN_WIDTH as i32 - BALL_MARGIN as i32).max(0),
        (monitor_h as i32 - BALL_WIN_HEIGHT as i32 - BALL_MARGIN as i32).max(0),
    )
}

/// Pure function: keep the ball window fully inside a physical monitor
/// rect. Missing one axis on a "monitor" that is too small stays put.
pub fn clamp_ball_to_monitor(
    x: i32,
    y: i32,
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: u32,
    monitor_h: u32,
) -> (i32, i32) {
    let max_x = monitor_x + (monitor_w as i32 - BALL_WIN_WIDTH as i32 - BALL_MARGIN as i32).max(0);
    let max_y = monitor_y + (monitor_h as i32 - BALL_WIN_HEIGHT as i32 - BALL_MARGIN as i32).max(0);
    let cx = x.clamp(monitor_x, max_x);
    let cy = y.clamp(monitor_y, max_y);
    (cx, cy)
}

fn primary_monitor_rect(app: &tauri::AppHandle) -> Option<(i32, i32, u32, u32)> {
    let monitor = app.primary_monitor().ok()??;
    Some((
        monitor.position().x,
        monitor.position().y,
        monitor.size().width,
        monitor.size().height,
    ))
}

/// Create the floatball window (idempotent — no-op if already present).
pub fn ensure_floatball(app: &tauri::AppHandle, settings: &SettingsState) -> tauri::Result<()> {
    if app.get_webview_window(FLOATBALL_LABEL).is_some() {
        return Ok(());
    }

    let rect = primary_monitor_rect(app);
    let (x, y) = match (settings.get().floatball_x, settings.get().floatball_y) {
        (Some(x), Some(y)) => match rect {
            Some((mx, my, mw, mh)) => clamp_ball_to_monitor(x, y, mx, my, mw, mh),
            None => (x, y),
        },
        _ => rect
            .map(|(_, _, mw, mh)| default_ball_pos(mw, mh))
            .unwrap_or((100, 100)),
    };
    let _ = settings.set_floatball_position(x, y);

    let window =
        WebviewWindowBuilder::new(app, FLOATBALL_LABEL, WebviewUrl::App("index.html".into()))
            .title("MyTools 悬浮球")
            .inner_size(BALL_WIN_WIDTH, BALL_WIN_HEIGHT)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(true)
            .focused(false)
            .build()?;

    window.set_position(PhysicalPosition::new(x, y))?;
    window.set_size(PhysicalSize::new(
        BALL_WIN_WIDTH as u32,
        BALL_WIN_HEIGHT as u32,
    ))?;
    tracing::info!("floatball window created at physical ({x}, {y})");
    Ok(())
}

/// Remove the floatball window (no-op if absent).
pub fn remove_floatball(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(FLOATBALL_LABEL) {
        let _ = window.close();
    }
}

/// Show / hide the launcher from the floatball (which cannot use
/// `window_show`/`window_hide` — those target the calling window).
#[tauri::command]
pub async fn floatball_toggle_main(app: tauri::AppHandle) -> Result<bool, String> {
    let Some(main) = app.get_webview_window("main") else {
        return Err("main window not found".into());
    };
    let visible = main.is_visible().map_err(|e| e.to_string())?;
    if visible {
        main.hide().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        main.show().map_err(|e| e.to_string())?;
        main.set_focus().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

/// Persist the ball's physical position, clamped inside the current
/// primary monitor.
#[tauri::command]
pub async fn floatball_save_position(
    app: tauri::AppHandle,
    settings: tauri::State<'_, SettingsState>,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let (cx, cy) = primary_monitor_rect(&app)
        .map(|(mx, my, mw, mh)| clamp_ball_to_monitor(x, y, mx, my, mw, mh))
        .unwrap_or((x, y));
    settings.set_floatball_position(cx, cy)?;
    if let Some(window) = app.get_webview_window(FLOATBALL_LABEL) {
        let _ = window.set_position(PhysicalPosition::new(cx, cy));
    }
    Ok(())
}

/// Toggle floatball visibility at runtime (settings + window lifecycle).
#[tauri::command]
pub async fn plugin_set_floatball_enabled(
    app: tauri::AppHandle,
    settings: tauri::State<'_, SettingsState>,
    on: bool,
) -> Result<(), String> {
    settings.set_floatball_enabled(on)?;
    if on {
        ensure_floatball(&app, &settings).map_err(|e| e.to_string())?;
    } else {
        remove_floatball(&app);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_positions_to_bottom_right() {
        let (x, y) = default_ball_pos(1920, 1080);
        assert_eq!(x, 1920 - BALL_WIN_WIDTH as i32 - BALL_MARGIN as i32);
        assert_eq!(y, 1080 - BALL_WIN_HEIGHT as i32 - BALL_MARGIN as i32);
    }

    #[test]
    fn default_pos_clamps_on_tiny_monitor() {
        let (x, y) = default_ball_pos(40, 40);
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn clamp_keeps_ball_inside_monitor() {
        let (cx, cy) = clamp_ball_to_monitor(2000, 2000, 0, 0, 1920, 1080);
        assert_eq!(cx, 1920 - BALL_WIN_WIDTH as i32 - BALL_MARGIN as i32);
        assert_eq!(cy, 1080 - BALL_WIN_HEIGHT as i32 - BALL_MARGIN as i32);
    }

    #[test]
    fn clamp_pulls_back_off_screen_left_top() {
        let (cx, cy) = clamp_ball_to_monitor(-50, -50, 0, 0, 1920, 1080);
        assert_eq!(cx, 0);
        assert_eq!(cy, 0);
    }

    #[test]
    fn clamp_respects_nonzero_monitor_origin() {
        let (cx, _) = clamp_ball_to_monitor(0, 300, 100, 0, 1920, 1080);
        assert_eq!(cx, 100);
    }
}
