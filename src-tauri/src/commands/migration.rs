//! One-time migration from the legacy Tauri identifier dir
//! (`%APPDATA%\com.mytools.app`) to Layout B.
//!
//! Every user-visible file used to live in the identifier dir; now config
//! files live in `%APPDATA%\MyTools` and data in
//! `%LOCALAPPDATA%\MyTools\data`. The legacy directory is emptied (and
//! removed when completely empty); the WebView2 profile
//! (`%LOCALAPPDATA%\com.mytools.app`) is never touched. Success is marked
//! by `layout-v2.json` in the new config root so the move only runs once;
//! failures are logged and retried on the next start.

use std::path::Path;

use serde_json::json;

pub const MARKER_FILE: &str = "layout-v2.json";

/// Marker for the official-tools pivot (v3): removes ecosystem plugins,
/// renames the plugin KV store file and drops store caches. Runs once.
pub const MARKER_FILE_V3: &str = "layout-v3.json";

/// Name of the plugin KV store after the Rubick ecosystem was dropped
/// (formerly `rubick-store.json`); see `commands/capability.rs`.
pub const PLUGIN_STORE_FILE: &str = "plugin-store.json";

const LEGACY_CONFIG_FILES: &[&str] = &[
    "settings.json",
    "permissions.json",
    "plugin-state.json",
    "plugin-origins.json",
    "rubick-store.json",
];

const LEGACY_DATA_FILES: &[&str] = &["mytools.db"];

const LEGACY_DATA_DIRS: &[&str] = &[
    "store-cache",
    "marketplace-icons",
    "marketplace-readme",
    "plugin-files",
    "logs",
];

/// True once the migration has completed on some previous start.
pub fn is_migrated(config_root: &Path) -> bool {
    config_root.join(MARKER_FILE).exists()
}

/// True once the v3 pivot migration has completed.
pub fn is_pivot_done(config_root: &Path) -> bool {
    config_root.join(MARKER_FILE_V3).exists()
}

/// Run the one-time v3 pivot: drop ecosystem-installed plugins, rename
/// `rubick-store.json` → `plugin-store.json` and delete the retired
/// marketplace caches. Safe to rerun; marked by `layout-v3.json`.
pub fn run_pivot(config_root: &Path, data_root: &Path) -> Result<(), String> {
    if is_pivot_done(config_root) {
        return Ok(());
    }
    std::fs::create_dir_all(config_root).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(data_root).map_err(|e| e.to_string())?;

    // The Rubick KV store becomes the generic plugin store.
    let legacy_store = config_root.join("rubick-store.json");
    let new_store = config_root.join(PLUGIN_STORE_FILE);
    if legacy_store.is_file() && !new_store.exists() {
        std::fs::rename(&legacy_store, &new_store)
            .map_err(|e| format!("store rename failed: {}", e))?;
    }

    // Ecosystem provenance names the plugin directories to remove.
    let removed = remove_ecosystem_plugins(&config_root.join("plugin-origins.json"), data_root);
    if let Err(e) = removed {
        tracing::warn!("ecosystem plugin cleanup failed: {}", e);
    }

    // Leftovers: ecosystem packages that were installed before provenance
    // tracking existed are not listed in plugin-origins.json. Their names
    // still carry the ecosystem prefix, so they can be identified safely.
    if let Err(e) = remove_ecosystem_leftovers(data_root) {
        tracing::warn!("ecosystem leftover cleanup failed: {}", e);
    }

    // Retired marketplace caches.
    for dir in ["store-cache", "marketplace-icons", "marketplace-readme"] {
        let path = data_root.join(dir);
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        }
    }

    write_marker_v3(config_root)
}

/// Delete plugin directories whose names carry the retired ecosystem
/// prefix (e.g. `rubick-markdown`). These packages cannot run on this
/// host and are not part of the toolbox any more.
fn remove_ecosystem_leftovers(data_root: &Path) -> Result<(), String> {
    let plugins_root = data_root.join("plugins");
    let Ok(entries) = std::fs::read_dir(&plugins_root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_ascii_lowercase().contains("rubick") {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
            tracing::info!("pivot: removed ecosystem plugin {}", name);
        }
    }
    Ok(())
}

