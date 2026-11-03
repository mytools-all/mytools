//! MyTools — Rust backend entry point (Tauri shell).
//!
//! Pure-Rust logic (plugin manifest, sqlite helpers, fuzzy match) lives
//! in the `mytools-core` workspace member so it can be unit-tested
//! without a Tauri runtime (see docs/07-testing.md §3).

mod bootstrap;
mod commands;
mod infrastructure;
mod runtime;
mod services;

use tauri::{Emitter, Manager};

pub fn run() {
    tauri::Builder::default()
        .manage(runtime::PluginSessions::default())
        .manage(runtime::CapabilityExecutor::default())
        .manage(runtime::DroppedFiles::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                if window.state::<runtime::DroppedFiles>().record(window.label(), paths.clone()) {
                    tracing::info!(window=window.label(),count=paths.len(),"native window file drop");
                    let _ = window.app_handle().emit(&format!("mytools:files-dropped:{}",window.label()), ());
                }
            }
            if matches!(event, tauri::WindowEvent::Destroyed) {
                window
                    .state::<runtime::PluginSessions>()
                    .forget(window.label());
            }
        })
        .on_webview_event(|webview, event| {
            if let tauri::WebviewEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                let window = webview.window();
                if webview.state::<runtime::DroppedFiles>().record(window.label(), paths.clone()) {
                    tracing::info!(window=window.label(),count=paths.len(),"native webview file drop");
                    let _ = webview.app_handle().emit(&format!("mytools:files-dropped:{}",window.label()), ());
                }
            }
        })
        .setup(bootstrap::setup)
        .invoke_handler(tauri::generate_handler![
            commands::panel::panel_show,
            commands::panel::panel_middle_enabled,
            commands::panel::panel_set_middle_enabled,
            commands::ping::ping,
            commands::capability::plugin_session_open,
            commands::capability::plugin_session_close,
            commands::window::window_show,
            commands::window::window_hide,
            commands::window::window_toggle,
            commands::window::window_set_layout,
            commands::window::window_save_bounds,
            commands::hotkey::hotkey_is_registered,
            commands::hotkey::hotkey_unregister,
            commands::hotkey::hotkey_register,
            commands::system::system_get_path,
            commands::search::search,
            commands::app_launcher::app_launcher_scan,
            commands::app_launcher::app_launcher_search,
            commands::app_launcher::app_launcher_open,
            commands::app_icon::app_icon,
            commands::app_index::app_index_refresh,
            commands::app_index::app_index_list,
            commands::app_index::app_index_refresh_if_stale,
            commands::app_index::app_resolve,
            commands::color::color_parse,
            commands::color::color_parse_hex,
            commands::color::color_parse_rgb,
            commands::clipboard::clipboard_list,
            commands::clipboard::clipboard_pin,
            commands::clipboard::clipboard_delete,
            commands::clipboard::clipboard_clear,
            commands::clipboard::clipboard_write,
            commands::plugin::plugin_list,
            commands::plugin::plugin_reload,
            commands::plugin::plugin_enable,
            commands::plugin::plugin_disable,
            commands::plugin::plugin_uninstall,
            commands::plugin::plugin_install_dir,
            commands::plugin::plugin_install_pack,
            commands::plugin::plugin_open_dir,
            commands::settings::settings_get,
            commands::settings::settings_set_theme,
            commands::settings::settings_set_usage,
            commands::plugin::plugin_get_root,
            commands::plugin::plugin_set_root,
            commands::plugin::plugin_reset_root,
            commands::plugin::plugin_open_root,
            commands::plugin_npm::plugin_install_npm,
            commands::catalog::plugin_catalog_fetch,
            commands::plugin_window::plugin_open_window,
            commands::plugin_window::plugin_close_window,
            commands::plugin_window::plugin_list_windows,
            commands::capability::plugin_capability_call,
            commands::capability::plugin_grant_capability,
            commands::capability::plugin_revoke_capability,
            commands::capability::plugin_capability_grants,
            commands::dir_grants::dir_grants_list,
            commands::dir_grants::dir_grants_revoke,
            commands::health::webview_health,
            commands::tray::plugin_set_tray_enabled,
            commands::floatball::plugin_set_floatball_enabled,
            commands::floatball::floatball_toggle_main,
            commands::floatball::floatball_save_position,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MyTools");
}

#[cfg(test)]
mod architecture_tests;
