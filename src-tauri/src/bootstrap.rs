use crate::commands;
use crate::services::settings::SettingsState;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{Listener, Manager};

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let (config_root, data_root) = initialize_storage(app);
    tracing::info!("MyTools starting up");
    crate::services::apps::index::warm();
    initialize_clipboard(app, &data_root);
    let settings = initialize_plugins(app, &config_root, &data_root);
    initialize_capabilities(app, &config_root)?;
    initialize_health(app);
    initialize_desktop(app, &settings);
    initialize_file_drop(app.handle());
    if let Err(error) = commands::panel::initialize(app.handle(), settings.get().middle_panel_enabled) {
        tracing::warn!("middle panel unavailable: {error}");
    }
    Ok(())
}

fn initialize_storage(app: &tauri::App) -> (PathBuf, PathBuf) {
    let config_root = crate::infrastructure::paths::config_root(app.handle());
    let data_root = crate::infrastructure::paths::data_root(app.handle());
    if let Err(e) = std::fs::create_dir_all(&config_root) {
        tracing::warn!("failed to create config root: {}", e);
    }
    if let Err(e) = std::fs::create_dir_all(&data_root) {
        tracing::warn!("failed to create data root: {}", e);
    }
    if let Some(guard) = crate::infrastructure::logging::init(&data_root) {
        app.manage(guard);
    }
    let legacy_dir = crate::infrastructure::paths::legacy_config_dir(app.handle());
    if let Err(e) = commands::migration::run(&legacy_dir, &config_root, &data_root) {
        tracing::warn!("legacy layout migration deferred (will retry): {}", e);
    }
    if let Err(e) = commands::migration::run_pivot(&config_root, &data_root) {
        tracing::warn!(
            "official-tools pivot migration deferred (will retry): {}",
            e
        );
    }

    (config_root, data_root)
}

fn initialize_clipboard(app: &tauri::App, data_root: &Path) {
    let db_path = data_root.join("mytools.db");
    let store = match crate::services::clipboard::open_store(db_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("clipboard store open failed: {}", e);
            crate::services::clipboard::open_in_memory_store().expect("in-memory clipboard store")
        }
    };
    crate::infrastructure::clipboard_watcher::spawn(app.handle().clone(), Arc::clone(&store));
    app.manage(store);
    tracing::info!("clipboard store ready");
}

fn initialize_plugins(app: &tauri::App, config_root: &Path, data_root: &Path) -> SettingsState {
    let settings_path = crate::services::settings::settings_path(config_root);
    let settings_state = crate::services::settings::SettingsState::load(settings_path);
    let plugin_registry = Arc::new(crate::services::plugins::PluginRegistry::new(
        crate::services::plugins::plugins_state_path(config_root),
    ));
    let plugins_root =
        crate::services::plugins::effective_plugins_root(&settings_state.get(), data_root);
    if let Err(e) = std::fs::create_dir_all(&plugins_root) {
        tracing::warn!("failed to create plugins dir: {}", e);
    }
    match plugin_registry.reload(&plugins_root) {
        Ok(n) => tracing::info!("loaded {} external plugin(s)", n),
        Err(e) => tracing::warn!("plugin scan failed: {}", e),
    }

    let watcher_tx = commands::plugin_watcher::spawn(
        app.handle().clone(),
        Arc::clone(&plugin_registry),
        plugins_root,
    );
    plugin_registry.install_watcher(watcher_tx);
    app.manage(Arc::clone(&plugin_registry));
    let plugin_server = commands::plugin_server::PluginServer::start(Arc::clone(&plugin_registry));
    if plugin_server.base().is_none() {
        tracing::warn!(
            "plugin server unavailable; plugin pages fall back to file:// and may be blocked"
        );
    }
    app.manage(plugin_server);
    app.manage(settings_state.clone());
    settings_state
}