/// Parse `plugin-origins.json`, delete the plugin directories (and their
/// plugin-files namespaces) tagged as ecosystem installs and rewrite the
/// file without those records. Legacy array records (all-ecosystem) are
/// emptied entirely.
fn remove_ecosystem_plugins(origins_path: &Path, data_root: &Path) -> Result<(), String> {
    let bytes = match std::fs::read(origins_path) {
        Ok(b) => b,
        Err(_) => return Ok(()),
    };
    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    let mut surviving: Vec<(String, String)> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    match value {
        serde_json::Value::Array(names) => {
            for name in names.iter().filter_map(|v| v.as_str()) {
                removed.push(name.to_string());
            }
        }
        serde_json::Value::Object(map) => {
            for (name, origin) in map {
                if origin.as_str() == Some("ecosystem") {
                    removed.push(name);
                } else {
                    let origin = origin.as_str().unwrap_or("custom").to_string();
                    surviving.push((name, origin));
                }
            }
        }
        _ => return Ok(()),
    }
    if removed.is_empty() {
        return Ok(());
    }

    let plugins_root = data_root.join("plugins");
    let plugin_files_root = data_root.join("plugin-files");
    for name in &removed {
        remove_plugin_dir(&plugins_root, name);
        remove_plugin_dir(&plugin_files_root, name);
        tracing::info!("pivot: removed ecosystem plugin {}", name);
    }

    let mut body = serde_json::Map::new();
    for (name, origin) in surviving {
        body.insert(name, serde_json::Value::String(origin));
    }
    std::fs::write(
        origins_path,
        serde_json::to_vec_pretty(&body).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("origins rewrite failed: {}", e))
}

/// Delete `<root>/<name>` when the name is a plain, safe directory name.
fn remove_plugin_dir(root: &Path, name: &str) {
    let name = name.trim();
    if name.is_empty()
        || name.starts_with('.')
        || name.contains('\\')
        || name.contains('/')
        || name.contains("..")
    {
        return;
    }
    let dir = root.join(name);
    if dir.is_dir() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Run the one-time migration. `legacy` may not exist (fresh installs).
pub fn run(legacy: &Path, config_root: &Path, data_root: &Path) -> Result<(), String> {
    if is_migrated(config_root) {
        return Ok(());
    }
    migrate_legacy(legacy, config_root, data_root)?;
    write_marker(config_root)
}

/// Move every legacy item into the new roots. Missing sources are
/// skipped, destinations are never overwritten, and cross-device moves
/// fall back to copy-verify-delete. Only returns `Err` when the legacy
/// directory existed but contained items that could not be moved.
pub fn migrate_legacy(legacy: &Path, config_root: &Path, data_root: &Path) -> Result<(), String> {
    if let Err(e) = std::fs::create_dir_all(config_root) {
        return Err(format!(
            "config root {} create failed: {}",
            config_root.display(),
            e
        ));
    }
    if let Err(e) = std::fs::create_dir_all(data_root) {
        return Err(format!(
            "data root {} create failed: {}",
            data_root.display(),
            e
        ));
    }

    for name in LEGACY_CONFIG_FILES {
        move_file(&legacy.join(name), &config_root.join(name))?;
    }
    for name in LEGACY_DATA_FILES {
        move_file(&legacy.join(name), &data_root.join(name))?;
    }
    for name in LEGACY_DATA_DIRS {
        merge_dir(&legacy.join(name), &data_root.join(name))?;
    }

    // Plugins are data: legacy identifier dir → `<data>/plugins`.
    merge_dir(&legacy.join("plugins"), &data_root.join("plugins"))?;

    // Historical residue from the pre-identifier layout:
    // `%APPDATA%\MyTools\plugins` (e.g. a stray demo-hello install).
    merge_dir(&config_root.join("plugins"), &data_root.join("plugins"))?;

    if !legacy.exists() {
        return Ok(());
    }
    if legacy_is_empty(legacy) {
        return std::fs::remove_dir(legacy)
            .map_err(|e| format!("legacy dir {}: remove failed: {}", legacy.display(), e));
    }
    Ok(())
}

fn write_marker(config_root: &Path) -> Result<(), String> {
    let marker = config_root.join(MARKER_FILE);
    let body = json!({ "version": 2, "migratedAt": now_secs() });
    std::fs::write(
        &marker,
        serde_json::to_string_pretty(&body).expect("marker json"),
    )
    .map_err(|e| format!("marker {}: write failed: {}", marker.display(), e))
}

fn write_marker_v3(config_root: &Path) -> Result<(), String> {
    let marker = config_root.join(MARKER_FILE_V3);
    let body = json!({ "version": 3, "pivot": "official-tools", "migratedAt": now_secs() });
    std::fs::write(
        &marker,
        serde_json::to_string_pretty(&body).expect("marker json"),
    )
    .map_err(|e| format!("marker {}: write failed: {}", marker.display(), e))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Move `src` → `dst`. Skips a missing source; keeps both when the
/// destination already exists (the legacy copy is left behind so a
/// retried start does the same nothing without data loss).
fn move_file(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() || dst.exists() {
        return Ok(());
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("{}: create failed: {}", parent.display(), e))?;
    }
    if let Err(err) = std::fs::rename(src, dst) {
        // Cross-device links cannot rename; copy + verify + delete.
        let bytes = std::fs::copy(src, dst).map_err(|e| {
            format!(
                "{} -> {}: move failed ({}) and copy failed ({})",
                src.display(),
                dst.display(),
                err,
                e
            )
        })?;
        let src_len = std::fs::metadata(src).map_err(|e| e.to_string())?.len();
        if bytes != src_len {
            return Err(format!(
                "{} -> {}: copy truncated ({} of {} bytes)",
                src.display(),
                dst.display(),
                bytes,
                src_len
            ));
        }
        std::fs::remove_file(src)
            .map_err(|e| format!("{}: remove after copy failed: {}", src.display(), e))?;
    }
    Ok(())
}

/// Recursively merge directory `src` into `dst` without overwriting any
/// existing destination entry.
fn merge_dir(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dst).map_err(|e| format!("{}: create failed: {}", dst.display(), e))?;
    for entry in
        std::fs::read_dir(src).map_err(|e| format!("{}: read failed: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("{}: read failed: {}", src.display(), e))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            merge_dir(&from, &to)?;
        } else {
            move_file(&from, &to)?;
        }
    }
    if legacy_is_empty(src) {
        let _ = std::fs::remove_dir(src);
    }
    Ok(())
}

