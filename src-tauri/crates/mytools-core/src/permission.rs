//! Persisted capability grants — the user's consent records.
//!
//! A grant says: "plugin X may use capability Y". Grants are consulted by
//! the policy when a decision would otherwise require confirmation, and
//! can be reviewed and revoked from the settings page.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// File name of the grant store inside the app data directory.
pub const PERMISSION_FILE: &str = "permissions.json";

/// One consent record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Grant {
    /// Plugin the grant belongs to.
    pub plugin: String,
    /// Capability the plugin may use.
    pub capability: String,
    /// When the user granted it (RFC 3339, UTC).
    pub granted_at: String,
}

/// On-disk shape of the grant store.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantFile {
    #[serde(default)]
    pub grants: Vec<Grant>,
}

/// In-memory store backed by a JSON file.
pub struct GrantStore {
    path: PathBuf,
    grants: Mutex<Vec<Grant>>,
}

impl GrantStore {
    /// Load the store; a missing or broken file yields an empty store.
    pub fn load(path: PathBuf) -> Self {
        let grants = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<GrantFile>(&bytes).ok())
            .map(|file| file.grants)
            .unwrap_or_default();
        Self {
            path,
            grants: Mutex::new(grants),
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// All grants, newest first.
    pub fn list(&self) -> Vec<Grant> {
        let mut grants = self.grants.lock().map(|g| g.clone()).unwrap_or_default();
        grants.sort_by(|a, b| b.granted_at.cmp(&a.granted_at));
        grants
    }

    /// True when the plugin may use the capability without asking again.
    pub fn has(&self, plugin: &str, capability: &str) -> bool {
        self.grants
            .lock()
            .map(|grants| {
                grants.iter().any(|g| {
                    g.plugin.eq_ignore_ascii_case(plugin)
                        && g.capability.eq_ignore_ascii_case(capability)
                })
            })
            .unwrap_or(false)
    }

    /// Record a grant (idempotent).
    pub fn grant(&self, plugin: &str, capability: &str) -> Result<(), String> {
        let plugin = plugin.trim();
        let capability = capability.trim();
        if plugin.is_empty() || capability.is_empty() {
            return Err("plugin and capability are required".to_string());
        }
        let mut grants = self.grants.lock().map_err(|_| "grant store poisoned")?;
        if !grants.iter().any(|g| {
            g.plugin.eq_ignore_ascii_case(plugin) && g.capability.eq_ignore_ascii_case(capability)
        }) {
            grants.push(Grant {
                plugin: plugin.to_string(),
                capability: capability.to_string(),
                granted_at: Utc::now().to_rfc3339(),
            });
        }
        self.save(&grants)
    }

    /// Remove one grant.
    pub fn revoke(&self, plugin: &str, capability: &str) -> Result<bool, String> {
        let mut grants = self.grants.lock().map_err(|_| "grant store poisoned")?;
        let before = grants.len();
        grants.retain(|g| {
            !(g.plugin.eq_ignore_ascii_case(plugin)
                && g.capability.eq_ignore_ascii_case(capability))
        });
        let removed = grants.len() != before;
        self.save(&grants)?;
        Ok(removed)
    }

    /// Remove every grant of a plugin.
    pub fn revoke_all(&self, plugin: &str) -> Result<usize, String> {
        let mut grants = self.grants.lock().map_err(|_| "grant store poisoned")?;
        let before = grants.len();
        grants.retain(|g| !g.plugin.eq_ignore_ascii_case(plugin));
        let removed = before - grants.len();
        self.save(&grants)?;
        Ok(removed)
    }

    fn save(&self, grants: &[Grant]) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = GrantFile {
            grants: grants.to_vec(),
        };
        let bytes = serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, bytes).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store(dir: &TempDir) -> GrantStore {
        GrantStore::load(dir.path().join(PERMISSION_FILE))
    }

    #[test]
    fn missing_file_yields_empty_store() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        assert!(store.list().is_empty());
        assert!(!store.has("demo", "fs.read"));
    }

    #[test]
    fn grant_is_recorded_and_idempotent() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        store.grant("demo", "fs.read").unwrap();
        store.grant("demo", "fs.read").unwrap();
        assert_eq!(store.list().len(), 1);
        assert!(store.has("demo", "fs.read"));
        assert!(!store.has("demo", "fs.write"));
    }

    #[test]
    fn grants_persist_across_reload() {
        let dir = TempDir::new().unwrap();
        store(&dir).grant("demo", "fs.read").unwrap();
        let reloaded = store(&dir);
        assert!(reloaded.has("demo", "fs.read"));
    }

    #[test]
    fn revoke_removes_one_grant() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        store.grant("demo", "fs.read").unwrap();
        store.grant("demo", "fs.write").unwrap();
        assert!(store.revoke("demo", "fs.read").unwrap());
        assert!(!store.has("demo", "fs.read"));
        assert!(store.has("demo", "fs.write"));
        assert!(!store.revoke("demo", "missing").unwrap());
    }

    #[test]
    fn revoke_all_clears_a_plugin() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        store.grant("demo", "fs.read").unwrap();
        store.grant("demo", "fs.write").unwrap();
        store.grant("other", "fs.read").unwrap();
        assert_eq!(store.revoke_all("demo").unwrap(), 2);
        assert!(store.has("other", "fs.read"));
    }

    #[test]
    fn empty_grant_is_rejected() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        assert!(store.grant("", "fs.read").is_err());
        assert!(store.grant("demo", "  ").is_err());
    }
}
