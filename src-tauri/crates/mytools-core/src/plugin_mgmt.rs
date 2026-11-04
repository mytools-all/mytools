//! Plugin management — enabled-state persistence, uninstall and archive
//! installation with strict path-safety guards.
//!
//! Pure-Rust (no Tauri) so every security-sensitive path check is
//! unit-testable on its own (see docs/07-testing.md §3).

use crate::capability::Origin;
use crate::manifest::{PluginManifest, PKG_MANIFEST_FILE};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const INSTALL_STAGING_PREFIX: &str = ".mytools-install-";

/// Name of the JSON file that stores the disabled-plugin set under
/// the app data directory.
pub const PLUGIN_STATE_FILE: &str = "plugin-state.json";
/// Name of the JSON file that records which plugins were installed
/// through an ecosystem channel (their provenance, not their dialect).
pub const PLUGIN_ORIGINS_FILE: &str = "plugin-origins.json";
/// Filename every plugin directory must contain.
pub const PLUGIN_MANIFEST_FILE: &str = "plugin.json";

/// Upper bound on archive entries before we refuse to extract — a
/// plugin package is a small web app, not a tarbomb.
const MAX_ARCHIVE_ENTRIES: usize = 20_000;

#[derive(Debug, thiserror::Error)]
pub enum PluginMgrError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest: {0}")]
    Manifest(#[from] crate::manifest::ManifestError),
    #[error("state json: {0}")]
    StateJson(#[from] serde_json::Error),
    #[error("invalid plugin name: {0}")]
    BadName(String),
    #[error("plugin not found: {0}")]
    NotFound(String),
    #[error("plugin already installed: {0}")]
    AlreadyInstalled(String),
    #[error("archive: {0}")]
    Archive(String),
    #[error("no plugin manifest (plugin.json / package.json) found")]
    NoManifest,
    #[error("plugin origins file is invalid: {0}")]
    BadOrigins(String),
    #[error("unsafe archive entry: {0}")]
    UnsafeEntry(String),
}

/// Enabled/disabled state for plugins. Only the *disabled* set is
/// persisted — everything not listed stays enabled by default.
#[derive(Debug, Clone, Default)]
pub struct PluginState {
    disabled: BTreeSet<String>,
}

impl PluginState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        !self.disabled.contains(name)
    }

    /// Returns the *new* state on success (input requested state).
    pub fn set_enabled(&mut self, name: &str, on: bool) -> bool {
        let enabled = self.is_enabled(name);
        if enabled == on {
            return enabled;
        }
        if on {
            self.disabled.remove(name);
        } else {
            self.disabled.insert(name.to_string());
        }
        on
    }

    pub fn disabled_names(&self) -> Vec<String> {
        self.disabled.iter().cloned().collect()
    }

    pub fn load(path: &Path) -> Result<Self, PluginMgrError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let bytes = fs::read(path)?;
        let list: Vec<String> = serde_json::from_slice(&bytes)?;
        Ok(Self {
            disabled: list.into_iter().collect(),
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), PluginMgrError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(&self.disabled_names())?;
        fs::write(path, bytes)?;
        Ok(())
    }
}

/// Provenance record for plugins that did not come from a plain scan.
///
/// Scanning a directory always yields `Custom`; only an install path may
/// promote a plugin to `Ecosystem` (marketplace) or `Official` (shipped
/// with the product). See D015.
#[derive(Debug, Clone, Default)]
pub struct PluginOrigins {
    origins: BTreeMap<String, Origin>,
}

impl PluginOrigins {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn origin_of(&self, name: &str) -> Option<Origin> {
        self.origins.get(name).copied()
    }

    pub fn is_ecosystem(&self, name: &str) -> bool {
        self.origin_of(name) == Some(Origin::Ecosystem)
    }

    pub fn is_official(&self, name: &str) -> bool {
        self.origin_of(name) == Some(Origin::Official)
    }

    fn set(&mut self, name: &str, origin: Origin) -> bool {
        self.origins.insert(name.to_string(), origin) != Some(origin)
    }

    /// Returns true when the record changed.
    pub fn mark_ecosystem(&mut self, name: &str) -> bool {
        self.set(name, Origin::Ecosystem)
    }

    /// Returns true when the record changed.
    pub fn mark_official(&mut self, name: &str) -> bool {
        self.set(name, Origin::Official)
    }

    /// Returns true when a record was removed.
    pub fn forget(&mut self, name: &str) -> bool {
        self.origins.remove(name).is_some()
    }

    pub fn names(&self) -> Vec<String> {
        self.origins.keys().cloned().collect()
    }

