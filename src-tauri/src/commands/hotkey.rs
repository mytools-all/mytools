//! Global hotkey registration — Alt+Space by default, switchable at
//! runtime from the settings page (`hotkey_register`). The handler runs
//! in the Tauri main thread and toggles the launcher panel.

use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::services::settings::SettingsState;

/// Lazily-initialized handle to the currently active shortcut. The
/// mutex guards the value so the settings page can swap it at runtime
/// (`Shortcut::new` is not const, hence the `OnceLock`).
fn current_slot() -> &'static Mutex<Shortcut> {
    static CELL: OnceLock<Mutex<Shortcut>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Shortcut::new(Some(Modifiers::ALT), Code::Space)))
}

pub fn current_shortcut() -> Shortcut {
    *current_slot().lock().expect("hotkey lock poisoned")
}

/// Toggle the panel when the configured shortcut is pressed. Shared by
/// the initial install and every `hotkey_register` call.
fn bind_toggle_handler(app_handle: AppHandle, shortcut: Shortcut) -> Result<(), String> {
    let handler_app = app_handle.clone();
    app_handle
        .global_shortcut()
        .on_shortcut(shortcut, move |_window, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                if let Some(window) = handler_app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hotkey_is_registered(app: AppHandle) -> bool {
    app.global_shortcut().is_registered(current_shortcut())
}

#[tauri::command]
pub async fn hotkey_unregister(app: AppHandle) -> Result<(), String> {
    let sc = current_shortcut();
    app.global_shortcut()
        .unregister(sc)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hotkey_register(
    app: AppHandle,
    settings: State<'_, SettingsState>,
    combo: String,
) -> Result<(), String> {
    let parsed =
        Shortcut::from_str(&combo).map_err(|e| format!("invalid shortcut '{combo}': {e}"))?;
    let mut guard = current_slot().lock().expect("hotkey lock poisoned");
    let _ = app.global_shortcut().unregister(*guard);
    bind_toggle_handler(app.clone(), parsed)?;
    *guard = parsed;
    drop(guard);
    settings.set_hotkey(combo.clone())?;
    tracing::info!("global shortcut changed to {combo}");
    Ok(())
}

/// Install the configured shortcut handler at startup. Invalid or
/// unavailable combos fall back to Alt+Space.
pub fn install(app: &AppHandle, combo: &str) -> Result<(), String> {
    let parsed = Shortcut::from_str(combo).unwrap_or_else(|_| {
        tracing::warn!("invalid stored shortcut '{combo}', falling back to Alt+Space");
        Shortcut::new(Some(Modifiers::ALT), Code::Space)
    });
    let mut guard = current_slot().lock().expect("hotkey lock poisoned");
    if *guard != parsed {
        let _ = app.global_shortcut().unregister(*guard);
    }
    bind_toggle_handler(app.clone(), parsed)?;
    *guard = parsed;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcut_is_alt_space() {
        assert_eq!(
            current_shortcut(),
            Shortcut::new(Some(Modifiers::ALT), Code::Space)
        );
    }

    #[test]
    fn rubick_style_combos_parse() {
        assert!(Shortcut::from_str("Alt+Shift+X").is_ok());
        assert!(Shortcut::from_str("Ctrl+Shift+1").is_ok());
        assert!(Shortcut::from_str("Super+E").is_ok());
    }

    #[test]
    fn malformed_combos_are_rejected() {
        assert!(Shortcut::from_str("not a combo").is_err());
        assert!(Shortcut::from_str("").is_err());
        assert!(Shortcut::from_str("Alt++").is_err());
    }
}
