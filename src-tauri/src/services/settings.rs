//! Persistent application settings in the config root
//! (`%APPDATA%\MyTools\settings.json`, see `commands/paths.rs`).
//!
//! Kept as a small JSON document (not the KV store plugin) so the
//! plugin root survives restarts and can be edited by hand.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

pub const SETTINGS_FILE: &str = "settings.json";

fn default_true() -> bool {
    true
}

fn default_detached_width() -> u32 {
    1024
}

fn default_detached_height() -> u32 {
    720
}

fn default_hotkey() -> String {
    "Alt+Space".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// Custom plugin directory. `None` means "use the platform default".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_root: Option<String>,
    #[serde(default = "crate::services::catalog::default_sources")]
    pub catalog_sources: crate::services::catalog::CatalogSources,
    #[serde(default = "default_true")]
    pub tray_enabled: bool,
    #[serde(default)]
    pub floatball_enabled: bool,
    #[serde(default = "default_true")]
    pub middle_panel_enabled: bool,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default)]
    pub pinned_plugins: Vec<String>,
    #[serde(default)]
    pub recent_plugins: Vec<String>,
    /// Default size of plugin webview windows opened via the launcher
    /// "detach to new window" button.
    #[serde(default = "default_detached_width")]
    pub detached_window_width: u32,
    #[serde(default = "default_detached_height")]
    pub detached_window_height: u32,
    /// Persisted floatball window position (logical px). `None` means
    /// "place at the default bottom-right corner on first run".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floatball_x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floatball_y: Option<i32>,
    /// Persisted launcher window bounds (physical px). `None` means
    /// "position at the default panel location on first run".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_y: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_height: Option<u32>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            plugin_root: None,
            catalog_sources: crate::services::catalog::default_sources(),
            tray_enabled: true,
            floatball_enabled: false,
            middle_panel_enabled: true,
            theme: Theme::default(),
            hotkey: default_hotkey(),
            pinned_plugins: Vec::new(),
            recent_plugins: Vec::new(),
            detached_window_width: default_detached_width(),
            detached_window_height: default_detached_height(),
            floatball_x: None,
            floatball_y: None,
            main_x: None,
            main_y: None,
            main_width: None,
            main_height: None,
        }
    }
}

pub fn settings_path(config_root: &Path) -> PathBuf {
    config_root.join(SETTINGS_FILE)
}

/// In-memory settings with JSON persistence, shared as Tauri state.
pub struct SettingsState {
    path: PathBuf,
    inner: RwLock<AppSettings>,
}

impl Clone for SettingsState {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            inner: RwLock::new(self.get()),
        }
    }
}

impl SettingsState {
    pub fn set_catalog_sources(&self, sources: crate::services::catalog::CatalogSources) -> Result<(), String> {
        sources.validate()?;
        self.inner.write().map_err(|e| e.to_string())?.catalog_sources = sources;
        self.save()
    }
    pub fn set_middle_panel_enabled(&self, enabled: bool) -> Result<(), String> {
        let mut settings = self.inner.write().map_err(|e| e.to_string())?;
        settings.middle_panel_enabled = enabled;
        drop(settings);
        self.save()
    }
    pub fn load(path: PathBuf) -> Self {
        // Unknown persisted fields (e.g. the retired plugin store source
        // and display mode) are ignored on read and dropped on next save.
        let settings = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<AppSettings>(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            inner: RwLock::new(settings),
        }
    }

    pub fn get(&self) -> AppSettings {
        self.inner.read().expect("settings read").clone()
    }

    pub fn set_plugin_root(&self, root: Option<String>) -> Result<(), String> {
        {
            let mut inner = self.inner.write().expect("settings write");
            inner.plugin_root = root;
        }
        self.save()
    }

    pub fn set_tray_enabled(&self, on: bool) -> Result<(), String> {
        {
            let mut inner = self.inner.write().expect("settings write");
            inner.tray_enabled = on;
        }
        self.save()
    }

