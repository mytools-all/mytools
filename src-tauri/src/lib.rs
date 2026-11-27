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
    let mut context = tauri::generate_context!();
    context.set_default_window_icon(Some(
        infrastructure::app_identity::icon().expect("invalid bundled MyTools icon"),
    ));
    let builder = tauri::Builder::default();
    #[cfg(windows)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _| {
        if !args.iter().any(|argument| argument == "--autostart") {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }));
    builder
        .manage(services::backups::Backups::default())
        .manage(commands::window_layout::LayoutState::default())
        .manage(runtime::PluginSessions::default())
        .manage(runtime::CapabilityExecutor::default())
        .manage(runtime::DroppedFiles::default())
        .manage(runtime::LaunchContexts::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .on_window_event(|window, event| {
            #[cfg(windows)]
            if matches!(event, tauri::WindowEvent::Focused(true)) {
                infrastructure::webview_idle::resume(window);
            }
            if matches!(
                event,
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
            ) {
                commands::window_layout::changed(window);
            }
            if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                if window
                    .state::<runtime::DroppedFiles>()
                    .record(window.label(), paths.clone())
                {
                    tracing::info!(
                        window = window.label(),
                        count = paths.len(),
                        "native window file drop"
                    );
                    let _ = window
                        .app_handle()
                        .emit(&format!("mytools:files-dropped:{}", window.label()), ());
                }
            }
            if matches!(event, tauri::WindowEvent::Destroyed) {
                commands::window_layout::forget(window);
                window
                    .state::<runtime::LaunchContexts>()
                    .forget(window.label());
                window
                    .state::<runtime::PluginSessions>()
                    .forget(window.label());
            }
        })
        .on_webview_event(|webview, event| {
            if let tauri::WebviewEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                let window = webview.window();
                if webview
                    .state::<runtime::DroppedFiles>()
                    .record(window.label(), paths.clone())
                {
                    tracing::info!(
                        window = window.label(),
                        count = paths.len(),
                        "native webview file drop"
                    );
                    let _ = webview
                        .app_handle()
                        .emit(&format!("mytools:files-dropped:{}", window.label()), ());
                }
            }
        })
        .on_page_load(|webview, _payload| {
            #[cfg(windows)]
            if let Err(error) = webview.with_webview(|platform| unsafe {
                let result = platform.controller().CoreWebView2()
                    .and_then(|view| view.Settings())
                    .and_then(|settings| settings.SetAreDefaultContextMenusEnabled(false));
                if let Err(error) = result {
                    tracing::warn!("disable default context menu: {error}");
                }
            }) {
                tracing::warn!("configure webview context menu: {error}");
            }
        })
        .setup(bootstrap::setup)
        .invoke_handler(tauri::generate_handler![
            commands::app_updates::app_update_status,
            commands::app_updates::app_update_check,
            commands::app_updates::app_update_set_mode,
            commands::app_updates::app_update_download,
            commands::app_updates::app_update_install,
            commands::app_updates::app_autostart_status,
            commands::app_updates::app_autostart_set,
            commands::backups::backup_info,
            commands::backups::backup_last_result,
            commands::backups::backup_export,
            commands::backups::backup_preview,
            commands::backups::backup_cancel_preview,
            commands::backups::backup_restore,
            commands::backups::backup_open_logs,
            commands::backups::backup_diagnostics,
            commands::resources::resources_snapshot,
            commands::resources::resources_storage,
            commands::panel::panel_show,
            commands::panel::panel_capture_image,
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
            commands::settings::settings_set_ui_state,
            commands::settings::settings_get,
            commands::settings::settings_set_theme,
            commands::settings::settings_set_usage,
            commands::plugin::plugin_get_root,
            commands::plugin::plugin_set_root,
            commands::plugin::plugin_reset_root,
            commands::plugin::plugin_open_root,
            commands::plugin_npm::plugin_install_npm,
            commands::plugin_npm::plugin_update_npm,
            commands::catalog::plugin_catalog_fetch,
            commands::catalog::plugin_catalog_sources_get,
            commands::catalog::plugin_catalog_sources_set,
            commands::plugin_window::plugin_open_window,
            commands::plugin_window::plugin_center_open_window,
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
        .run(context)
        .expect("error while running MyTools");
}

#[cfg(test)]
mod architecture_tests;
