use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct PluginSessions {
    bindings: std::sync::Mutex<HashMap<String, (String, String)>>,
    updating: std::sync::Mutex<HashSet<String>>,
    sequence: std::sync::atomic::AtomicU64,
}

impl PluginSessions {
    pub fn open(&self, window: &str, plugin: &str) -> Result<String, String> {
        let updating = self.updating.lock().map_err(|e| e.to_string())?;
        if updating.contains(plugin) {
            return Err("插件正在更新，请稍后打开".into());
        }
        let token = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_string();
        self.bindings
            .lock()
            .map_err(|e| e.to_string())?
            .insert(window.to_string(), (token.clone(), plugin.to_string()));
        Ok(token)
    }

    pub fn begin_update(&self, plugin: &str) -> Result<PluginUpdateLease<'_>, String> {
        let mut updating = self.updating.lock().map_err(|e| e.to_string())?;
        if updating.contains(plugin) {
            return Err("插件已经在更新中".into());
        }
        let bindings = self.bindings.lock().map_err(|e| e.to_string())?;
        if bindings.values().any(|(_, name)| name == plugin) {
            return Err("插件正在使用，请先保存内容并关闭插件，再重试更新".into());
        }
        updating.insert(plugin.to_string());
        Ok(PluginUpdateLease {
            sessions: self,
            plugin: plugin.to_string(),
        })
    }

    pub fn forget(&self, window: &str) {
        if let Ok(mut bindings) = self.bindings.lock() {
            bindings.remove(window);
        }
    }

    pub fn close(&self, window: &str, token: &str) {
        if let Ok(mut bindings) = self.bindings.lock() {
            if bindings
                .get(window)
                .is_some_and(|binding| binding.0 == token)
            {
                bindings.remove(window);
            }
        }
    }

    pub fn active_windows(&self) -> Vec<(String, String)> {
        self.bindings
            .lock()
            .map(|bindings| {
                bindings
                    .iter()
                    .map(|(window, (_, plugin))| (window.clone(), plugin.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn matches(&self, window: &str, token: &str, plugin: &str) -> bool {
        self.bindings
            .lock()
            .ok()
            .and_then(|bindings| bindings.get(window).cloned())
            .is_some_and(|binding| binding.0 == token && binding.1 == plugin)
    }
}

pub struct PluginUpdateLease<'a> {
    sessions: &'a PluginSessions,
    plugin: String,
}
impl Drop for PluginUpdateLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut updating) = self.sessions.updating.lock() {
            updating.remove(&self.plugin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_lease_prevents_replacement_of_active_plugin_and_blocks_new_sessions() {
        let sessions = PluginSessions::default();
        let token = sessions.open("main", "one").unwrap();
        assert!(sessions.begin_update("one").is_err());
        sessions.close("main", &token);
        let lease = sessions.begin_update("one").unwrap();
        assert!(sessions.open("plugin-one", "one").is_err());
        assert!(sessions.begin_update("one").is_err());
        assert!(sessions.open("plugin-two", "two").is_ok());
        drop(lease);
        assert!(sessions.open("plugin-one", "one").is_ok());
    }
    #[test]
    fn sessions_bind_window_plugin_and_generation() {
        let sessions = PluginSessions::default();
        let token = sessions.open("main", "one").expect("session");
        assert!(sessions.matches("main", &token, "one"));
        assert!(!sessions.matches("plugin-one-1", &token, "one"));
        assert!(!sessions.matches("main", &token, "two"));
        assert!(!sessions.matches("main", "forged", "one"));
        let next = sessions.open("main", "two").expect("session");
        assert!(!sessions.matches("main", &token, "one"));
        assert!(sessions.matches("main", &next, "two"));
        sessions.close("main", &token);
        assert!(sessions.matches("main", &next, "two"));
        sessions.close("main", &next);
        assert!(!sessions.matches("main", &next, "two"));
    }
}
