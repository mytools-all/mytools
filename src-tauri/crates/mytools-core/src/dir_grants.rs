//! User-granted directory access for plugins (设置 → 插件 → 目录授权).
//!
//! A plugin cannot touch the user's real files by default. When the
//! plugin needs a real directory (批量重命名 / 本地搜索), the host opens
//! the native directory dialog for the user; the chosen directory is
//! recorded here and `fs.*` capabilities may then operate inside it.
//! Grants persist in the config root so they survive restarts and can
//! be revoked from the settings page at any time.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// File the grants persist to (config root, like `permissions.json`).
pub const DIR_GRANTS_FILE: &str = "directory-grants.json";

/// One persisted grant: `plugin` scope nails the directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirGrant {
    pub plugin: String,
    pub path: String,
}

#[derive(Debug)]
pub struct DirGrantStore {
    path: PathBuf,
    inner: RwLock<Vec<DirGrant>>,
}

impl DirGrantStore {
    pub fn load(path: PathBuf) -> Self {
        let grants = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Vec<DirGrant>>(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            inner: RwLock::new(grants),
        }
    }

    pub fn list(&self) -> Vec<DirGrant> {
        self.inner
            .read()
            .expect("dir grants read")
            .iter()
            .map(|g| DirGrant {
                plugin: g.plugin.clone(),
                path: g.path.clone(),
            })
            .collect()
    }

    /// Record a grant. Duplicate grants for the same plugin + directory
    /// are ignored; the directory must exist.
    pub fn grant(&self, plugin: &str, dir: &Path) -> Result<(), String> {
        let dir = dir
            .canonicalize()
            .map_err(|e| format!("directory {} unavailable: {}", dir.display(), e))?;
        if !dir.is_dir() {
            return Err(format!("{} is not a directory", dir.display()));
        }
        let path = strip_verbatim(&dir.to_string_lossy());
        let mut grants = self.inner.write().expect("dir grants write");
        if grants
            .iter()
            .any(|g| g.plugin == plugin && same_dir(&g.path, &path))
        {
            return Ok(());
        }
        if grants.len() >= 100 {
            return Err("too many directory grants".into());
        }
        grants.push(DirGrant {
            plugin: plugin.to_string(),
            path,
        });
        save(&self.path, &grants)
    }

    /// Remove grants. `dir = None` revokes every directory of `plugin`.
    pub fn revoke(&self, plugin: &str, dir: Option<&str>) -> Result<usize, String> {
        let wanted = dir.map(|d| {
            let canonical = std::path::Path::new(d)
                .canonicalize()
                .map(|p| strip_verbatim(&p.to_string_lossy()))
                .unwrap_or_else(|_| strip_verbatim(d));
            canonical
        });
        let mut grants = self.inner.write().expect("dir grants write");
        let before = grants.len();
        grants.retain(|g| {
            !(g.plugin == plugin
                && match &wanted {
                    Some(want) => same_dir(&g.path, want),
                    None => true,
                })
        });
        let removed = before - grants.len();
        if removed > 0 {
            save(&self.path, &grants)?;
        }
        Ok(removed)
    }

    /// Whether `target` (a file or subdirectory) lies inside a directory
    /// the given plugin was granted. Path comparison is case-insensitive
    /// and length-checked at the separator, so `C:\a\b` does not match
    /// the grant `C:\a\bc`.
    pub fn is_granted(&self, plugin: &str, target: &Path) -> bool {
        let target_raw = target.to_string_lossy().into_owned();
        let target_canonical = target
            .canonicalize()
            .map(|p| strip_verbatim(&p.to_string_lossy()))
            .unwrap_or_else(|_| strip_verbatim(&target_raw));
        let grants = self.inner.read().expect("dir grants read");
        grants.iter().any(|g| {
            g.plugin == plugin
                && (covers(&g.path, &target_canonical) || covers(&g.path, &target_raw))
        })
    }
}
/// Windows canonical paths carry a `\\?\` verbatim prefix; drop it so
/// the stored path stays readable and comparable with plain paths.
fn strip_verbatim(path: &str) -> String {
    path.strip_prefix(r"\\?\").unwrap_or(path).to_string()
}

/// Windows paths compare case-insensitively; `/` and `\` are equal.
fn same_dir(left: &str, right: &str) -> bool {
    normalize(left) == normalize(right)
}

fn covers(grant: &str, target: &str) -> bool {
    let grant_n = normalize(grant);
    let target_n = normalize(target);
    target_n.len() >= grant_n.len()
        && target_n.starts_with(&grant_n)
        && (target_n.len() == grant_n.len() || target_n.as_bytes()[grant_n.len()] == b'\\')
}

fn normalize(path: &str) -> String {
    let mut out = strip_verbatim(path.trim())
        .to_ascii_lowercase()
        .replace('/', "\\");
    while out.ends_with('\\') && out.len() > 3 {
        out.pop();
    }
    out
}

fn save(path: &Path, grants: &[DirGrant]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        path,
        serde_json::to_vec_pretty(grants).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// All directory grants (settings page).
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn store(dir: &TempDir) -> DirGrantStore {
        DirGrantStore::load(dir.path().join("grants").join(DIR_GRANTS_FILE))
    }

    #[test]
    fn paths_compare_case_insensitively() {
        assert!(same_dir("C:\\Tools", "c:/tools"));
        assert!(same_dir("C:\\Tools\\", "C:\\Tools"));
        assert!(!same_dir("C:\\Tools", "C:\\Tools2"));
    }

    #[test]
    fn prefix_match_requires_a_separator_boundary() {
        assert!(covers("C:\\data", "C:\\data\\a.txt"));
        assert!(covers("C:\\data", "C:\\data"));
        assert!(covers("C:\\data", "c:\\Data\\sub\\x"));
        assert!(!covers("C:\\data", "C:\\database\\x"));
        assert!(!covers("C:\\data", "D:\\data\\x"));
    }

    #[test]
    fn grants_round_trip_and_dedupe() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("workspace");
        fs::create_dir_all(&root).unwrap();
        let grants = store(&dir);
        assert!(grants.list().is_empty());

        grants.grant("demo", &root).unwrap();
        assert!(grants.is_granted("demo", &root.join("a.txt")));

        let reloaded = store(&dir);
        assert!(
            reloaded.is_granted("demo", &root.join("a.txt")),
            "persisted"
        );
        assert!(!reloaded.is_granted("other", &root.join("a.txt")));
        // Granting the same directory again is a no-op.
        reloaded.grant("demo", &root).unwrap();
        assert_eq!(reloaded.list().len(), 1);
        assert!(!reloaded.list()[0].path.is_empty());
    }

    #[test]
    fn revokes_by_plugin_or_single_path() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("a")).unwrap();
        fs::create_dir(dir.path().join("b")).unwrap();
        let grants = store(&dir);
        grants.grant("demo", &dir.path().join("a")).unwrap();
        grants.grant("demo", &dir.path().join("b")).unwrap();
        grants.grant("other", &dir.path().join("a")).unwrap();

        let removed = grants
            .revoke("demo", Some(&dir.path().join("b").to_string_lossy()))
            .unwrap();
        assert_eq!(removed, 1);
        assert_eq!(grants.list().len(), 2);

        grants.revoke("demo", None).unwrap();
        assert_eq!(grants.list().len(), 1);
        assert_eq!(grants.list()[0].plugin, "other");
    }

    #[test]
    fn missing_directory_is_rejected() {
        let dir = TempDir::new().unwrap();
        let grants = store(&dir);
        assert!(grants.grant("demo", &dir.path().join("nope")).is_err());
    }
}
