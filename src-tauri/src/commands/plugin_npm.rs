//! npm package install channel — resolve a package on the npm registry,
//! download its tarball and install it as a MyTools plugin via the
//! shared archive installer (`.tgz` → `install_from_tar_gz`).
//!
//! This is the developer-facing install path (插件中心 → 开发者). It
//! records no special provenance: npm installs are treated as
//! `Origin::Custom` unless they match a shipped official package name.

use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use mytools_core::plugin_mgmt::is_safe_plugin_name;

const NPM_REGISTRY: &str = "https://registry.npmjs.org/";

static DL_SEQ: AtomicU64 = AtomicU64::new(0);

fn encode_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 8);
    for b in name.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn split_spec(spec: &str) -> (String, Option<String>) {
    if let Some(stripped) = spec.strip_prefix('@') {
        if let Some(pos) = stripped.find('@') {
            let name = format!("@{}", &stripped[..pos]);
            let version = &stripped[pos + 1..];
            return (name, (!version.is_empty()).then(|| version.to_string()));
        }
        return (spec.to_string(), None);
    }
    if let Some(pos) = spec.find('@') {
        let name = spec[..pos].to_string();
        let version = spec[pos + 1..].to_string();
        return (name, (!version.is_empty()).then_some(version));
    }
    (spec.to_string(), None)
}

fn resolve_tarball(name: &str, version: &Option<String>) -> Result<String, String> {
    if !is_safe_plugin_name(name) {
        return Err(format!("invalid package name: {name}"));
    }
    let url = format!("{}{}", NPM_REGISTRY, encode_name(name));
    let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
    let mut json = String::new();
    resp.into_reader()
        .take(4 * 1024 * 1024)
        .read_to_string(&mut json)
        .map_err(|e| e.to_string())?;
    resolve_tarball_from_doc(name, version, &json)
}

/// Pure helper: pick the tarball URL out of an already-fetched npm
/// registry JSON document, returning a unified "not found" error
/// when the registry returned `{"error":"Not found"}`.
fn resolve_tarball_from_doc(
    name: &str,
    version: &Option<String>,
    json: &str,
) -> Result<String, String> {
    let doc: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;

    // The npm registry returns HTTP 200 OK with body `{"error":"Not found"}`
    // for missing packages.  Detect that explicitly so the user gets a
    // single, clear error message instead of a misleading
    // "has no version" / "has no dist-tags.latest" follow-on error.
    if let Some(err) = doc.get("error").and_then(|v| v.as_str()) {
        tracing::warn!(
            "npm registry: package not found: name={} version={:?} error={}",
            name,
            version,
            err
        );
        return Err(format!(
            "package {name} not found on npm registry{}",
            version
                .as_ref()
                .map(|v| format!(" (version {v})"))
                .unwrap_or_default()
        ));
    }

    let resolve = |v: &str| {
        doc["versions"][v]["dist"]["tarball"]
            .as_str()
            .map(|s| s.to_string())
    };

    match version {
        Some(v) => {
            if let Some(t) = resolve(v).or_else(|| resolve(&format!("v{v}"))) {
                return Ok(t);
            }
            Err(format!("package {name} has no version {v}"))
        }
        None => {
            let latest = doc["dist-tags"]["latest"]
                .as_str()
                .ok_or_else(|| format!("package {name} has no dist-tags.latest"))?;
            resolve(latest).ok_or_else(|| format!("package {name}@{latest} has no tarball"))
        }
    }
}

fn download_tarball(url: &str) -> Result<PathBuf, String> {
    let resp = ureq::get(url).call().map_err(|e| e.to_string())?;
    let seq = DL_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("mytools-npm-{}-{}.tgz", std::process::id(), seq));
    let mut reader = resp.into_reader();
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    std::io::copy(&mut reader, &mut file).map_err(|e| e.to_string())?;
    Ok(path)
}

#[tauri::command]
pub async fn plugin_install_npm(
    app: tauri::AppHandle,
    registry: tauri::State<'_, crate::services::plugins::SharedPluginRegistry>,
    spec: String,
) -> Result<String, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || install_npm_owned(app, registry, spec))
        .await
        .map_err(|e| e.to_string())?
}

