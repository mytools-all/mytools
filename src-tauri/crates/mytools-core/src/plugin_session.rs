use std::collections::HashMap;

#[derive(Default)]
pub struct PluginSessions {
    bindings: std::sync::Mutex<HashMap<String, (String, String)>>,
    sequence: std::sync::atomic::AtomicU64,
}

impl PluginSessions {
    pub fn open(&self, window: &str, plugin: &str) -> Result<String, String> {
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

    pub fn matches(&self, window: &str, token: &str, plugin: &str) -> bool {
        self.bindings
            .lock()
            .ok()
            .and_then(|bindings| bindings.get(window).cloned())
            .is_some_and(|binding| binding.0 == token && binding.1 == plugin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
