//! Plugin manifest parsing.
//!
//! Supports both the native MyTools schema and the Rubick schema
//! (which is reused by the ecosystem). Parsing errors are surfaced to
//! the loader so a single malformed plugin does not abort the scan.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::capability::Origin;

/// Rubick/npm plugins may ship the manifest inside `package.json` when
/// there is no `plugin.json` (see `PluginManifest::from_package_json_*`).
pub const PKG_MANIFEST_FILE: &str = "package.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    /// Rubick plugins may omit the identifier in `plugin.json`; it is
    /// then recovered from the sibling `package.json` (see
    /// `merge_package_json_identity`). `main` stays required so `{}`
    /// is still rejected as a manifest.
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default)]
    pub version: String,
    pub main: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    /// Native icon path relative to the plugin dir (docs/04 schema).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Rubick preload script, executed before `main` inside the iframe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preload: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_page: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_search: Option<String>,
    #[serde(default)]
    pub features: Vec<PluginFeature>,
    /// Minimum host API version this plugin needs. Absent means "the
    /// current major version".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<u32>,
    /// Declared source hint (`official` / `ecosystem` / `custom`).
    /// Informational only: the host decides the effective origin and a
    /// manifest may never escalate its own privileges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Capabilities the plugin declares it needs. An empty list means
    /// "no restriction" (legacy plugins).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
    /// Explicit elevation requests, e.g. `{ "exec": true }`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub permissions: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFeature {
    pub code: String,
    pub explain: String,
    /// Command aliases. Rubick plugins mix plain strings with search
    /// matrix objects (`{"type": "img", "label": "…"}`); we keep the
    /// strings and use the object `label` as an extra alias.
    #[serde(default, deserialize_with = "deserialize_cmds")]
    pub cmds: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

fn deserialize_cmds<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let entries: Vec<serde_json::Value> = Deserialize::deserialize(de)?;
    Ok(entries
        .into_iter()
        .filter_map(|v| {
            v.as_str()
                .map(String::from)
                .or_else(|| v.get("label").and_then(|l| l.as_str()).map(String::from))
        })
        .collect())
}

impl PluginManifest {
    /// Parse a manifest from a JSON byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Parse a manifest from a file on disk.
    pub fn from_path(path: &Path) -> Result<Self, ManifestError> {
        let bytes = std::fs::read(path).map_err(ManifestError::Io)?;
        Self::from_bytes(&bytes).map_err(ManifestError::Parse)
    }

    /// Parse a `package.json`-embedded manifest. Valid only when the
    /// package actually declares rubick plugin fields: `main` plus at
    /// least one `feature`. Returns `None` for ordinary npm packages.
    pub fn from_package_json_bytes(bytes: &[u8]) -> Option<Self> {
        let m = Self::from_bytes(bytes).ok()?;
        if m.main.trim().is_empty() || m.features.is_empty() {
            return None;
        }
        Some(m)
    }

    /// `package.json` manifest on disk (see `from_package_json_bytes`).
    pub fn from_package_json_path(path: &Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        Self::from_package_json_bytes(&bytes)
    }

    /// Display name (Rubick uses `pluginName`, native uses `name`).
    pub fn display_name(&self) -> &str {
        self.plugin_name.as_deref().unwrap_or(&self.name)
    }

    /// Fill empty identity fields from the directory's `package.json`.
    ///
    /// Rubick plugins such as `rubick-markdown` ship a partial
    /// `plugin.json` (no `name`/`version`) next to a complete
    /// `package.json`; merging keeps them installable and loadable.
    /// Existing non-empty values always win.
    pub fn merge_package_json_identity(&mut self, dir: &Path) {
        let Ok(bytes) = std::fs::read(dir.join(PKG_MANIFEST_FILE)) else {
            return;
        };
        let Ok(pkg) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return;
        };
        let str_field = |key: &str| {
            pkg.get(key)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        if self.name.trim().is_empty() {
            if let Some(name) = str_field("name") {
                self.name = name;
            }
        }
        if self.version.trim().is_empty() {
            if let Some(version) = str_field("version") {
                self.version = version;
            }
        }
        if self.description.trim().is_empty() {
            if let Some(description) = str_field("description") {
                self.description = description;
            }
        }
    }

    /// True if this is a Rubick-style "system" plugin (main-process level;
    /// the compat layer reports it as incompatible, see
    /// docs/02-architecture.md §5.3).
    pub fn is_system_plugin(&self) -> bool {
        self.plugin_type.as_deref() == Some("system")
    }

    /// Capabilities the plugin declares. Empty means "no restriction".
    pub fn declared_capabilities(&self) -> &[String] {
        &self.capabilities
    }

