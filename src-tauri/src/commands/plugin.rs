use crate::commands::plugin_server::{serve_url, PluginServer};
use crate::services::plugins::{effective_plugins_root, SharedPluginRegistry};
use crate::services::settings::SettingsState;
use mytools_core::plugin_loader::LoadedPlugin;
use std::path::{Path, PathBuf};

fn data_root(app: &tauri::AppHandle) -> PathBuf {
    crate::infrastructure::paths::data_root(app)
}

fn open_in_explorer(app: &tauri::AppHandle, dir: &Path) -> Result<(), String> {
    use tauri_plugin_shell::process::CommandEvent;
    use tauri_plugin_shell::ShellExt;

    let dir_arg = dir.to_string_lossy().into_owned();
    let mut cmd = app.shell().command("explorer");
    cmd = cmd.args([dir_arg]);
    let (mut rx, _child) = cmd.spawn().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let CommandEvent::Terminated(_) | CommandEvent::Error(_) = event {
                break;
            }
        }
    });
    Ok(())
}

pub fn notify_plugins_changed(app: &tauri::AppHandle) {
    use tauri::Emitter;
    let _ = app.emit("plugin:changed", ());
}

#[tauri::command]
pub async fn plugin_list(
    registry: tauri::State<'_, SharedPluginRegistry>,
    server: tauri::State<'_, PluginServer>,
) -> Result<Vec<LoadedPlugin>, String> {
    let base = match server.base() {
        Some(b) => b.to_string(),
        None => return Ok(registry.list()),
    };

    let user_root = registry.root();
    Ok(registry
        .list()
        .into_iter()
        .map(|mut plugin| {
            if let Some(root) = user_root.as_ref() {
                plugin.main_url = serve_url(&base, root, &plugin.dir, &plugin.manifest.main);
                if let Some(preload) = plugin.manifest.preload.clone() {
                    plugin.preload_url = Some(serve_url(&base, root, &plugin.dir, &preload));
                }
            }
            plugin
        })
        .collect())
}

#[tauri::command]
pub async fn plugin_reload(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedPluginRegistry>,
    settings: tauri::State<'_, SettingsState>,
    root: Option<String>,
) -> Result<usize, String> {
    let path = match root {
        Some(r) if !r.trim().is_empty() => PathBuf::from(r),
        _ => effective_plugins_root(&settings.get(), &data_root(&app)),
    };
    let n = registry.set_root(&path)?;
    notify_plugins_changed(&app);
    Ok(n)
}

#[tauri::command]
pub async fn plugin_get_root(
    app: tauri::AppHandle,
    settings: tauri::State<'_, SettingsState>,
) -> Result<String, String> {
    Ok(effective_plugins_root(&settings.get(), &data_root(&app))
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
pub async fn plugin_set_root(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedPluginRegistry>,
    settings: tauri::State<'_, SettingsState>,
    root: String,
) -> Result<String, String> {
    let trimmed = root.trim().to_string();
    if trimmed.is_empty() {
        return Err("插件目录不能为空".into());
    }
    let path = PathBuf::from(&trimmed);
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    registry.set_root(&path)?;
    settings.set_plugin_root(Some(trimmed))?;
    notify_plugins_changed(&app);
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn plugin_reset_root(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedPluginRegistry>,
    settings: tauri::State<'_, SettingsState>,
) -> Result<String, String> {
    settings.set_plugin_root(None)?;
    let path = effective_plugins_root(&settings.get(), &data_root(&app));
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    registry.set_root(&path)?;
    notify_plugins_changed(&app);
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn plugin_open_root(
    app: tauri::AppHandle,
    settings: tauri::State<'_, SettingsState>,
) -> Result<(), String> {
    let path = effective_plugins_root(&settings.get(), &data_root(&app));
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    open_in_explorer(&app, &path)
}

#[tauri::command]
pub async fn plugin_enable(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedPluginRegistry>,
    code: String,
) -> Result<(), String> {
    registry.set_enabled(&code, true)?;
    notify_plugins_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn plugin_disable(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedPluginRegistry>,
    code: String,
) -> Result<(), String> {
    registry.set_enabled(&code, false)?;
    notify_plugins_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn plugin_uninstall(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedPluginRegistry>,
    code: String,
) -> Result<(), String> {
    registry.uninstall(&code)?;
    notify_plugins_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn plugin_install_dir(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedPluginRegistry>,
    dir: String,
) -> Result<String, String> {
    let name = registry.install_dir(Path::new(&dir))?;
    notify_plugins_changed(&app);
    Ok(name)
}

#[tauri::command]
pub async fn plugin_install_pack(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedPluginRegistry>,
    pack: String,
) -> Result<String, String> {
    let name = registry.install_pack(Path::new(&pack))?;
    notify_plugins_changed(&app);
    Ok(name)
}

#[tauri::command]
pub async fn plugin_open_dir(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedPluginRegistry>,
    code: String,
) -> Result<(), String> {
    let dir = registry
        .open_dir(&code)
        .ok_or_else(|| format!("plugin not found: {}", code))?;
    open_in_explorer(&app, &dir)
}