    pub fn load(path: &Path) -> Result<Self, PluginMgrError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let bytes = fs::read(path)?;
        // Current format: `{ "<name>": "ecosystem" | "official" }`.
        if let Ok(map) = serde_json::from_slice::<BTreeMap<String, Origin>>(&bytes) {
            return Ok(Self { origins: map });
        }
        // Legacy format: a plain array of ecosystem package names.
        if let Ok(list) = serde_json::from_slice::<Vec<String>>(&bytes) {
            return Ok(Self {
                origins: list
                    .into_iter()
                    .map(|name| (name, Origin::Ecosystem))
                    .collect(),
            });
        }
        Err(PluginMgrError::BadOrigins(
            "plugin origins file must be an object or a legacy array".to_string(),
        ))
    }

    pub fn save(&self, path: &Path) -> Result<(), PluginMgrError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(&self.origins)?;
        fs::write(path, bytes)?;
        Ok(())
    }
}

/// Whether `name` is a valid, traversal-free plugin directory name.
/// Scoped npm names like `@scope/pkg` are allowed (nested sub-paths).
pub fn is_safe_plugin_name(name: &str) -> bool {
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    name.split(['/', '\\']).all(|part| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '@' | '+' | '~'))
    })
}

/// Resolve `root/<name>` after validating the name. The result is
/// guaranteed to stay within `root`.
pub fn safe_plugin_dir(root: &Path, name: &str) -> Result<PathBuf, PluginMgrError> {
    if !is_safe_plugin_name(name) {
        return Err(PluginMgrError::BadName(name.to_string()));
    }
    let resolved = root.join(name);
    if !resolved.starts_with(root) {
        return Err(PluginMgrError::BadName(name.to_string()));
    }
    Ok(resolved)
}

/// Read the manifest for an installed plugin directory: `plugin.json`
/// first, else a `package.json` that carries rubick plugin fields.
///
/// Identity fields missing from a partial `plugin.json` are merged in
/// from the sibling `package.json` (e.g. `rubick-markdown`).
pub fn read_manifest(dir: &Path) -> Result<PluginManifest, PluginMgrError> {
    let plugin_file = dir.join(PLUGIN_MANIFEST_FILE);
    if plugin_file.is_file() {
        let mut manifest = PluginManifest::from_path(&plugin_file)?;
        manifest.merge_package_json_identity(dir);
        return Ok(manifest);
    }
    if let Some(mut manifest) = PluginManifest::from_package_json_path(&dir.join(PKG_MANIFEST_FILE))
    {
        manifest.merge_package_json_identity(dir);
        return Ok(manifest);
    }
    Err(PluginMgrError::NoManifest)
}

/// First candidate that is a valid, traversal-free directory name.
pub fn first_safe_name<'a>(candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    candidates
        .into_iter()
        .map(str::trim)
        .find(|candidate| is_safe_plugin_name(candidate))
        .map(str::to_string)
}

/// Directory name an installed plugin should use. Prefers the manifest
/// `name`, then a sanitized `pluginName`, then the source directory
/// name.
pub fn resolve_install_name(
    manifest: &PluginManifest,
    src_dir: &Path,
) -> Result<String, PluginMgrError> {
    let plugin_name = manifest.plugin_name.as_deref().unwrap_or("");
    let dir_name = src_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    first_safe_name([manifest.name.as_str(), plugin_name, dir_name])
        .ok_or_else(|| PluginMgrError::BadName(manifest.name.clone()))
}

/// Name for an archive install: manifest name → `pluginName`. Never
/// falls back to a directory name because the source is a temp staging
/// dir (npm tarballs root at `package/`).
pub fn archive_install_name(manifest: &PluginManifest) -> Result<String, PluginMgrError> {
    let plugin_name = manifest.plugin_name.as_deref().unwrap_or("");
    first_safe_name([manifest.name.as_str(), plugin_name])
        .ok_or_else(|| PluginMgrError::BadName(manifest.name.clone()))
}

/// Remove a plugin directory. Refuses to touch anything that does not
/// look like a plugin (i.e. has no parseable manifest).
pub fn uninstall_plugin(root: &Path, name: &str) -> Result<(), PluginMgrError> {
    let dir = safe_plugin_dir(root, name)?;
    if !dir.is_dir() {
        return Err(PluginMgrError::NotFound(name.to_string()));
    }
    read_manifest(&dir).map_err(|_| PluginMgrError::NotFound(name.to_string()))?;
    fs::remove_dir_all(&dir)?;
    Ok(())
}

