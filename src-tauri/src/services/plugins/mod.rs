use crate::services::settings::AppSettings;
#[cfg(test)]
use mytools_core::capability::Origin;
use mytools_core::plugin_loader::{scan_dir_enabled, LoadedPlugin};
use mytools_core::plugin_mgmt::{
    install_from_dir, install_from_tar_gz, install_from_zip, uninstall_plugin, PluginOrigins,
    PluginState,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

struct Inner {
    plugins: Vec<LoadedPlugin>,
    state: PluginState,
    origins: PluginOrigins,
    root: Option<PathBuf>,
}

pub struct PluginRegistry {
    inner: RwLock<Inner>,
    state_path: PathBuf,
    origins_path: PathBuf,
    watcher_tx: RwLock<Option<Sender<PathBuf>>>,
}

impl PluginRegistry {
    pub fn new(state_path: PathBuf) -> Self {
        let state = PluginState::load(&state_path).unwrap_or_default();
        let origins_path =
            state_path.with_file_name(mytools_core::plugin_mgmt::PLUGIN_ORIGINS_FILE);
        let origins = PluginOrigins::load(&origins_path).unwrap_or_default();
        Self {
            inner: RwLock::new(Inner {
                plugins: Vec::new(),
                state,
                origins,
                root: None,
            }),
            state_path,
            origins_path,
            watcher_tx: RwLock::new(None),
        }
    }

    pub fn reload(&self, root: &Path) -> Result<usize, String> {
        let mut inner = self.inner.write().expect("plugin registry write");
        let mut loaded = scan_dir_enabled(root, &|name| inner.state.is_enabled(name))
            .map_err(|e| e.to_string())?;
        for plugin in &mut loaded {
            if let Some(origin) = inner.origins.origin_of(&plugin.manifest.name) {
                plugin.origin = plugin.manifest.effective_origin(origin);
            }
        }
        let count = loaded.len();
        inner.plugins = loaded;
        inner.root = Some(root.to_path_buf());
        Ok(count)
    }

    #[cfg(test)]
    pub fn mark_official(&self, name: &str) -> Result<(), String> {
        self.mark_origin(name, Origin::Official)
    }

    #[cfg(test)]
    fn mark_origin(&self, name: &str, origin: Origin) -> Result<(), String> {
        let mut inner = self.inner.write().expect("plugin registry write");
        match origin {
            Origin::Official => {
                inner.origins.mark_official(name);
            }
            Origin::Custom => {
                inner.origins.forget(name);
            }
            Origin::Ecosystem => {
                inner.origins.forget(name);
            }
        }
        inner
            .origins
            .save(&self.origins_path)
            .map_err(|e| e.to_string())?;
        if let Some(plugin) = inner
            .plugins
            .iter_mut()
            .find(|p| p.key() == name || p.manifest.name == name)
        {
            plugin.origin = plugin.manifest.effective_origin(origin);
        }
        Ok(())
    }

    pub fn set_root(&self, root: &Path) -> Result<usize, String> {
        let count = self.reload(root)?;
        if let Some(tx) = self.watcher_tx.read().expect("watcher read").as_ref() {
            let _ = tx.send(root.to_path_buf());
        }
        Ok(count)
    }

    pub fn install_watcher(&self, tx: Sender<PathBuf>) {
        *self.watcher_tx.write().expect("watcher write") = Some(tx);
    }

    pub fn list(&self) -> Vec<LoadedPlugin> {
        self.inner
            .read()
            .expect("plugin registry read")
            .plugins
            .clone()
    }

    pub fn root(&self) -> Option<PathBuf> {
        self.inner
            .read()
            .expect("plugin registry read")
            .root
            .clone()
    }

    fn dir_of(&self, name: &str) -> Option<PathBuf> {
        self.inner
            .read()
            .expect("plugin registry read")
            .plugins
            .iter()
            .find(|p| p.key() == name)
            .map(|p| PathBuf::from(&p.dir))
    }

    fn rescan(&self) -> Result<(), String> {
        let root = self
            .root()
            .ok_or_else(|| "plugins root not set".to_string())?;
        self.reload(&root).map(|_| ())
    }

    pub fn set_enabled(&self, name: &str, on: bool) -> Result<bool, String> {
        {
            let mut inner = self.inner.write().expect("plugin registry write");
            inner.state.set_enabled(name, on);
            if let Some(p) = inner.plugins.iter_mut().find(|p| p.key() == name) {
                p.enabled = on;
            }
        }
        self.save_state()?;
        self.rescan()?;
        Ok(on)
    }

    fn save_state(&self) -> Result<(), String> {
        let state = self
            .inner
            .read()
            .expect("plugin registry read")
            .state
            .clone();
        state.save(&self.state_path).map_err(|e| e.to_string())
    }

    pub fn uninstall(&self, name: &str) -> Result<(), String> {
        let root = self
            .root()
            .ok_or_else(|| "plugins root not set".to_string())?;
        uninstall_plugin(&root, name).map_err(|e| e.to_string())?;
        {
            let mut inner = self.inner.write().expect("plugin registry write");
            if inner.origins.forget(name) {
                inner
                    .origins
                    .save(&self.origins_path)
                    .map_err(|e| e.to_string())?;
            }
        }
        self.rescan()
    }

    pub fn install_dir(&self, src: &Path) -> Result<String, String> {
        let root = self
            .root()
            .ok_or_else(|| "plugins root not set".to_string())?;
        let name = install_from_dir(src, &root).map_err(|e| e.to_string())?;
        self.rescan().map(|_| name)
    }

    pub fn install_pack(&self, pack: &Path) -> Result<String, String> {
        let root = self
            .root()
            .ok_or_else(|| "plugins root not set".to_string())?;
        let ext = pack
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        let name = match ext.as_str() {
            "tgz" | "gz" => install_from_tar_gz(pack, &root).map_err(|e| e.to_string())?,
            "zip" => install_from_zip(pack, &root).map_err(|e| e.to_string())?,
            _ => return Err(format!("unsupported archive type: .{}", ext)),
        };
        self.rescan().map(|_| name)
    }

    pub fn open_dir(&self, name: &str) -> Option<PathBuf> {
        self.dir_of(name)
    }
}

pub type SharedPluginRegistry = Arc<PluginRegistry>;

pub fn effective_plugins_root(settings: &AppSettings, data_root: &Path) -> PathBuf {
    if let Some(dir) = std::env::var_os("MYTOOLS_PLUGINS_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(root) = settings.plugin_root.as_deref() {
        let root = root.trim();
        if !root.is_empty() {
            return PathBuf::from(root);
        }
    }
    data_root.join("plugins")
}

pub fn plugins_state_path(config_root: &Path) -> PathBuf {
    config_root.join(mytools_core::plugin_mgmt::PLUGIN_STATE_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn official_provenance_survives_a_reload() {
        let dir = TempDir::new().unwrap();
        let plugins_root = dir.path().join("plugins");
        let plugin_dir = plugins_root.join("mytools-hello");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name":"mytools-hello","version":"1.0.0","main":"index.html"}"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();
        let state_path = dir.path().join("state").join("plugin-state.json");

        let registry = PluginRegistry::new(state_path.clone());
        registry.reload(&plugins_root).unwrap();
        assert_eq!(registry.list()[0].origin, Origin::Custom);

        registry.mark_official("mytools-hello").unwrap();
        assert_eq!(registry.list()[0].origin, Origin::Official);

        let reopened = PluginRegistry::new(state_path);
        reopened.reload(&plugins_root).unwrap();
        assert_eq!(reopened.list()[0].origin, Origin::Official);
    }

    #[test]
    fn uninstall_forgets_official_provenance() {
        let dir = TempDir::new().unwrap();
        let plugins_root = dir.path().join("plugins");
        let plugin_dir = plugins_root.join("mytools-json");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name":"mytools-json","version":"1.0.0","main":"index.html"}"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();
        let state_path = dir.path().join("state").join("plugin-state.json");

        let registry = PluginRegistry::new(state_path.clone());
        registry.reload(&plugins_root).unwrap();
        registry.mark_official("mytools-json").unwrap();
        registry.uninstall("mytools-json").unwrap();
        assert!(registry.list().is_empty());

        let origins =
            PluginOrigins::load(&state_path.with_file_name("plugin-origins.json")).unwrap();
        assert!(
            origins.origin_of("mytools-json").is_none(),
            "uninstall clears the provenance"
        );
    }
}