    /// True when the plugin asks for an elevated permission level.
    pub fn requests_permission(&self, level: &str) -> bool {
        match self.permissions.get(level) {
            Some(serde_json::Value::Bool(true)) => true,
            Some(serde_json::Value::String(s)) => s.eq_ignore_ascii_case("true"),
            _ => false,
        }
    }

    /// The effective origin of this plugin.
    ///
    /// The host decides; a manifest may only *restrict* itself (declare
    /// `custom`), never claim a more privileged origin.
    pub fn effective_origin(&self, detected: Origin) -> Origin {
        match self
            .origin
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("custom") => Origin::Custom,
            _ => detected,
        }
    }

    /// Reason why this plugin cannot run on this host, if any.
    pub fn incompatibility(&self, host_api_version: u32) -> Option<String> {
        if let Some(required) = self.api_version {
            if required > host_api_version {
                return Some(format!(
                    "needs host API v{required}, this build provides v{host_api_version}"
                ));
            }
        }
        if self.is_system_plugin() {
            return Some("system plugin (main-process level)".to_string());
        }
        None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] serde_json::Error),
    /// The file parsed as JSON but does not describe a plugin.
    #[error("not a plugin manifest: {reason}")]
    NotAPluginManifest { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    const NATIVE: &str = r#"{
        "name": "mytools-plugin-demo",
        "description": "Demo",
        "version": "0.1.0",
        "main": "index.html",
        "features": [
            { "code": "go", "explain": "Run", "cmds": ["demo"] }
        ]
    }"#;

    const RUBICK: &str = r#"{
        "name": "rubick-plugin-demo",
        "pluginName": "Rubick Demo",
        "description": "Demo",
        "version": "0.0.1",
        "main": "index.html",
        "preload": "preload.js",
        "homePage": "https://github.com/example/demo",
        "pluginType": "ui",
        "features": [
            { "code": "index", "explain": "Test", "cmds": ["demo", "测试"] }
        ]
    }"#;

    const NATIVE_WITH_ICON: &str = r#"{
        "name": "mytools-plugin-icon",
        "description": "Icon Demo",
        "version": "0.1.0",
        "main": "index.html",
        "icon": "icon.png",
        "features": [
            { "code": "go", "explain": "Run", "cmds": ["demo"] }
        ]
    }"#;

    const RUBICK_SYSTEM: &str = r#"{
        "name": "rubick-system-plugin-demo",
        "pluginName": "System Plugin",
        "description": "system",
        "version": "0.0.0",
        "main": "index.js",
        "pluginType": "system"
    }"#;

    const RUBICK_MIXED_CMDS: &str = r#"{
        "name": "rubick-plugin-demo",
        "pluginName": "Demo",
        "description": "demo",
        "version": "0.0.1",
        "main": "index.html",
        "features": [
            {
                "code": "hello",
                "explain": "say hi",
                "cmds": ["hello", "你好", { "type": "img", "label": "上传截图" }]
            }
        ]
    }"#;

    const PKG_AS_MANIFEST: &str = r#"{
        "name": "rubick-plugin-translate",
        "pluginName": "翻译",
        "description": "translate",
        "version": "1.0.0",
        "pluginType": "ui",
        "logo": "https://example.com/logo.ico",
        "features": [{ "code": "index", "explain": "翻译", "cmds": ["fy"] }],
        "main": "index.html",
        "preload": "preload.js",
        "homePage": "https://example.com"
    }"#;

    #[test]
    fn parses_native_manifest() {
        let m = PluginManifest::from_bytes(NATIVE.as_bytes()).unwrap();
        assert_eq!(m.name, "mytools-plugin-demo");
        assert_eq!(m.features.len(), 1);
        assert_eq!(m.features[0].cmds[0], "demo");
        assert!(!m.is_system_plugin());
    }

    #[test]
    fn parses_rubick_manifest() {
        let m = PluginManifest::from_bytes(RUBICK.as_bytes()).unwrap();
        assert_eq!(m.display_name(), "Rubick Demo");
        assert_eq!(m.features[0].cmds[1], "测试");
        assert_eq!(m.preload.as_deref(), Some("preload.js"));
        assert_eq!(
            m.home_page.as_deref(),
            Some("https://github.com/example/demo")
        );
        assert!(!m.is_system_plugin());
    }

    #[test]
    fn parses_native_icon() {
        let m = PluginManifest::from_bytes(NATIVE_WITH_ICON.as_bytes()).unwrap();
        assert_eq!(m.icon.as_deref(), Some("icon.png"));
        assert!(m.preload.is_none());
    }

    #[test]
    fn parses_v1_declaration_fields() {
        let raw = r#"{
            "name": "demo",
            "version": "1.0.0",
            "main": "index.html",
            "apiVersion": 1,
            "origin": "custom",
            "capabilities": ["clipboard.write", "shell.open"],
            "permissions": { "exec": true, "fs": "no" },
            "features": [{ "code": "run", "explain": "run" }]
        }"#;
        let m = PluginManifest::from_bytes(raw.as_bytes()).unwrap();
        assert_eq!(m.api_version, Some(1));
        assert_eq!(m.origin.as_deref(), Some("custom"));
        assert_eq!(
            m.declared_capabilities(),
            &["clipboard.write".to_string(), "shell.open".to_string()]
        );
        assert!(m.requests_permission("exec"));
        assert!(!m.requests_permission("fs"));
    }

    #[test]
    fn legacy_manifests_have_no_declaration_fields() {
        let m = PluginManifest::from_bytes(NATIVE.as_bytes()).unwrap();
        assert_eq!(m.api_version, None);
        assert_eq!(m.origin, None);
        assert!(m.declared_capabilities().is_empty());
        assert!(m.permissions.is_empty());
        assert!(!m.requests_permission("exec"));
        assert!(m.incompatibility(1).is_none());
    }

    #[test]
    fn manifest_can_only_restrict_its_own_origin() {
        let mut m = PluginManifest::from_bytes(NATIVE.as_bytes()).unwrap();
        assert_eq!(m.effective_origin(Origin::Ecosystem), Origin::Ecosystem);
        m.origin = Some("custom".to_string());
        assert_eq!(m.effective_origin(Origin::Ecosystem), Origin::Custom);
        m.origin = Some("official".to_string());
        assert_eq!(
            m.effective_origin(Origin::Ecosystem),
            Origin::Ecosystem,
            "a manifest must not escalate its origin"
        );
    }

    #[test]
    fn newer_api_version_is_incompatible() {
        let raw = r#"{
            "name": "demo",
            "version": "1.0.0",
            "main": "index.html",
            "apiVersion": 9,
            "features": [{ "code": "run", "explain": "run" }]
        }"#;
        let m = PluginManifest::from_bytes(raw.as_bytes()).unwrap();
        let reason = m.incompatibility(1).expect("must be incompatible");
        assert!(reason.contains("v9"), "{reason}");
        assert!(m.incompatibility(9).is_none());
    }

    #[test]
    fn system_plugins_are_incompatible() {
        let raw = r#"{
            "name": "demo",
            "version": "1.0.0",
            "main": "index.html",
            "pluginType": "system",
            "features": [{ "code": "run", "explain": "run" }]
        }"#;
        let m = PluginManifest::from_bytes(raw.as_bytes()).unwrap();
        assert!(m.incompatibility(1).is_some());
    }

    #[test]
    fn serializes_to_camel_case_for_frontend() {
        let m = PluginManifest::from_bytes(RUBICK.as_bytes()).unwrap();
        let json = serde_json::to_value(&m).unwrap();
        assert_eq!(json["pluginName"], "Rubick Demo");
        assert_eq!(json["pluginType"], "ui");
        assert_eq!(json["homePage"], "https://github.com/example/demo");
        assert!(
            json.get("plugin_name").is_none(),
            "must not leak snake_case"
        );
    }

    #[test]
    fn detects_system_plugin() {
        let m = PluginManifest::from_bytes(RUBICK_SYSTEM.as_bytes()).unwrap();
        assert!(m.is_system_plugin());
    }

    #[test]
    fn missing_required_field_errors() {
        let bad = r#"{ "name": "x" }"#;
        assert!(PluginManifest::from_bytes(bad.as_bytes()).is_err());
    }

    #[test]
    fn mixed_cmds_keep_strings_and_labels() {
        let m = PluginManifest::from_bytes(RUBICK_MIXED_CMDS.as_bytes()).unwrap();
        assert_eq!(
            m.features[0].cmds,
            vec![
                "hello".to_string(),
                "你好".to_string(),
                "上传截图".to_string()
            ]
        );
    }

    #[test]
    fn package_json_with_features_is_a_manifest() {
        let m = PluginManifest::from_package_json_bytes(PKG_AS_MANIFEST.as_bytes()).unwrap();
        assert_eq!(m.name, "rubick-plugin-translate");
        assert_eq!(m.display_name(), "翻译");
        assert_eq!(m.features[0].cmds, vec!["fy".to_string()]);
        assert_eq!(m.logo.as_deref(), Some("https://example.com/logo.ico"));
    }

    #[test]
    fn plain_package_json_is_rejected() {
        let plain = r#"{ "name": "left-pad", "version": "1.3.0", "main": "index.js" }"#;
        assert!(PluginManifest::from_package_json_bytes(plain.as_bytes()).is_none());
    }
}
