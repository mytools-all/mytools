//! Plugin loader — scan a directory tree for `plugin.json` files and
//! surface the parsed manifests. v0.1 reads synchronously; v0.3
//! caches the result and watches for changes via `notify`.

use crate::capability::{Origin, HOST_API_VERSION};
use crate::manifest::{ManifestError, PluginManifest, PKG_MANIFEST_FILE};
use crate::plugin_mgmt::first_safe_name;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub dir: String,
    pub main_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preload_url: Option<String>,
    /// Display icon: native relative PNG as data URI, Rubick URL as-is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub enabled: bool,
    /// Effective origin decided by the host (official / ecosystem / custom).
    pub origin: Origin,
    /// Ecosystem manifest dialect this plugin was written against, if any.
    /// This is *dialect identity*, not a privilege: it decides which
    /// compatibility adapter grades the plugin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecosystem: Option<String>,
    /// Capabilities the plugin declares it needs.
    pub declared_capabilities: Vec<String>,
    /// Why this plugin cannot run on this host, if it cannot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incompatible: Option<String>,
}

impl LoadedPlugin {
    pub fn key(&self) -> String {
        self.manifest.name.clone()
    }

    pub fn is_system(&self) -> bool {
        self.manifest.is_system_plugin()
    }

    /// True when the plugin declares a capability set (and is therefore
    /// restricted to it).
    pub fn restricts_capabilities(&self) -> bool {
        !self.declared_capabilities.is_empty()
    }

    pub fn declares_capability(&self, capability: &str) -> bool {
        self.declared_capabilities
            .iter()
            .any(|c| c.eq_ignore_ascii_case(capability))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("manifest: {0}")]
    Manifest(#[from] ManifestError),
    #[error("walk: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("plugin {dir}: {source}")]
    Plugin {
        dir: String,
        #[source]
        source: ManifestError,
    },
}

pub fn scan_dir(root: &Path) -> Result<Vec<LoadedPlugin>, LoaderError> {
    scan_dir_enabled(root, &|_| true)
}

/// Scan a directory tree for `plugin.json` files and surface the
/// parsed manifests. `is_enabled` lets the caller overlay the
/// per-plugin enabled state so disabled plugins still appear (with
/// `enabled = false`) in the manager UI while staying loaded.
///
/// Rubick plugins published to npm may only carry the manifest inside
/// `package.json` (no `plugin.json`); those are accepted too, but a
/// directory with a real `plugin.json` always wins.
pub fn scan_dir_enabled(
    root: &Path,
    is_enabled: &dyn Fn(&str) -> bool,
) -> Result<Vec<LoadedPlugin>, LoaderError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(root)
        .max_depth(3)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(crate::plugin_mgmt::INSTALL_STAGING_PREFIX)
        })
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() != "plugin.json" && entry.file_name() != PKG_MANIFEST_FILE {
            continue;
        }
        let dir = entry
            .path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| root.to_path_buf());
        match entry.file_name().to_str() {
            Some("plugin.json") => match PluginManifest::from_path(entry.path()) {
                Ok(manifest) => match ensure_plugin_name(manifest, &dir) {
                    Some(manifest) => {
                        let origin = detect_origin(&manifest, false);
                        out.push(make_loaded(dir, manifest, origin, is_enabled));
                    }
                    None => eprintln!(
                        "[plugin_loader] skip plugin at {}: no usable name",
                        dir.display()
                    ),
                },
                Err(e) => {
                    eprintln!("[plugin_loader] skip plugin at {}: {}", dir.display(), e);
                }
            },
            Some(PKG_MANIFEST_FILE) => {
                // package.json is only a manifest when the dir has no
                // plugin.json and the fields look like a rubick plugin.
                if dir.join("plugin.json").is_file() {
                    continue;
                }
                if let Some(manifest) = PluginManifest::from_package_json_path(entry.path()) {
                    if let Some(manifest) = ensure_plugin_name(manifest, &dir) {
                        let origin = detect_origin(&manifest, true);
                        out.push(make_loaded(dir, manifest, origin, is_enabled));
                    }
                }
            }
            _ => {}
        }
    }
    out.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(out)
}

/// Ensure the manifest carries a usable identifier so it can be keyed
/// and toggled. Missing `name` is recovered from a sibling
/// `package.json`, then `pluginName`, then the directory name.
fn ensure_plugin_name(mut manifest: PluginManifest, dir: &Path) -> Option<PluginManifest> {
    manifest.merge_package_json_identity(dir);
    if !manifest.name.trim().is_empty() {
        return Some(manifest);
    }
    let plugin_name = manifest.plugin_name.as_deref().unwrap_or("");
    let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    manifest.name = first_safe_name([plugin_name, dir_name])?;
    Some(manifest)
}