fn initialize_capabilities(
    app: &tauri::App,
    config_root: &Path,
) -> Result<(), mytools_core::capability::CapabilityError> {
    let grant_store = Arc::new(mytools_core::permission::GrantStore::load(
        config_root.join(mytools_core::permission::PERMISSION_FILE),
    ));
    let dir_grants = Arc::new(mytools_core::dir_grants::DirGrantStore::load(
        config_root.join(mytools_core::dir_grants::DIR_GRANTS_FILE),
    ));
    let capability_registry = Arc::new(crate::services::capabilities::build_registry(
        app.handle(),
        Arc::clone(&grant_store),
        Arc::clone(&dir_grants),
    )?);
    tracing::info!(
        "capability registry ready ({} capabilities)",
        capability_registry.ids().len()
    );
    app.manage(Arc::clone(&capability_registry));
    app.manage(Arc::clone(&grant_store));
    app.manage(Arc::clone(&dir_grants));
    Ok(())
}

fn initialize_health(app: &tauri::App) {
    let my_pid = std::process::id();
    let health = Arc::new(commands::health::WebviewHealth::new(my_pid));
    health.sample();
    app.manage(Arc::clone(&health));
    tauri::async_runtime::spawn(async move {
        commands::health::sample_periodically(health, std::time::Duration::from_secs(2)).await;
    });
    tracing::info!("webview health monitor started (pid={})", my_pid);
}

fn initialize_desktop(app: &tauri::App, settings: &SettingsState) {
    if let Some(window) = app.get_webview_window("main") {
        let saved = settings.get();
        match (
            saved.main_x,
            saved.main_y,
            saved.main_width,
            saved.main_height,
        ) {
            (Some(x), Some(y), Some(w), Some(h)) => {
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
                let _ = window.set_size(tauri::PhysicalSize::new(w, h));
                tracing::info!("main window restored to ({x}, {y}) {w}x{h}");
            }
            _ => {
                let _ = commands::window::position_on_active_monitor(&window, app.handle(), None);
                tracing::info!("main window created and positioned");
            }
        }
    }

    let hotkey = settings.get().hotkey;
    if let Err(e) = commands::hotkey::install(app.handle(), &hotkey) {
        tracing::warn!("failed to register shortcut '{hotkey}': {}", e);
    } else {
        tracing::info!("shortcut '{hotkey}' registered");
    }

    if settings.get().tray_enabled {
        if let Err(e) = commands::tray::ensure_tray(app.handle()) {
            tracing::warn!("failed to create tray icon: {e}");
        } else {
            tracing::info!("system tray created");
        }
    } else {
        tracing::info!("system tray disabled by settings");
    }
    let app_handle_for_tray = app.handle().clone();
    let listener_handle = app_handle_for_tray.clone();
    listener_handle.listen("settings:tray", move |event| {
        let payload = event.payload();
        let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
            return;
        };
        let on = value.as_bool().unwrap_or(false);
        let handle = app_handle_for_tray.clone();
        if on {
            if let Err(e) = commands::tray::ensure_tray(&handle) {
                tracing::warn!("failed to create tray on settings change: {e}");
            }
        } else {
            commands::tray::remove_tray(&handle);
        }
    });

    let floatball_settings = settings.clone();
    if floatball_settings.get().floatball_enabled {
        if let Err(e) = commands::floatball::ensure_floatball(app.handle(), &floatball_settings) {
            tracing::warn!("failed to create floatball: {e}");
        } else {
            tracing::info!("floatball created");
        }
    } else {
        tracing::info!("floatball disabled by settings");
    }
}

fn initialize_file_drop(app: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        use tauri::{Emitter, Manager};
        let app=app.clone();
        tauri::async_runtime::spawn(async move {
            let mut timer=tokio::time::interval(std::time::Duration::from_millis(750));
            loop {
                timer.tick().await;
                let handle=app.clone();
                if app.run_on_main_thread(move || {
                    for (label,window) in handle.webview_windows() {
                        if label!="main" && !label.starts_with("plugin-"){continue;}
                        let app=handle.clone();
                        crate::infrastructure::file_drop::refresh(&window,std::sync::Arc::new(move|paths| {
                            tracing::info!(window=%label,count=paths.len(),"native OLE file drop");
                            if app.state::<crate::runtime::DroppedFiles>().record(&label,paths) {
                                let _=app.emit(&format!("mytools:files-dropped:{label}"),());
                            }
                        }));
                    }
                }).is_err(){break;}
            }
        });
    }
    #[cfg(not(windows))]
    let _=app;
}
