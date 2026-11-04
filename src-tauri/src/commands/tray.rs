//! System tray icon and menu.
//!
//! The launcher can show / hide / restart / open settings from the
//! tray. Creation is driven by `settings.tray_enabled`; toggling the
//! setting at runtime emits `"settings:tray"` from the frontend and
//! `lib.rs` translates that to `ensure_tray` / `remove_tray`.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

const TRAY_ID: &str = "mytools-launcher";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayMenuId {
    Settings,
    ShowLauncher,
    HideLauncher,
    Restart,
    Quit,
}

impl TrayMenuId {
    pub fn from_menu_id(id: &str) -> Option<Self> {
        match id {
            "settings" => Some(Self::Settings),
            "show-launcher" => Some(Self::ShowLauncher),
            "hide-launcher" => Some(Self::HideLauncher),
            "restart" => Some(Self::Restart),
            "quit" => Some(Self::Quit),
            _ => None,
        }
    }
}

/// Build the launcher context menu.
pub fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let show_launcher =
        MenuItem::with_id(app, "show-launcher", "显示主页", true, None::<&str>)?;
    let hide_launcher =
        MenuItem::with_id(app, "hide-launcher", "隐藏主页", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let restart = MenuItem::with_id(app, "restart", "重启", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &settings,
            &show_launcher,
            &hide_launcher,
            &separator,
            &restart,
            &quit,
        ],
    )
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    use tauri::Emitter;
    let Some(menu_id) = TrayMenuId::from_menu_id(id) else {
        return;
    };
    match menu_id {
        TrayMenuId::Settings => {
            let _ = app.emit("tray:menu", "settings");
        }
        TrayMenuId::ShowLauncher => {
            let _ = app.emit("tray:menu", "show-launcher");
        }
        TrayMenuId::HideLauncher => {
            let _ = app.emit("tray:menu", "hide-launcher");
        }
        TrayMenuId::Restart => {
            app.restart();
        }
        TrayMenuId::Quit => {
            app.exit(0);
        }
    }
}

/// Create the tray icon (idempotent — does nothing if already present).
pub fn ensure_tray(app: &AppHandle) -> tauri::Result<()> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }
    let menu = build_menu(app)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("MyTools")
        .menu(&menu)
        .show_menu_on_left_click(true);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let tray: TrayIcon<tauri::Wry> = builder.build(app)?;
    let app_handle = app.clone();
    tray.on_menu_event(move |_tray, event| {
        handle_menu_event(&app_handle, event.id.as_ref());
    });
    let app_handle = app.clone();
    tray.on_tray_icon_event(move |_tray, event| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            use tauri::Emitter;
            let _ = app_handle.emit("tray:menu", "show-launcher");
        }
    });
    Ok(())
}

/// Remove the tray icon (no-op if absent).
pub fn remove_tray(app: &AppHandle) {
    app.remove_tray_by_id(TRAY_ID);
}

/// Toggle tray visibility at runtime (settings + icon lifecycle).
#[tauri::command]
pub async fn plugin_set_tray_enabled(
    app: AppHandle,
    settings: tauri::State<'_, crate::services::settings::SettingsState>,
    on: bool,
) -> Result<(), String> {
    settings.set_tray_enabled(on)?;
    if on {
        ensure_tray(&app).map_err(|e| e.to_string())?;
    } else {
        remove_tray(&app);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_id_round_trips_for_each_action() {
        let cases = [
            (TrayMenuId::Settings, "settings"),
            (TrayMenuId::ShowLauncher, "show-launcher"),
            (TrayMenuId::HideLauncher, "hide-launcher"),
            (TrayMenuId::Restart, "restart"),
            (TrayMenuId::Quit, "quit"),
        ];
        for (id, str) in cases {
            assert_eq!(TrayMenuId::from_menu_id(str), Some(id));
        }
    }

    #[test]
    fn menu_id_unknown_string_is_none() {
        assert_eq!(TrayMenuId::from_menu_id("nope"), None);
        assert_eq!(TrayMenuId::from_menu_id(""), None);
    }
}
