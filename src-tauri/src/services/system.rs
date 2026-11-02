//! System helpers exposed to the frontend / plugins.
//!
//! `system_get_path` resolves the well known directories (`home`,
//! `appData`, `config`, `temp`, `desktop`) that plugins ask for through
//! the `system.path` capability.

use tauri::{AppHandle, Manager, Runtime};

/// Mapping from the well-known path names to a Tauri path resolver
/// method name. Kept pure so it is unit-testable without a running
/// Tauri app.
fn resolve_path_kind(name: &str) -> Result<&'static str, String> {
    match name {
        "home" => Ok("home"),
        "appData" => Ok("appData"),
        "config" => Ok("config"),
        "temp" => Ok("temp"),
        "desktop" => Ok("desktop"),
        other => Err(format!("unknown path name: {other}")),
    }
}

pub fn get_path<R: Runtime>(app: &AppHandle<R>, name: &str) -> Result<String, String> {
    let kind = resolve_path_kind(name)?;
    let dir = match kind {
        "home" => app.path().home_dir(),
        "appData" => Ok(crate::infrastructure::paths::data_root(app)),
        "config" => Ok(crate::infrastructure::paths::config_root(app)),
        "temp" => app.path().temp_dir(),
        "desktop" => app.path().desktop_dir(),
        _ => unreachable!("validated by resolve_path_kind"),
    };
    dir.map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

/// Returns true if the window label is one of the runtime-managed
/// plugin webview windows (i.e. `plugin-*`). Used by the plugin window
/// lifecycle commands to gate access.
#[cfg(test)]
mod tests {
    use super::resolve_path_kind;

    #[test]
    fn accepts_supported_path_names() {
        for name in ["home", "appData", "config", "temp", "desktop"] {
            assert!(resolve_path_kind(name).is_ok(), "{name} should resolve");
        }
    }

    #[test]
    fn rejects_unknown_or_malformed_names() {
        for name in ["", "home ", "HOMEDIR", "downloads", "appdata", ".."] {
            assert!(
                resolve_path_kind(name).is_err(),
                "{name:?} must be rejected"
            );
        }
    }
}