/// Detect the origin of a scanned plugin.
///
/// Origin is decided by **provenance**, never by manifest content: a
/// plugin must not be able to raise its own privileges by declaring
/// `origin` or by wearing ecosystem-specific fields. Plugins found in the
/// user's plugin directory are therefore `custom` until an install path
/// assigns a stronger origin (the ecosystem adapter, or the official
/// plugin flow). A manifest may still *restrict* itself to `custom`.
pub fn detect_origin(manifest: &PluginManifest, _from_package_json: bool) -> Origin {
    manifest.effective_origin(Origin::Custom)
}

/// Detect the manifest dialect of a plugin from its content. Dialect
/// identity is content-based (an ecosystem plugin is recognisable by its
/// manifest fields); it never grants privileges — origin stays a
/// provenance decision (see [`detect_origin`]).
pub fn detect_ecosystem(manifest: &PluginManifest) -> Option<String> {
    if manifest.plugin_type.is_some() || manifest.plugin_name.is_some() {
        Some("rubick".to_string())
    } else {
        None
    }
}

fn make_loaded(
    dir: PathBuf,
    manifest: PluginManifest,
    origin: Origin,
    is_enabled: &dyn Fn(&str) -> bool,
) -> LoadedPlugin {
    let main_rel = manifest.main.clone();
    let main_url = plugin_main_url(&dir, &main_rel);
    let preload_url = manifest
        .preload
        .as_deref()
        .map(|p| plugin_main_url(&dir, p));
    let icon = resolve_icon(&dir, &manifest);
    let key = manifest.name.clone();
    let declared_capabilities = manifest.declared_capabilities().to_vec();
    let incompatible = manifest.incompatibility(HOST_API_VERSION);
    let ecosystem = detect_ecosystem(&manifest);
    LoadedPlugin {
        manifest,
        dir: dir.to_string_lossy().into_owned(),
        main_url,
        preload_url,
        icon,
        enabled: is_enabled(&key),
        origin,
        ecosystem,
        declared_capabilities,
        incompatible,
    }
}

/// 512 KB cap — icons are small; anything larger is not worth
/// base64-ing into a UI string.
const ICON_MAX_BYTES: u64 = 512 * 1024;

/// Resolve the display icon for a plugin:
///  - native `icon` (relative path) → `data:` URI if the file loads;
///  - otherwise Rubick `logo`: http(s)/data as-is, relative path
///    resolved against the plugin dir into a `data:` URI.
fn resolve_icon(dir: &Path, manifest: &PluginManifest) -> Option<String> {
    if let Some(rel) = manifest.icon.as_deref() {
        if let Some(uri) = file_as_data_uri(dir, rel) {
            return Some(uri);
        }
    }
    let logo = manifest.logo.as_deref()?;
    if logo.starts_with("http://") || logo.starts_with("https://") || logo.starts_with("data:") {
        return Some(logo.to_string());
    }
    file_as_data_uri(dir, logo)
}

fn file_as_data_uri(dir: &Path, rel: &str) -> Option<String> {
    let path = dir.join(rel);
    let meta = fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > ICON_MAX_BYTES {
        return None;
    }
    let mime = mime_from_extension(rel)?;
    let bytes = fs::read(&path).ok()?;
    Some(format!("data:{};base64,{}", mime, base64_encode(&bytes)))
}

