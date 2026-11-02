//! Product data/config roots ("Layout B").
//!
//! This is the single source of truth for where MyTools keeps its files:
//!
//! | kind      | location                        | contents                                     |
//! |-----------|---------------------------------|----------------------------------------------|
//! | installed | `%LOCALAPPDATA%\MyTools\`       | `mytools.exe`, `WebView2Loader.dll` (NSIS)   |
//! | data      | `%LOCALAPPDATA%\MyTools\data\`  | `plugins/`, `mytools.db`, `plugin-files/`, `logs/` |
//! | config    | `%APPDATA%\MyTools\`            | `settings.json`, `permissions.json`, `plugin-state.json`, `plugin-origins.json`, `plugin-store.json` |
//!
//! The directories are derived from the Tauri identifier dirs
//! (`…\com.mytools.app`) by stepping one level up and joining the
//! product name, so they stay stable regardless of install location.
//! `%LOCALAPPDATA%\com.mytools.app` is the WebView2 runtime profile and
//! is deliberately left alone — see `docs/05-data-and-storage.md`.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

pub const PRODUCT_DIR: &str = "MyTools";
pub const DATA_SUBDIR: &str = "data";

/// Hand-editable config root:
/// `%APPDATA%\MyTools`.
pub fn config_root<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_config_dir()
        .or_else(|_| app.path().app_data_dir())
        .map(|dir| config_root_from(&dir))
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Machine-local data root:
/// `%LOCALAPPDATA%\MyTools\data`.
pub fn data_root<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_local_data_dir()
        .map(|dir| data_root_from(&dir))
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// The legacy directory everything used to live in:
/// `%APPDATA%\com.mytools.app`. Kept only for the one-time migration.
pub fn legacy_config_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// `%APPDATA%\MyTools` from the identifier dir `…\com.mytools.app`.
pub fn config_root_from(identifier_dir: &Path) -> PathBuf {
    parent_of(identifier_dir).join(PRODUCT_DIR)
}

/// `%LOCALAPPDATA%\MyTools\data` from the identifier dir `…\com.mytools.app`.
pub fn data_root_from(identifier_dir: &Path) -> PathBuf {
    parent_of(identifier_dir)
        .join(PRODUCT_DIR)
        .join(DATA_SUBDIR)
}

fn parent_of(dir: &Path) -> PathBuf {
    match dir.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => dir.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_derive_from_identifier_dir() {
        let id = Path::new("C:/Users/u/AppData/Roaming/com.mytools.app");
        assert_eq!(
            config_root_from(id),
            PathBuf::from("C:/Users/u/AppData/Roaming/MyTools")
        );
        let local = Path::new("C:/Users/u/AppData/Local/com.mytools.app");
        assert_eq!(
            data_root_from(local),
            PathBuf::from("C:/Users/u/AppData/Local/MyTools/data")
        );
    }

    #[test]
    fn identifier_dir_without_parent_falls_back_to_itself() {
        let id = Path::new("com.mytools.app");
        assert_eq!(
            config_root_from(id),
            PathBuf::from("com.mytools.app/MyTools")
        );
        assert_eq!(
            data_root_from(id),
            PathBuf::from("com.mytools.app/MyTools/data")
        );
    }
}