    pub fn set_floatball_enabled(&self, on: bool) -> Result<(), String> {
        {
            let mut inner = self.inner.write().expect("settings write");
            inner.floatball_enabled = on;
        }
        self.save()
    }

    pub fn set_theme(&self, theme: Theme) -> Result<(), String> {
        {
            let mut inner = self.inner.write().expect("settings write");
            inner.theme = theme;
        }
        self.save()
    }

    pub fn set_hotkey(&self, hotkey: String) -> Result<(), String> {
        let trimmed = hotkey.trim().to_string();
        if trimmed.is_empty() {
            return Err("hotkey cannot be empty".into());
        }
        {
            let mut inner = self.inner.write().expect("settings write");
            inner.hotkey = trimmed;
        }
        self.save()
    }

    pub fn set_usage(&self, pinned: Vec<String>, recent: Vec<String>) -> Result<(), String> {
        {
            let mut inner = self.inner.write().expect("settings write");
            inner.pinned_plugins = pinned;
            inner.recent_plugins = recent;
        }
        self.save()
    }

    pub fn set_floatball_position(&self, x: i32, y: i32) -> Result<(), String> {
        {
            let mut inner = self.inner.write().expect("settings write");
            inner.floatball_x = Some(x);
            inner.floatball_y = Some(y);
        }
        self.save()
    }

    /// Persist the launcher window's physical bounds. Width/height are
    /// expected to be positive (validated by the caller / window min).
    pub fn set_main_bounds(&self, x: i32, y: i32, width: u32, height: u32) -> Result<(), String> {
        {
            let mut inner = self.inner.write().expect("settings write");
            inner.main_x = Some(x);
            inner.main_y = Some(y);
            inner.main_width = Some(width);
            inner.main_height = Some(height);
        }
        self.save()
    }

    #[allow(dead_code)]
    pub fn set_detached_window_size(&self, width: u32, height: u32) -> Result<(), String> {
        if width < 320 || height < 240 {
            return Err("detached window size must be at least 320x240".into());
        }
        {
            let mut inner = self.inner.write().expect("settings write");
            inner.detached_window_width = width;
            inner.detached_window_height = height;
        }
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        let settings = self.get();
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, bytes).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_include_new_fields() {
        let s = AppSettings::default();
        assert!(s.tray_enabled);
        assert!(!s.floatball_enabled);
        assert!(s.plugin_root.is_none());
        assert_eq!(s.theme, Theme::System);
        assert_eq!(s.hotkey, "Alt+Space");
        assert!(s.pinned_plugins.is_empty());
        assert!(s.recent_plugins.is_empty());
        assert_eq!(s.detached_window_width, 1024);
        assert_eq!(s.detached_window_height, 720);
        assert!(s.floatball_x.is_none());
        assert!(s.floatball_y.is_none());
        assert!(s.main_x.is_none());
        assert!(s.main_y.is_none());
        assert!(s.main_width.is_none());
        assert!(s.main_height.is_none());
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let s: AppSettings = serde_json::from_str(r#"{"pluginRoot":"D:/plug"}"#).unwrap();
        assert_eq!(s.plugin_root.as_deref(), Some("D:/plug"));
        assert!(s.tray_enabled);
        assert!(!s.floatball_enabled);
        assert_eq!(s.theme, Theme::System);
        assert_eq!(s.hotkey, "Alt+Space");
        assert_eq!(s.detached_window_width, 1024);
        assert_eq!(s.detached_window_height, 720);
    }

    #[test]
    fn plugin_root_roundtrips_through_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = settings_path(dir.path());
        let state = SettingsState::load(path.clone());
        assert!(state.get().plugin_root.is_none());

        state
            .set_plugin_root(Some("D:/my plugins".to_string()))
            .unwrap();
        let reloaded = SettingsState::load(path);
        assert_eq!(reloaded.get().plugin_root.as_deref(), Some("D:/my plugins"));

        reloaded.set_plugin_root(None).unwrap();
        assert!(SettingsState::load(settings_path(dir.path()))
            .get()
            .plugin_root
            .is_none());
    }