fn mime_from_extension(name: &str) -> Option<&'static str> {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "svg" => Some("image/svg+xml"),
        "webp" => Some("image/webp"),
        "ico" => Some("image/x-icon"),
        _ => None,
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18 & 0x3f) as usize] as char);
        out.push(TABLE[(n >> 12 & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(n >> 6 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn plugin_main_url(dir: &Path, main: &str) -> String {
    let main_path = dir.join(main);
    let normalized = main_path.to_string_lossy().replace('\\', "/");
    if normalized.starts_with('/') {
        format!("file://{}", normalized)
    } else {
        format!("file:///{}", normalized)
    }
}

pub fn find_plugins_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("MYTOOLS_PLUGINS_DIR") {
        return PathBuf::from(dir);
    }
    let candidates = plugin_root_candidates(
        std::env::current_dir().ok().as_deref(),
        std::env::current_exe().ok().as_deref(),
    );
    pick_plugins_root(candidates)
}

fn plugin_root_candidates(cwd: Option<&Path>, exe: Option<&Path>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(exe) = exe.and_then(|e| e.parent()) {
        out.push(exe.join("plugins"));
    }
    if let Some(cwd) = cwd {
        if let Some(project) = cwd.parent() {
            out.push(project.join("plugins"));
        }
        out.push(cwd.join("plugins"));
    }
    out
}

fn pick_plugins_root(candidates: Vec<PathBuf>) -> PathBuf {
    candidates
        .into_iter()
        .find(|p| p.is_dir())
        .unwrap_or_else(|| PathBuf::from("plugins"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn scan_returns_empty_for_missing_dir() {
        let dir = TempDir::new().unwrap();
        let ghost = dir.path().join("does-not-exist");
        let plugins = scan_dir(&ghost).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn scan_ignores_uncommitted_archive_staging() {
        let root = TempDir::new().unwrap();
        for name in ["ready", ".mytools-install-pending"] {
            let dir = root.path().join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("plugin.json"),
                format!(r#"{{"name":"{name}","version":"0.1.0","main":"index.html"}}"#),
            )
            .unwrap();
            fs::write(dir.join("index.html"), "<html></html>").unwrap();
        }
        let plugins = scan_dir(root.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.name, "ready");
    }

    #[test]
    fn native_plugin_is_custom_origin() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("hello");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name":"hello","version":"1.0.0","main":"index.html",
                "features":[{"code":"run","explain":"run"}]}"#,
        )
        .unwrap();
        let plugins = scan_dir(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].origin, Origin::Custom);
        assert!(plugins[0].incompatible.is_none());
        assert!(plugins[0].declared_capabilities.is_empty());
    }

    #[test]
    fn ecosystem_fields_do_not_grant_a_stronger_origin() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("rubick-demo");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name":"rubick-demo","version":"1.0.0","main":"index.html",
                "pluginType":"ui",
                "features":[{"code":"run","explain":"run"}]}"#,
        )
        .unwrap();
        let plugins = scan_dir(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(
            plugins[0].origin,
            Origin::Custom,
            "origin comes from provenance, not from manifest fields"
        );
    }

    #[test]
    fn manifest_dialect_is_detected_from_fields() {
        let dir = TempDir::new().unwrap();
        let native = dir.path().join("hello");
        fs::create_dir_all(&native).unwrap();
        fs::write(
            native.join("plugin.json"),
            r#"{"name":"hello","version":"1.0.0","main":"index.html"}"#,
        )
        .unwrap();
        let rubick = dir.path().join("rubick-demo");
        fs::create_dir_all(&rubick).unwrap();
        fs::write(
            rubick.join("plugin.json"),
            r#"{"name":"rubick-demo","pluginName":"Demo","version":"1.0.0",
                "main":"index.html","pluginType":"ui"}"#,
        )
        .unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        let hello = plugins.iter().find(|p| p.manifest.name == "hello").unwrap();
        let demo = plugins
            .iter()
            .find(|p| p.manifest.name == "rubick-demo")
            .unwrap();
        assert_eq!(hello.ecosystem, None);
        assert_eq!(demo.ecosystem.as_deref(), Some("rubick"));
        assert_eq!(
            demo.origin,
            Origin::Custom,
            "dialect identity must not change origin"
        );
    }

    #[test]
    fn declared_capabilities_are_exposed() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("hello");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name":"hello","version":"1.0.0","main":"index.html",
                "capabilities":["clipboard.write"],
                "features":[{"code":"run","explain":"run"}]}"#,
        )
        .unwrap();
        let plugins = scan_dir(dir.path()).unwrap();
        assert!(plugins[0].restricts_capabilities());
        assert!(plugins[0].declares_capability("clipboard.write"));
        assert!(!plugins[0].declares_capability("fs.read"));
    }

    #[test]
    fn plugin_requiring_a_newer_api_is_incompatible() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("future");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name":"future","version":"1.0.0","main":"index.html",
                "apiVersion":99,
                "features":[{"code":"run","explain":"run"}]}"#,
        )
        .unwrap();
        let plugins = scan_dir(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        let reason = plugins[0].incompatible.as_deref().expect("incompatible");
        assert!(reason.contains("v99"), "{reason}");
    }

    #[test]
    fn scan_picks_up_valid_manifest() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("hello");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "name": "hello",
                "description": "demo",
                "version": "0.1.0",
                "main": "index.html",
                "features": [
                    { "code": "x", "explain": "X", "cmds": ["x"] }
                ]
            }"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.name, "hello");
        assert!(plugins[0].main_url.starts_with("file:///"));
    }

    #[test]
    fn scan_skips_invalid_manifest_without_aborting() {
        let dir = TempDir::new().unwrap();
        let good = dir.path().join("good");
        fs::create_dir_all(&good).unwrap();
        fs::write(
            good.join("plugin.json"),
            r#"{
                "name": "good",
                "description": "x",
                "version": "0.0.1",
                "main": "index.html",
                "features": []
            }"#,
        )
        .unwrap();

        let bad = dir.path().join("bad");
        fs::create_dir_all(&bad).unwrap();
        fs::write(bad.join("plugin.json"), "{not json").unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.name, "good");
    }

    #[test]
    fn scan_respects_max_depth() {
        let dir = TempDir::new().unwrap();
        // 4 levels deep — should be ignored (max_depth = 3)
        let deep = dir.path().join("a").join("b").join("c").join("d");
        fs::create_dir_all(&deep).unwrap();
        fs::write(
            deep.join("plugin.json"),
            r#"{"name":"deep","description":"x","version":"0.0.1","main":"x.html","features":[]}"#,
        )
        .unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn candidates_ordered_exe_then_project_then_cwd() {
        let exe = Path::new("C:/app/bin/mytools.exe");
        let cwd = Path::new("C:/app/src-tauri");
        let got = plugin_root_candidates(Some(cwd), Some(exe));
        assert_eq!(
            got,
            vec![
                PathBuf::from("C:/app/bin/plugins"),
                PathBuf::from("C:/app/plugins"),
                PathBuf::from("C:/app/src-tauri/plugins")
            ]
        );
    }

    #[test]
    fn candidates_without_exe_fallback_to_project_and_cwd() {
        let cwd = Path::new("C:/app/src-tauri");
        let got = plugin_root_candidates(Some(cwd), None);
        assert_eq!(
            got,
            vec![
                PathBuf::from("C:/app/plugins"),
                PathBuf::from("C:/app/src-tauri/plugins")
            ]
        );
    }

    #[test]
    fn pick_prefers_first_existing_dir() {
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("plugins");
        fs::create_dir_all(&project).unwrap();

        let got = pick_plugins_root(vec![
            PathBuf::from("/nowhere/plugins"),
            project.clone(),
            PathBuf::from("/nowhere/cwd/plugins"),
        ]);
        assert_eq!(got, project);
    }

    #[test]
    fn pick_falls_back_to_default_when_none_exist() {
        let got = pick_plugins_root(vec![PathBuf::from("/nowhere/a/plugins")]);
        assert_eq!(got, PathBuf::from("plugins"));
    }

    #[test]
    fn scan_resolves_preload_url_native_icon_and_enabled() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("with-meta");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "name": "with-meta",
                "pluginName": "With Meta",
                "description": "x",
                "version": "0.1.0",
                "main": "index.html",
                "preload": "preload.js",
                "icon": "icon.png"
            }"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();
        fs::write(plugin_dir.join("preload.js"), "window.k = 1").unwrap();
        // 1x1 red PNG
        let png: Vec<u8> = vec![
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0xbc, 0xbb, 0x6a,
            0x47, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        fs::write(plugin_dir.join("icon.png"), &png).unwrap();

        let plugins = scan_dir_enabled(dir.path(), &|name| name != "with-meta").unwrap();
        assert_eq!(plugins.len(), 1);
        let p = &plugins[0];
        assert!(p.main_url.ends_with("/index.html"));
        assert!(p.preload_url.as_deref().unwrap().ends_with("/preload.js"));
        assert!(!p.enabled);
        let icon = p.icon.as_deref().unwrap();
        assert!(icon.starts_with("data:image/png;base64,"), "{icon}");
    }

    #[test]
    fn native_icon_missing_falls_back_to_logo() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("logo-only");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "name": "logo-only",
                "description": "x",
                "version": "0.1.0",
                "main": "index.html",
                "icon": "missing.png",
                "logo": "data:image/png;base64,QUJD"
            }"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        assert_eq!(
            plugins[0].icon.as_deref(),
            Some("data:image/png;base64,QUJD")
        );
    }

    #[test]
    fn serializes_loaded_plugin_to_camel_case() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("ser");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{ "name": "ser", "pluginName": "Ser", "description": "x", "version": "0.1.0", "main": "index.html" }"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        let json = serde_json::to_value(&plugins[0]).unwrap();
        assert_eq!(json["mainUrl"], plugins[0].main_url);
        assert!(json.get("main_url").is_none());
        assert_eq!(json["enabled"], true);
    }

    #[test]
    fn scan_accepts_package_json_manifest_and_prefers_plugin_json() {
        let dir = TempDir::new().unwrap();

        let pkg_only = dir.path().join("rubick-translate-style");
        fs::create_dir_all(&pkg_only).unwrap();
        fs::write(
            pkg_only.join("package.json"),
            r#"{
                "name": "rubick-translate-style",
                "pluginName": "翻译",
                "description": "翻译",
                "version": "1.0.0",
                "pluginType": "ui",
                "main": "index.html",
                "preload": "preload.js",
                "features": [{ "code": "index", "explain": "翻译", "cmds": ["fy"] }]
            }"#,
        )
        .unwrap();
        fs::write(pkg_only.join("index.html"), "<html></html>").unwrap();
        fs::write(pkg_only.join("preload.js"), "window.k = 1").unwrap();

        // plain package.json without rubick fields must NOT load
        let plain = dir.path().join("left-pad");
        fs::create_dir_all(&plain).unwrap();
        fs::write(
            plain.join("package.json"),
            r#"{ "name": "left-pad", "version": "1.3.0", "main": "index.js" }"#,
        )
        .unwrap();

        // plugin.json wins when both are present
        let both = dir.path().join("both");
        fs::create_dir_all(&both).unwrap();
        fs::write(
            both.join("plugin.json"),
            r#"{ "name": "both", "description": "x", "version": "0.1.0", "main": "index.html" }"#,
        )
        .unwrap();
        fs::write(
            both.join("package.json"),
            r#"{ "name": "both-from-pkg", "version": "9", "main": "index.html", "features": [{ "code": "a", "explain": "b", "cmds": ["c"] }] }"#,
        )
        .unwrap();
        fs::write(both.join("index.html"), "<html></html>").unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        let names: Vec<&str> = plugins.iter().map(|p| p.manifest.name.as_str()).collect();
        assert_eq!(names, vec!["both", "rubick-translate-style"]);
        let pkg = plugins
            .iter()
            .find(|p| p.manifest.name == "rubick-translate-style")
            .unwrap();
        assert_eq!(pkg.manifest.display_name(), "翻译");
        assert!(pkg.main_url.ends_with("/index.html"));
        assert!(pkg.preload_url.as_deref().unwrap().ends_with("/preload.js"));
    }

    #[test]
    fn scan_merges_partial_plugin_json_from_package_json() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("rubick-markdown");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "pluginName": "超级Markdown",
                "description": "强大的 Markdown 编辑器",
                "main": "index.html",
                "logo": "logo.png",
                "features": [{ "code": "md", "explain": "编辑器", "cmds": ["MD"] }]
            }"#,
        )
        .unwrap();
        fs::write(
            plugin_dir.join("package.json"),
            r#"{
                "name": "rubick-markdown",
                "version": "0.0.2",
                "main": "index.html",
                "features": [{ "code": "md", "explain": "编辑器", "cmds": ["MD"] }]
            }"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();
        fs::write(plugin_dir.join("logo.png"), b"fake-png").unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.name, "rubick-markdown");
        assert_eq!(plugins[0].manifest.display_name(), "超级Markdown");
        assert_eq!(plugins[0].manifest.version, "0.0.2");
        assert!(plugins[0]
            .icon
            .as_deref()
            .unwrap()
            .starts_with("data:image/png;base64,"));
    }

    #[test]
    fn scan_falls_back_to_dir_name_when_name_missing() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("orphan-plug");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{ "main": "index.html", "features": [] }"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.name, "orphan-plug");
    }

    #[test]
    fn relative_logo_resolves_to_data_uri() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("rel-logo");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "name": "rel-logo",
                "description": "x",
                "version": "0.1.0",
                "main": "index.html",
                "logo": "logo.png"
            }"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();
        fs::write(plugin_dir.join("logo.png"), b"fake-png").unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        let icon = plugins[0].icon.as_deref().unwrap();
        assert!(icon.starts_with("data:image/png;base64,"), "{icon}");
    }

    #[test]
    fn missing_relative_logo_falls_back_to_none() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("no-logo");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "name": "no-logo",
                "description": "x",
                "version": "0.1.0",
                "main": "index.html",
                "logo": "missing.png"
            }"#,
        )
        .unwrap();
        fs::write(plugin_dir.join("index.html"), "<html></html>").unwrap();

        let plugins = scan_dir(dir.path()).unwrap();
        assert_eq!(plugins[0].icon, None);
    }
}