fn install_npm_owned(
    app: tauri::AppHandle,
    registry: crate::services::plugins::SharedPluginRegistry,
    spec: String,
) -> Result<String, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err("empty package spec".into());
    }
    let (name, version) = split_spec(spec);
    let tarball = resolve_tarball(&name, &version)?;
    let tmp = download_tarball(&tarball)?;
    // Always clean up the staged tarball, success or failure.
    let result = registry.install_pack(&tmp);
    let _ = std::fs::remove_file(&tmp);
    let name = result?;
    crate::commands::plugin::notify_plugins_changed(&app);
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::{
        download_tarball, encode_name, resolve_tarball, resolve_tarball_from_doc, split_spec,
    };

    #[test]
    fn encodes_scoped_names_for_registry_url() {
        assert_eq!(encode_name("demo"), "demo");
        assert_eq!(encode_name("@scope/pkg"), "%40scope%2Fpkg");
        assert_eq!(encode_name("a-b.c~d"), "a-b.c~d");
    }

    #[test]
    fn splits_spec_into_name_and_version() {
        assert_eq!(split_spec("demo"), ("demo".into(), None));
        assert_eq!(
            split_spec("demo@1.2.3"),
            ("demo".into(), Some("1.2.3".into()))
        );
        assert_eq!(split_spec("@scope/pkg"), ("@scope/pkg".into(), None));
        assert_eq!(
            split_spec("@scope/pkg@^2"),
            ("@scope/pkg".into(), Some("^2".into()))
        );
        assert_eq!(split_spec("demo@"), ("demo".into(), None));
    }

    /// End-to-end check against the real npm registry: resolve → download
    /// → install → scan. Covers the four real-world rubick layouts:
    ///  `rubick-plugin-demo`    (plugin.json + mixed cmds + relative logo)
    ///  `rubick-plugin-translate@1.0.2` (manifest inside package.json)
    ///  `rubick-ui-clipboard`   (package.json manifest, pluginType "system")
    ///  `rubick-markdown`       (partial plugin.json + full package.json)
    #[test]
    fn resolve_tarball_returns_unified_not_found_error() {
        // npm registry body shape for missing packages.
        let body = r#"{"error":"Not found"}"#;
        let err = resolve_tarball_from_doc("rubick-ui-wallhaven", &None, body).unwrap_err();
        assert!(
            err.contains("rubick-ui-wallhaven not found on npm registry"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn resolve_tarball_not_found_message_includes_version() {
        let body = r#"{"error":"Not found"}"#;
        let err = resolve_tarball_from_doc("rubick-ui-wallhaven", &Some("0.2.0".into()), body)
            .unwrap_err();
        assert!(err.contains("(version 0.2.0)"), "unexpected error: {err}");
    }

    #[test]
    fn resolve_tarball_extracts_tarball_from_valid_doc() {
        let body = r#"{
            "dist-tags": {"latest": "1.2.3"},
            "versions": {
                "1.2.3": {"dist": {"tarball": "https://registry.npmjs.org/x/-/x-1.2.3.tgz"}}
            }
        }"#;
        let url = resolve_tarball_from_doc("x", &None, body).expect("valid doc resolves");
        assert!(url.ends_with("x-1.2.3.tgz"));
    }

    #[test]
    fn resolve_tarball_missing_version_reports_long_request() {
        let body = r#"{
            "dist-tags": {"latest": "1.0.0"},
            "versions": {
                "1.0.0": {"dist": {"tarball": "https://x/y.tgz"}}
            }
        }"#;
        let err = resolve_tarball_from_doc("x", &Some("9.9.9".into()), body).unwrap_err();
        assert!(
            err.contains("has no version 9.9.9"),
            "unexpected error: {err}"
        );
    }

    #[test]
    #[ignore = "requires network access to registry.npmjs.org"]
    fn installs_real_rubick_plugins_from_npm() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("plugins");

        for spec in [
            "rubick-plugin-demo",
            "rubick-plugin-translate@1.0.2",
            "rubick-ui-clipboard@1.0.4",
            "rubick-markdown@0.0.2",
        ] {
            let (name, version) = split_spec(spec);
            let tarball = resolve_tarball(&name, &version).unwrap();
            let tmp = download_tarball(&tarball).unwrap();
            let installed = mytools_core::install_from_tar_gz(&tmp, &root).unwrap();
            let _ = std::fs::remove_file(&tmp);
            assert_eq!(installed, name);

            let plugins = mytools_core::scan_dir(&root).unwrap();
            let p = plugins
                .iter()
                .find(|p| p.manifest.name == name)
                .unwrap_or_else(|| panic!("{name} not found after install"));
            assert!(
                p.main_url.ends_with(".html"),
                "main_url for {name}: {}",
                p.main_url
            );
            let dir_on_disk = dir.path().join("plugins").join(&name);
            assert!(dir_on_disk.join("index.html").is_file());
        }
    }
}