/// Copy an unpacked plugin directory into `root`. Returns the plugin
/// name. Fails if the target name is already installed.
pub fn install_from_dir(src: &Path, root: &Path) -> Result<String, PluginMgrError> {
    let manifest = read_manifest(src)?;
    let name = resolve_install_name(&manifest, src)?;
    install_staged(&name, src, root)
}

fn install_staged(name: &str, src: &Path, root: &Path) -> Result<String, PluginMgrError> {
    let dest = safe_plugin_dir(root, name)?;
    if dest.exists() {
        return Err(PluginMgrError::AlreadyInstalled(name.to_string()));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    copy_dir_contents(src, &dest)?;
    Ok(name.to_string())
}

fn copy_dir_contents(src: &Path, dest: &Path) -> Result<(), PluginMgrError> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            let to = dest.join(entry.file_name());
            copy_dir_contents(&from, &to)?;
        } else if ft.is_symlink() {
            // never follow symlinks during install
            continue;
        } else if ft.is_file() {
            copy_file_with_parent(&from, &dest.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn copy_file_with_parent(from: &Path, to: &Path) -> Result<(), PluginMgrError> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(from, to)?;
    Ok(())
}

/// Install a plugin from a `.zip` archive. The archive must contain a
/// `plugin.json` (at any depth), or a `package.json` carrying rubick
/// plugin fields; only its containing tree is extracted and installed
/// as `root/<manifest.name>`.
pub fn install_from_zip(zip_path: &Path, root: &Path) -> Result<String, PluginMgrError> {
    let file = fs::File::open(zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| PluginMgrError::Archive(e.to_string()))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(PluginMgrError::Archive("too many entries".into()));
    }

    let entries: Vec<(String, bool)> = (0..archive.len())
        .filter_map(|i| {
            let entry = archive.by_index(i).ok()?;
            let name = entry.name().to_string();
            let is_dir = entry.is_dir();
            Some((name, is_dir))
        })
        .collect();

    let manifest_rel = entries
        .iter()
        .find(|(name, is_dir)| !*is_dir && name.ends_with("/plugin.json"))
        .map(|(name, _)| name.clone())
        .or_else(|| {
            entries
                .iter()
                .find(|(name, is_dir)| !*is_dir && name == PLUGIN_MANIFEST_FILE)
                .map(|(name, _)| name.clone())
        })
        .or_else(|| {
            // rubick npm packages may only carry the manifest inside
            // package.json; read_manifest later validates the fields.
            entries
                .iter()
                .find(|(name, is_dir)| {
                    !*is_dir && (name.ends_with("/package.json") || name == PKG_MANIFEST_FILE)
                })
                .map(|(name, _)| name.clone())
        })
        .ok_or(PluginMgrError::NoManifest)?;

    let prefix: PathBuf = Path::new(&manifest_rel)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();

    let staging_guard = staging_dir(root)?;
    let staging = staging_guard.path().to_path_buf();
    let result = (|| -> Result<String, PluginMgrError> {
        let mut guard_manifest_found = false;
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| PluginMgrError::Archive(e.to_string()))?;
            let name = entry.name().to_string();
            guard_entry_path(&name)?;
            if entry.is_dir() {
                continue;
            }
            if name == manifest_rel {
                guard_manifest_found = true;
            }
            let Ok(rel) = Path::new(&name).strip_prefix(&prefix) else {
                continue;
            };
            if rel.as_os_str().is_empty() {
                continue;
            }
            let dest = staging.join(rel);
            copy_entry_to_file(&mut entry, &dest)?;
        }
        if !guard_manifest_found {
            return Err(PluginMgrError::NoManifest);
        }
        let name = archive_install_name(&read_manifest(&staging)?)?;
        let dest = safe_plugin_dir(root, &name)?;
        if dest.exists() {
            return Err(PluginMgrError::AlreadyInstalled(name));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&staging, &dest)?;
        Ok(name)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

/// Install a plugin from an npm-style `.tgz` tarball (Phase 2). The
/// archive contains a single `package/` directory at its root; that
/// whole tree becomes `root/<manifest.name>`.
pub fn install_from_tar_gz(tar_path: &Path, root: &Path) -> Result<String, PluginMgrError> {
    let file = fs::File::open(tar_path)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    let staging_guard = staging_dir(root)?;
    let staging = staging_guard.path().to_path_buf();
    let result = (|| -> Result<String, PluginMgrError> {
        let mut count = 0usize;
        for entry in archive.entries()? {
            let mut entry = entry?;
            let kind = entry.header().entry_type();
            if !kind.is_file() && !kind.is_dir() {
                return Err(PluginMgrError::UnsafeEntry("tar links and special files are not supported".into()));
            }
            count += 1;
            if count > MAX_ARCHIVE_ENTRIES {
                return Err(PluginMgrError::Archive("too many entries".into()));
            }
            let path = entry
                .path()
                .map_err(|e| PluginMgrError::Archive(e.to_string()))?;
            let rel: PathBuf = path.components().skip(1).collect();
            if rel.as_os_str().is_empty() {
                continue;
            }
            let rel = rel.to_string_lossy().replace('\\', "/");
            guard_entry_path(&rel)?;
            let dest = staging.join(&rel);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            entry.unpack(&dest)?;
        }
        let name = archive_install_name(&read_manifest(&staging)?)?;
        let dest = safe_plugin_dir(root, &name)?;
        if dest.exists() {
            return Err(PluginMgrError::AlreadyInstalled(name));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&staging, &dest)?;
        Ok(name)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

pub fn update_from_tar_gz(
    pack: &Path, root: &Path, name: &str, from_version: &str, version: &str,
) -> Result<String, PluginMgrError> {
    let dest = safe_plugin_dir(root, name)?;
    if fs::symlink_metadata(&dest)?.file_type().is_symlink() {
        return Err(PluginMgrError::UnsafeEntry(name.into()));
    }
    let previous = read_manifest(&dest)?;
    if previous.version != from_version {
        return Err(PluginMgrError::Archive("插件版本已变化，请重新检查更新".into()));
    }
    let staging = staging_dir(root)?;
    let installed = install_from_tar_gz(pack, staging.path())?;
    if installed != name {
        return Err(PluginMgrError::Archive("更新包身份与已安装插件不一致".into()));
    }
    let candidate = staging.path().join(name);
    let manifest = read_manifest(&candidate)?;
    if manifest.version != version {
        return Err(PluginMgrError::Archive("更新包版本与请求版本不一致".into()));
    }
    let entry = candidate.join(&manifest.main).canonicalize()?;
    if !entry.starts_with(candidate.canonicalize()?) || !entry.is_file() {
        return Err(PluginMgrError::UnsafeEntry(manifest.main));
    }
    let backup_root = staging_dir(staging.path())?.keep();
    let backup = backup_root.join("previous");
    if let Err(error) = replace_update(&candidate, &dest, &backup, || fs::rename(&candidate, &dest)) {
        if backup.exists() {
            let recovery = staging.keep();
            return Err(PluginMgrError::Archive(format!("{error}；旧版保留于 {}", recovery.display())));
        }
        return Err(error);
    }
    Ok(name.into())
}

fn replace_update(
    _candidate: &Path, dest: &Path, backup: &Path,
    activate: impl FnOnce() -> std::io::Result<()>,
) -> Result<(), PluginMgrError> {
    fs::rename(dest, backup)?;
    if let Err(error) = activate() {
        fs::rename(backup, dest).map_err(|rollback| PluginMgrError::Archive(format!("更新失败：{error}；恢复失败：{rollback}")))?;
        return Err(error.into());
    }
    Ok(())
}

fn staging_dir(root: &Path) -> Result<tempfile::TempDir, PluginMgrError> {
    fs::create_dir_all(root)?;
    Ok(tempfile::Builder::new()
        .prefix(INSTALL_STAGING_PREFIX)
        .tempdir_in(root)?)
}

fn guard_entry_path(name: &str) -> Result<(), PluginMgrError> {
    if name.is_empty() || name.contains("..") || Path::new(name).is_absolute() {
        return Err(PluginMgrError::UnsafeEntry(name.to_string()));
    }
    Ok(())
}

fn copy_entry_to_file(from: &mut impl std::io::Read, to: &Path) -> Result<(), PluginMgrError> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = fs::File::create(to)?;
    std::io::copy(from, &mut out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_is_inside_the_target_root_and_cleans_up() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("plugins");
        let path = {
            let staging = staging_dir(&root).unwrap();
            assert_eq!(staging.path().parent(), Some(root.as_path()));
            staging.path().to_path_buf()
        };
        assert!(!path.exists());
    }

    #[test]
    fn safe_names_accepted_and_rejected() {
        for ok in ["demo-hello", "rubick-plugin-json", "@scope/pkg", "a.b_c+d"] {
            assert!(is_safe_plugin_name(ok), "{ok}");
        }
        for bad in [
            "", ".", "..", "../evil", "a/../b", "a\\..\\b", "/abs", "a/b/c/",
        ] {
            assert!(!is_safe_plugin_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn safe_plugin_dir_stays_within_root() {
        let root = Path::new("C:/plugins");
        assert_eq!(
            safe_plugin_dir(root, "demo").unwrap(),
            Path::new("C:/plugins/demo")
        );
        assert!(safe_plugin_dir(root, "..").is_err());
        assert!(safe_plugin_dir(root, "../escape").is_err());
        assert!(safe_plugin_dir(root, "a/../../b").is_err());
    }

    #[test]
    fn uninstall_refuses_non_plugin_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin = dir.path().join("fake");
        fs::create_dir_all(&plugin).unwrap();
        fs::write(plugin.join("readme.txt"), "not a plugin").unwrap();
        assert!(matches!(
            uninstall_plugin(dir.path(), "fake"),
            Err(PluginMgrError::NotFound(_))
        ));
        assert!(plugin.exists(), "must not delete non-plugin dir");

        // a manifest that does not parse must also refuse uninstall
        fs::write(plugin.join("plugin.json"), "{}").unwrap();
        assert!(matches!(
            uninstall_plugin(dir.path(), "fake"),
            Err(PluginMgrError::NotFound(_))
        ));
        assert!(plugin.exists());

        fs::write(
            plugin.join("plugin.json"),
            r#"{ "name": "fake", "description": "x", "version": "0.1.0", "main": "index.html" }"#,
        )
        .unwrap();
        uninstall_plugin(dir.path(), "fake").unwrap();
        assert!(!plugin.exists());
    }

    #[test]
    fn install_from_dir_copies_and_rejects_duplicates() {
        let dir = tempfile::TempDir::new().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("plugin.json"),
            r#"{ "name": "hello", "description": "x", "version": "0.1.0", "main": "index.html" }"#,
        )
        .unwrap();
        fs::write(src.join("index.html"), "<html></html>").unwrap();
        fs::create_dir_all(src.join("assets")).unwrap();
        fs::write(src.join("assets").join("a.js"), "x").unwrap();

        let root = dir.path().join("plugins");
        let name = install_from_dir(&src, &root).unwrap();
        assert_eq!(name, "hello");
        assert!(root.join("hello").join("index.html").is_file());
        assert!(root.join("hello").join("assets").join("a.js").is_file());
        assert!(matches!(
            install_from_dir(&src, &root),
            Err(PluginMgrError::AlreadyInstalled(_))
        ));
    }

    #[test]
    fn install_from_zip_extracts_only_plugin_tree() {
        let dir = tempfile::TempDir::new().unwrap();
        let zip_path = dir.path().join("bundle.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        fn add(zw: &mut zip::ZipWriter<fs::File>, name: &str, data: &[u8]) {
            zw.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(zw, data).unwrap();
        }
        add(
            &mut zw,
            "pkg/plugin.json",
            br#"{ "name": "zplug", "description": "x", "version": "0.1.0", "main": "index.html" }"#,
        );
        add(&mut zw, "pkg/index.html", b"<html></html>");
        add(&mut zw, "readme.txt", b"ignore me");
        zw.finish().unwrap();

        let root = dir.path().join("plugins");
        let name = install_from_zip(&zip_path, &root).unwrap();
        assert_eq!(name, "zplug");
        assert!(root.join("zplug").join("index.html").is_file());
        assert!(!root.join("zplug").join("readme.txt").exists());
    }

    #[test]
    fn zip_with_traversal_entry_is_rejected() {
        let dir = tempfile::TempDir::new().unwrap();
        let zip_path = dir.path().join("evil.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        zw.start_file("pkg/plugin.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(
            &mut zw,
            br#"{ "name": "evil", "description": "x", "version": "0.1.0", "main": "index.html" }"#,
        )
        .unwrap();
        zw.start_file("pkg/../evil", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut zw, b"x").unwrap();
        zw.finish().unwrap();

        let root = dir.path().join("plugins");
        assert!(matches!(
            install_from_zip(&zip_path, &root),
            Err(PluginMgrError::UnsafeEntry(_))
        ));
    }

    #[test]
    fn install_from_tar_gz_strips_package_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let tar_path = dir.path().join("pkg.tgz");
        let file = fs::File::create(&tar_path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tw = tar::Builder::new(enc);
        let manifest = r#"{ "name": "tgzplug", "description": "x", "version": "0.1.0", "main": "index.html" }"#;
        let mut mh = tar::Header::new_gnu().clone();
        mh.set_size(manifest.len() as u64);
        mh.set_mode(0o644);
        mh.set_cksum();
        tw.append_data(&mut mh, "package/plugin.json", manifest.as_bytes())
            .unwrap();
        let html = b"<html></html>";
        let mut ih = tar::Header::new_gnu().clone();
        ih.set_size(html.len() as u64);
        ih.set_mode(0o644);
        ih.set_cksum();
        tw.append_data(&mut ih, "package/index.html", &html[..])
            .unwrap();
        let enc = tw.into_inner().unwrap();
        drop(enc);

        let root = dir.path().join("plugins");
        let name = install_from_tar_gz(&tar_path, &root).unwrap();
        assert_eq!(name, "tgzplug");
        assert!(root.join("tgzplug").join("index.html").is_file());
        assert!(root.join("tgzplug").join("plugin.json").is_file());
    }

    #[test]
    fn state_roundtrips_through_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("plugin-state.json");
        let mut state = PluginState::new();
        assert!(state.is_enabled("a"));
        state.set_enabled("a", false);
        state.set_enabled("b", true);
        state.save(&path).unwrap();

        let loaded = PluginState::load(&path).unwrap();
        assert!(!loaded.is_enabled("a"));
        assert!(loaded.is_enabled("b"));
        assert_eq!(loaded.disabled_names(), vec!["a".to_string()]);
    }

    #[test]
    fn state_load_missing_file_is_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let state = PluginState::load(&dir.path().join("nope.json")).unwrap();
        assert_eq!(state.disabled_names(), Vec::<String>::new());
    }

    const PKG_MANIFEST: &str = r#"{
        "name": "rubick-plugin-translate",
        "pluginName": "翻译",
        "description": "翻译",
        "version": "1.0.2",
        "pluginType": "ui",
        "main": "index.html",
        "preload": "preload.js",
        "features": [{ "code": "index", "explain": "翻译", "cmds": ["fy", "translate"] }]
    }"#;

    #[test]
    fn install_from_dir_accepts_package_json_manifest() {
        let dir = tempfile::TempDir::new().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("package.json"), PKG_MANIFEST).unwrap();
        fs::write(src.join("index.html"), "<html></html>").unwrap();

        let root = dir.path().join("plugins");
        let name = install_from_dir(&src, &root).unwrap();
        assert_eq!(name, "rubick-plugin-translate");
        assert!(root.join(&name).join("index.html").is_file());
        uninstall_plugin(&root, &name).unwrap();
        assert!(!root.join(&name).exists());
    }

    #[test]
    fn install_from_tar_gz_accepts_package_json_manifest() {
        let dir = tempfile::TempDir::new().unwrap();
        let tar_path = dir.path().join("translate.tgz");
        let manifest = PKG_MANIFEST;
        let file = fs::File::create(&tar_path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tw = tar::Builder::new(enc);
        for (p, data) in [
            ("package/package.json", manifest),
            ("package/index.html", "<html></html>"),
        ] {
            let mut h = tar::Header::new_gnu().clone();
            h.set_size(data.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            tw.append_data(&mut h, p, data.as_bytes()).unwrap();
        }
        let enc = tw.into_inner().unwrap();
        drop(enc);

        let root = dir.path().join("plugins");
        let name = install_from_tar_gz(&tar_path, &root).unwrap();
        assert_eq!(name, "rubick-plugin-translate");
        assert!(root.join(&name).join("index.html").is_file());
        uninstall_plugin(&root, &name).unwrap();
    }

    #[test]
    fn install_from_zip_accepts_package_json_manifest() {
        let dir = tempfile::TempDir::new().unwrap();
        let zip_path = dir.path().join("bundle.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        fn add(zw: &mut zip::ZipWriter<fs::File>, name: &str, data: &str) {
            zw.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(zw, data.as_bytes()).unwrap();
        }
        add(&mut zw, "pkg/package.json", PKG_MANIFEST);
        add(&mut zw, "pkg/index.html", "<html></html>");
        zw.finish().unwrap();

        let root = dir.path().join("plugins");
        let name = install_from_zip(&zip_path, &root).unwrap();
        assert_eq!(name, "rubick-plugin-translate");
        assert!(root.join(&name).join("index.html").is_file());
    }

    /// Mirrors `rubick-markdown@0.0.2`: a `plugin.json` that carries
    /// `main`/`features` but no `name`/`version`, next to a complete
    /// `package.json`.
    const PARTIAL_PLUGIN_JSON: &str = r#"{
        "pluginName": "超级Markdown",
        "description": "强大的 Markdown 编辑器",
        "main": "index.html",
        "preload": "preload.js",
        "logo": "logo.png",
        "features": [{ "code": "md", "explain": "编辑器", "cmds": ["MD"] }]
    }"#;

    const FULL_PACKAGE_JSON: &str = r#"{
        "name": "rubick-markdown",
        "version": "0.0.2",
        "pluginName": "Markdown 编辑器",
        "description": "基于掘金文档风格的 markdown 编辑器",
        "pluginType": "ui",
        "main": "index.html",
        "preload": "preload.js",
        "features": [{ "code": "md", "explain": "编辑器", "cmds": ["MD"] }]
    }"#;

    #[test]
    fn read_manifest_merges_partial_plugin_json_from_package_json() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("plugin.json"), PARTIAL_PLUGIN_JSON).unwrap();
        fs::write(dir.path().join("package.json"), FULL_PACKAGE_JSON).unwrap();

        let manifest = read_manifest(dir.path()).unwrap();
        assert_eq!(manifest.name, "rubick-markdown");
        assert_eq!(manifest.version, "0.0.2");
        assert_eq!(manifest.plugin_name.as_deref(), Some("超级Markdown"));
    }

    #[test]
    fn install_from_dir_uses_package_json_name_for_partial_plugin_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let src = dir.path().join("rubick-markdown");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("plugin.json"), PARTIAL_PLUGIN_JSON).unwrap();
        fs::write(src.join("package.json"), FULL_PACKAGE_JSON).unwrap();
        fs::write(src.join("index.html"), "<html></html>").unwrap();

        let root = dir.path().join("plugins");
        let name = install_from_dir(&src, &root).unwrap();
        assert_eq!(name, "rubick-markdown");
        assert!(root.join("rubick-markdown").join("index.html").is_file());
    }

    #[test]
    fn install_from_tar_gz_uses_package_json_name_for_partial_plugin_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let tar_path = dir.path().join("markdown.tgz");
        let file = fs::File::create(&tar_path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tw = tar::Builder::new(enc);
        for (p, data) in [
            ("package/plugin.json", PARTIAL_PLUGIN_JSON),
            ("package/package.json", FULL_PACKAGE_JSON),
            ("package/index.html", "<html></html>"),
        ] {
            let mut h = tar::Header::new_gnu().clone();
            h.set_size(data.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            tw.append_data(&mut h, p, data.as_bytes()).unwrap();
        }
        let enc = tw.into_inner().unwrap();
        drop(enc);

        let root = dir.path().join("plugins");
        let name = install_from_tar_gz(&tar_path, &root).unwrap();
        assert_eq!(name, "rubick-markdown");
        assert!(root.join("rubick-markdown").join("index.html").is_file());
    }

    #[test]
    fn resolve_install_name_falls_back_to_plugin_name_then_dir() {
        let named = PluginManifest::from_bytes(PARTIAL_PLUGIN_JSON.as_bytes()).unwrap();
        assert_eq!(
            resolve_install_name(&named, Path::new("C:/tmp/staging")).unwrap(),
            "超级Markdown"
        );

        let unnamed =
            PluginManifest::from_bytes(r#"{ "main": "index.html", "features": [] }"#.as_bytes())
                .unwrap();
        assert_eq!(
            resolve_install_name(&unnamed, Path::new("C:/tmp/my-plug")).unwrap(),
            "my-plug"
        );
    }

    #[test]
    fn archive_name_rejects_staging_dir_fallback() {
        let manifest = PluginManifest::from_bytes(
            r#"{ "pluginName": "../evil", "main": "index.html", "features": [] }"#.as_bytes(),
        )
        .unwrap();
        assert!(matches!(
            archive_install_name(&manifest),
            Err(PluginMgrError::BadName(_))
        ));
    }

    #[test]
    fn origins_default_to_custom_and_round_trip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(PLUGIN_ORIGINS_FILE);
        let mut origins = PluginOrigins::load(&path).unwrap();
        assert!(!origins.is_ecosystem("rubick-markdown"));
        assert!(origins.mark_ecosystem("rubick-markdown"));
        assert!(!origins.mark_ecosystem("rubick-markdown"));
        origins.save(&path).unwrap();

        let loaded = PluginOrigins::load(&path).unwrap();
        assert!(loaded.is_ecosystem("rubick-markdown"));
        assert_eq!(loaded.origin_of("rubick-markdown"), Some(Origin::Ecosystem));
        assert_eq!(loaded.names(), vec!["rubick-markdown".to_string()]);
    }

    #[test]
    fn origins_track_official_plugins_separately() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(PLUGIN_ORIGINS_FILE);
        let mut origins = PluginOrigins::new();
        assert!(origins.mark_official("mytools-hello"));
        assert!(!origins.mark_official("mytools-hello"));
        assert!(origins.is_official("mytools-hello"));
        assert!(!origins.is_ecosystem("mytools-hello"));
        origins.save(&path).unwrap();

        let loaded = PluginOrigins::load(&path).unwrap();
        assert_eq!(loaded.origin_of("mytools-hello"), Some(Origin::Official));
    }

    #[test]
    fn origins_load_legacy_ecosystem_arrays() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(PLUGIN_ORIGINS_FILE);
        fs::write(&path, br#"["rubick-markdown","rubick-plugin-demo"]"#).unwrap();
        let loaded = PluginOrigins::load(&path).unwrap();
        assert!(loaded.is_ecosystem("rubick-markdown"));
        assert!(loaded.is_ecosystem("rubick-plugin-demo"));
        assert!(!loaded.is_official("rubick-markdown"));
    }

    #[test]
    fn origins_forget_removes_a_record() {
        let mut origins = PluginOrigins::new();
        origins.mark_ecosystem("a");
        origins.mark_ecosystem("b");
        assert!(origins.forget("a"));
        assert!(!origins.forget("a"));
        assert_eq!(origins.names(), vec!["b".to_string()]);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    fn write_pack(path: &Path, name: &str, version: &str, entry: bool) {
        let file=fs::File::create(path).unwrap();
        let encoder=flate2::write::GzEncoder::new(file,flate2::Compression::default());
        let mut builder=tar::Builder::new(encoder);
        let manifest=serde_json::json!({"name":name,"description":"test","version":version,"main":"index.html"}).to_string();
        let mut entries=vec![("package/plugin.json",manifest.as_bytes())];
        if entry {entries.push(("package/index.html",b"new version"));}
        for (name,body) in entries {
            let mut header=tar::Header::new_gnu();header.set_size(body.len() as u64);header.set_mode(0o644);header.set_cksum();
            builder.append_data(&mut header,name,body).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap();
    }
    fn installed(root: &Path) {
        let dir=root.join("demo");fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("plugin.json"),r#"{"name":"demo","description":"old","version":"0.1.0","main":"index.html"}"#).unwrap();
        fs::write(dir.join("index.html"),"old version").unwrap();
    }
    #[test]
    fn update_validated_package_preserves_external_storage() {
        let temp=tempfile::tempdir().unwrap();let root=temp.path().join("plugins");installed(&root);
        let storage=temp.path().join("storage.json");fs::write(&storage,"user data").unwrap();
        let pack=temp.path().join("update.tgz");write_pack(&pack,"demo","0.2.0",true);
        update_from_tar_gz(&pack,&root,"demo","0.1.0","0.2.0").unwrap();
        assert_eq!(fs::read_to_string(root.join("demo/index.html")).unwrap(),"new version");
        assert_eq!(fs::read_to_string(storage).unwrap(),"user data");
        assert_eq!(fs::read_dir(root).unwrap().count(),1);
    }
    #[test]
    fn bad_identity_version_entry_or_archive_never_replaces_old_plugin() {
        let temp=tempfile::tempdir().unwrap();let root=temp.path().join("plugins");installed(&root);
        let pack=temp.path().join("update.tgz");
        for (name,version,entry,from) in [("other","0.2.0",true,"0.1.0"),("demo","0.3.0",true,"0.1.0"),("demo","0.2.0",false,"0.1.0"),("demo","0.2.0",true,"0.0.9")] {
            write_pack(&pack,name,version,entry);
            assert!(update_from_tar_gz(&pack,&root,"demo",from,"0.2.0").is_err());
            assert_eq!(fs::read_to_string(root.join("demo/index.html")).unwrap(),"old version");
        }
        fs::write(&pack,"not gzip").unwrap();assert!(update_from_tar_gz(&pack,&root,"demo","0.1.0","0.2.0").is_err());
    }
    #[test]
    fn failed_activation_restores_the_old_directory() {
        let temp=tempfile::tempdir().unwrap();let dest=temp.path().join("demo");let backup=temp.path().join("backup");let candidate=temp.path().join("candidate");
        fs::create_dir(&dest).unwrap();fs::write(dest.join("index.html"),"old version").unwrap();
        let result=replace_update(&candidate,&dest,&backup,||Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied,"simulated locked directory")));
        assert!(result.is_err());assert!(!backup.exists());assert_eq!(fs::read_to_string(dest.join("index.html")).unwrap(),"old version");
    }
}
