use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct Busy {
    plugins: HashSet<String>,
    maintenance: bool,
    operations: usize,
}

#[derive(Default)]
pub struct PluginSessions {
    bindings: std::sync::Mutex<HashMap<String, (String, String)>>,
    updating: std::sync::Mutex<Busy>,
    sequence: std::sync::atomic::AtomicU64,
}

impl PluginSessions {
    pub fn begin_mutation(&self) -> Result<MutationLease<'_>, String> {
        let mut busy = self.updating.lock().map_err(|e| e.to_string())?;
        if busy.maintenance {
            return Err("正在备份或恢复，请稍后修改插件".into());
        }
        busy.operations += 1;
        Ok(MutationLease { sessions: self })
    }
    pub fn begin_maintenance(&self) -> Result<MaintenanceLease<'_>, String> {
        let mut busy = self.updating.lock().map_err(|e| e.to_string())?;
        if busy.maintenance || !busy.plugins.is_empty() || busy.operations > 0 {
            return Err("已有安装、更新或数据维护操作，请稍候".into());
        }
        if !self.bindings.lock().map_err(|e| e.to_string())?.is_empty() {
            return Err("请先保存内容并关闭插件，再进行备份或恢复".into());
        }
        busy.maintenance = true;
        Ok(MaintenanceLease { sessions: self })
    }
    pub fn open(&self, window: &str, plugin: &str) -> Result<String, String> {
        let updating = self.updating.lock().map_err(|e| e.to_string())?;
        if updating.maintenance || updating.plugins.contains(plugin) {
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
        if updating.maintenance || updating.plugins.contains(plugin) {
            return Err("插件已经在更新中".into());
        }
        let bindings = self.bindings.lock().map_err(|e| e.to_string())?;
        if bindings.values().any(|(_, name)| name == plugin) {
            return Err("插件正在使用，请先保存内容并关闭插件，再重试更新".into());
        }
        updating.plugins.insert(plugin.to_string());
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

pub struct MaintenanceLease<'a> {
    sessions: &'a PluginSessions,
}
pub struct MutationLease<'a> {
    sessions: &'a PluginSessions,
}
impl Drop for MutationLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut busy) = self.sessions.updating.lock() {
            busy.operations = busy.operations.saturating_sub(1);
        }
    }
}
impl Drop for MaintenanceLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut busy) = self.sessions.updating.lock() {
            busy.maintenance = false;
        }
    }
}
impl Drop for PluginUpdateLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut updating) = self.sessions.updating.lock() {
            updating.plugins.remove(&self.plugin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maintenance_waits_for_mutations_and_blocks_sessions_until_released() {
        let sessions = PluginSessions::default();
        let mutation = sessions.begin_mutation().unwrap();
        assert!(sessions.begin_maintenance().is_err());
        drop(mutation);
        let maintenance = sessions.begin_maintenance().unwrap();
        assert!(sessions.begin_mutation().is_err());
        assert!(sessions.begin_update("one").is_err());
        assert!(sessions.open("main", "one").is_err());
        drop(maintenance);
        assert!(sessions.open("main", "one").is_ok());
        assert!(sessions.begin_maintenance().is_err());
    }
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
