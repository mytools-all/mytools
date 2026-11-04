use serde_json::Value;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

struct Record {
    created: Instant,
    plugin: String,
    payload: Value,
}

#[derive(Default)]
pub struct LaunchContexts {
    snapshot: Mutex<Option<(Instant, Value)>>,
    pending: Mutex<HashMap<String, Record>>,
}

impl LaunchContexts {
    pub fn capture(&self, payload: Value) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            *snapshot = Some((Instant::now(), payload));
        }
    }

    pub fn bind(&self, window: &str, plugin: &str) -> Result<(), String> {
        let snapshot = self.snapshot.lock().map_err(|_| "上下文不可用")?.take();
        let mut pending = self.pending.lock().map_err(|_| "上下文不可用")?;
        pending.retain(|_, record| record.created.elapsed() < Duration::from_secs(120));
        if let Some((created, payload)) =
            snapshot.filter(|(time, _)| time.elapsed() < Duration::from_secs(120))
        {
            pending.insert(
                window.into(),
                Record {
                    created,
                    plugin: plugin.into(),
                    payload,
                },
            );
        }
        Ok(())
    }

    pub fn take(&self, window: &str, plugin: &str) -> Result<Value, String> {
        let mut pending = self.pending.lock().map_err(|_| "上下文不可用")?;
        if let Some(record) = pending.get(window) {
            if record.plugin != plugin {
                return Err("上下文不属于此插件".into());
            }
        }
        Ok(pending
            .remove(window)
            .filter(|record| record.created.elapsed() < Duration::from_secs(120))
            .map(|record| record.payload)
            .unwrap_or(Value::Null))
    }

    pub fn forget(&self, window: &str) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(window);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_is_bound_to_the_selected_plugin_and_consumed_once() {
        let contexts = LaunchContexts::default();
        contexts.capture(serde_json::json!({"text":"fixture"}));
        contexts.bind("plugin-a-1", "a").unwrap();
        assert!(contexts.take("plugin-b-2", "b").unwrap().is_null());
        assert!(contexts.take("plugin-a-1", "b").is_err());
        assert_eq!(contexts.take("plugin-a-1", "a").unwrap()["text"], "fixture");
        assert!(contexts.take("plugin-a-1", "a").unwrap().is_null());
        contexts.bind("plugin-a-2", "a").unwrap();
        assert!(contexts.take("plugin-a-2", "a").unwrap().is_null());
    }

    #[test]
    fn new_capture_replaces_old_and_expired_context_is_not_delivered() {
        let contexts = LaunchContexts::default();
        contexts.capture(serde_json::json!({"text":"old"}));
        contexts.capture(serde_json::json!({"text":"new"}));
        contexts.bind("one", "a").unwrap();
        assert_eq!(contexts.take("one", "a").unwrap()["text"], "new");
        *contexts.snapshot.lock().unwrap() = Some((
            Instant::now() - Duration::from_secs(121),
            serde_json::json!({"text":"expired"}),
        ));
        contexts.bind("two", "a").unwrap();
        assert!(contexts.take("two", "a").unwrap().is_null());
    }
}