    #[test]
    fn tray_and_floatball_setters_round_trip() {
        let dir = tempfile::TempDir::new().unwrap();
        let state = SettingsState::load(settings_path(dir.path()));
        assert!(state.get().tray_enabled);
        assert!(!state.get().floatball_enabled);

        state.set_tray_enabled(false).unwrap();
        state.set_floatball_enabled(true).unwrap();

        let reloaded = SettingsState::load(settings_path(dir.path()));
        assert!(!reloaded.get().tray_enabled);
        assert!(reloaded.get().floatball_enabled);
    }

    #[test]
    fn preferences_and_usage_round_trip() {
        let dir = tempfile::TempDir::new().unwrap();
        let state = SettingsState::load(settings_path(dir.path()));
        state.set_theme(Theme::Dark).unwrap();
        state.set_hotkey("Ctrl+Shift+1".into()).unwrap();
        state
            .set_usage(
                vec!["app:C:\\Tools\\x.exe".into()],
                vec!["mytools-json".into()],
            )
            .unwrap();

        let reloaded = SettingsState::load(settings_path(dir.path())).get();
        assert_eq!(reloaded.theme, Theme::Dark);
        assert_eq!(reloaded.hotkey, "Ctrl+Shift+1");
        assert_eq!(reloaded.pinned_plugins, vec!["app:C:\\Tools\\x.exe"]);
        assert_eq!(reloaded.recent_plugins, vec!["mytools-json"]);
    }

    #[test]
    fn retired_store_and_display_mode_fields_are_ignored() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = settings_path(dir.path());
        std::fs::write(
            &path,
            r#"{"pluginStoreSource":"https://gitee.com/old/store.json","pluginDisplayMode":"incompatible","theme":"dark"}"#,
        )
        .unwrap();

        let state = SettingsState::load(path.clone());
        assert_eq!(state.get().theme, Theme::Dark);
        state.set_tray_enabled(true).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("pluginStoreSource"));
        assert!(!raw.contains("pluginDisplayMode"));
    }

    #[test]
    fn empty_hotkey_is_rejected() {
        let dir = tempfile::TempDir::new().unwrap();
        let state = SettingsState::load(settings_path(dir.path()));
        assert!(state.set_hotkey("   ".into()).is_err());
    }

    #[test]
    fn floatball_position_roundtrips_through_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let state = SettingsState::load(settings_path(dir.path()));
        assert!(state.get().floatball_x.is_none());
        assert!(state.get().floatball_y.is_none());

        state.set_floatball_position(1280, 720).unwrap();

        let reloaded = SettingsState::load(settings_path(dir.path()));
        assert_eq!(reloaded.get().floatball_x, Some(1280));
        assert_eq!(reloaded.get().floatball_y, Some(720));
    }

    #[test]
    fn main_bounds_roundtrips_through_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let state = SettingsState::load(settings_path(dir.path()));
        assert!(state.get().main_x.is_none());

        state.set_main_bounds(200, 150, 900, 600).unwrap();

        let reloaded = SettingsState::load(settings_path(dir.path()));
        assert_eq!(reloaded.get().main_x, Some(200));
        assert_eq!(reloaded.get().main_y, Some(150));
        assert_eq!(reloaded.get().main_width, Some(900));
        assert_eq!(reloaded.get().main_height, Some(600));
    }

    #[test]
    fn detached_window_size_validates_minimum() {
        let dir = tempfile::TempDir::new().unwrap();
        let state = SettingsState::load(settings_path(dir.path()));
        assert!(state.set_detached_window_size(100, 100).is_err());
        state.set_detached_window_size(640, 480).unwrap();
        assert_eq!(state.get().detached_window_width, 640);
        assert_eq!(state.get().detached_window_height, 480);
    }
}