fn legacy_is_empty(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn moves_config_and_data_into_the_new_roots() {
        let base = TempDir::new().unwrap();
        let legacy = base.path().join("legacy");
        let config = base.path().join("config");
        let data = base.path().join("data");
        fs::create_dir_all(legacy.join("plugins/demo-hello")).unwrap();
        fs::create_dir_all(legacy.join("store-cache")).unwrap();
        fs::write(legacy.join("settings.json"), "{}").unwrap();
        fs::write(legacy.join("plugin-state.json"), "{}").unwrap();
        fs::write(legacy.join("rubick-store.json"), "{}").unwrap();
        fs::write(legacy.join("mytools.db"), "db").unwrap();
        fs::write(legacy.join("plugins/demo-hello/plugin.json"), "{}").unwrap();
        fs::write(legacy.join("store-cache/a.json"), "a").unwrap();

        run(&legacy, &config, &data).unwrap();

        assert_eq!(
            fs::read_to_string(config.join("settings.json")).unwrap(),
            "{}"
        );
        assert_eq!(
            fs::read_to_string(config.join("plugin-state.json")).unwrap(),
            "{}"
        );
        assert_eq!(fs::read_to_string(data.join("mytools.db")).unwrap(), "db");
        assert_eq!(
            fs::read_to_string(data.join("plugins/demo-hello/plugin.json")).unwrap(),
            "{}"
        );
        assert_eq!(
            fs::read_to_string(data.join("store-cache/a.json")).unwrap(),
            "a"
        );
        assert!(config.join(MARKER_FILE).exists());
        assert!(!legacy.exists(), "legacy dir emptied and removed");
        assert!(is_migrated(&config));
    }

    #[test]
    fn is_idempotent_after_the_marker() {
        let base = TempDir::new().unwrap();
        let legacy = base.path().join("legacy");
        let config = base.path().join("config");
        let data = base.path().join("data");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("settings.json"), "{}").unwrap();
        run(&legacy, &config, &data).unwrap();
        fs::write(config.join("settings.json"), "{\"theme\":\"dark\"}").unwrap();

        run(&legacy, &config, &data).unwrap();
        run(&legacy, &config, &data).unwrap();

        assert_eq!(
            fs::read_to_string(config.join("settings.json")).unwrap(),
            "{\"theme\":\"dark\"}"
        );
    }

    #[test]
    fn never_overwrites_an_existing_destination() {
        let base = TempDir::new().unwrap();
        let legacy = base.path().join("legacy");
        let config = base.path().join("config");
        let data = base.path().join("data");
        fs::create_dir_all(&legacy).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(legacy.join("settings.json"), "{\"old\":true}").unwrap();
        fs::write(config.join("settings.json"), "{\"new\":true}").unwrap();

        run(&legacy, &config, &data).unwrap();

        assert_eq!(
            fs::read_to_string(config.join("settings.json")).unwrap(),
            "{\"new\":true}"
        );
        assert!(legacy.join("settings.json").exists(), "legacy copy kept");
        assert!(config.join(MARKER_FILE).exists(), "still marked done");
    }

    #[test]
    fn merges_stray_config_plugins_into_data_plugins() {
        let base = TempDir::new().unwrap();
        let legacy = base.path().join("legacy");
        let config = base.path().join("config");
        let data = base.path().join("data");
        fs::create_dir_all(&legacy).unwrap();
        fs::create_dir_all(config.join("plugins/demo-hello")).unwrap();
        fs::write(config.join("plugins/demo-hello/plugin.json"), "{}").unwrap();

        run(&legacy, &config, &data).unwrap();

        assert_eq!(
            fs::read_to_string(data.join("plugins/demo-hello/plugin.json")).unwrap(),
            "{}"
        );
    }

    #[test]
    fn fresh_install_is_a_noop() {
        let base = TempDir::new().unwrap();
        let legacy = base.path().join("legacy");
        let config = base.path().join("config");
        let data = base.path().join("data");
        run(&legacy, &config, &data).unwrap();
        assert!(is_migrated(&config));
        assert!(!legacy.exists());
    }

    #[test]
    fn pivot_removes_ecosystem_plugins_and_renames_store() {
        let base = TempDir::new().unwrap();
        let config = base.path().join("config");
        let data = base.path().join("data");
        let plugins_root = data.join("plugins");
        let files_root = data.join("plugin-files");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(plugins_root.join("ip-config-rubick-plugin")).unwrap();
        fs::create_dir_all(plugins_root.join("mytools-json")).unwrap();
        fs::create_dir_all(plugins_root.join("demo-hello")).unwrap();
        fs::create_dir_all(files_root.join("ip-config-rubick-plugin")).unwrap();
        fs::write(
            plugins_root.join("ip-config-rubick-plugin/plugin.json"),
            "{}",
        )
        .unwrap();
        fs::write(config.join("rubick-store.json"), r#"{"k":"v"}"#).unwrap();
        fs::write(
            config.join("plugin-origins.json"),
            r#"{
                "ip-config-rubick-plugin": "ecosystem",
                "mytools-json": "official",
                "demo-hello": "custom"
            }"#,
        )
        .unwrap();

        run_pivot(&config, &data).unwrap();

        assert!(
            !plugins_root.join("ip-config-rubick-plugin").is_dir(),
            "removed"
        );
        assert!(
            !files_root.join("ip-config-rubick-plugin").is_dir(),
            "files removed"
        );
        assert!(plugins_root.join("mytools-json").is_dir(), "official kept");
        assert!(plugins_root.join("demo-hello").is_dir(), "custom kept");
        assert_eq!(
            fs::read_to_string(config.join("plugin-store.json")).unwrap(),
            r#"{"k":"v"}"#
        );
        assert!(!config.join("rubick-store.json").exists());

        let origins: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(config.join("plugin-origins.json")).unwrap())
                .unwrap();
        assert!(origins.get("ip-config-rubick-plugin").is_none());
        assert_eq!(
            origins.get("mytools-json").and_then(|v| v.as_str()),
            Some("official")
        );
        assert!(config.join(MARKER_FILE_V3).exists());
    }

    #[test]
    fn pivot_treats_legacy_array_origins_as_ecosystem() {
        let base = TempDir::new().unwrap();
        let config = base.path().join("config");
        let data = base.path().join("data");
        let plugins_root = data.join("plugins");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(plugins_root.join("rubick-markdown")).unwrap();
        fs::write(config.join("plugin-origins.json"), r#"["rubick-markdown"]"#).unwrap();

        run_pivot(&config, &data).unwrap();

        assert!(!plugins_root.join("rubick-markdown").is_dir());
    }

    #[test]
    fn pivot_removes_ecosystem_leftovers_by_name() {
        let base = TempDir::new().unwrap();
        let config = base.path().join("config");
        let data = base.path().join("data");
        let plugins_root = data.join("plugins");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(plugins_root.join("rubick-markdown")).unwrap();
        fs::create_dir_all(plugins_root.join("mytools-json")).unwrap();
        fs::create_dir_all(plugins_root.join("demo-hello")).unwrap();

        run_pivot(&config, &data).unwrap();

        assert!(
            !plugins_root.join("rubick-markdown").is_dir(),
            "leftover removed"
        );
        assert!(plugins_root.join("mytools-json").is_dir(), "official kept");
        assert!(plugins_root.join("demo-hello").is_dir(), "custom kept");
    }

    #[test]
    fn pivot_is_idempotent_after_the_marker() {
        let base = TempDir::new().unwrap();
        let config = base.path().join("config");
        let data = base.path().join("data");
        let plugins_root = data.join("plugins");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(plugins_root.join("rubick-markdown")).unwrap();
        fs::write(
            config.join("plugin-origins.json"),
            r#"{"rubick-markdown":"ecosystem"}"#,
        )
        .unwrap();

        run_pivot(&config, &data).unwrap();
        // Second run: works again, but there is nothing left to remove.
        // Simulate a re-created dir with no provenance entry — the marker
        // must short-circuit the cleanup.
        fs::create_dir_all(plugins_root.join("rubick-markdown")).unwrap();
        run_pivot(&config, &data).unwrap();
        assert!(plugins_root.join("rubick-markdown").is_dir());
    }
}
