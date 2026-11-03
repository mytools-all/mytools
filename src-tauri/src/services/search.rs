//! Unified search command — takes a query string and returns the
//! top matching results from the built-in command catalog. External
//! plugins and the Start Menu app index are added in Day 9 / Day 8.

use mytools_core::{builtin::try_inline_calculator, fuzzy_match, PluginManifest, BUILTIN_COMMANDS};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub kind: &'static str,
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub score: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

pub fn run_with_plugins(
    query: &str,
    plugins: &[PluginManifest],
    icons: &HashMap<String, Option<String>>,
) -> Vec<SearchResult> {
    run_with_app_search(query, plugins, icons, |query| {
        search_apps(query, &crate::services::apps::index::apps().apps)
    })
}

fn run_with_app_search(
    query: &str,
    plugins: &[PluginManifest],
    icons: &HashMap<String, Option<String>>,
    app_search: impl FnOnce(&str) -> Vec<SearchResult>,
) -> Vec<SearchResult> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }

    if let Some(value) = try_inline_calculator(q) {
        return vec![SearchResult {
            kind: "calculator",
            id: "calculator:inline".into(),
            title: value,
            subtitle: format!("= {}", q.trim_start_matches('=').trim()),
            score: i64::MAX,
            icon: None,
        }];
    }

    let mut results: Vec<SearchResult> = Vec::new();

    let builtin_candidates: Vec<(String, String)> = BUILTIN_COMMANDS
        .iter()
        .map(|c| {
            let title = format!("{} ({})", c.explain, c.cmds.first().copied().unwrap_or(""));
            (c.code.to_string(), title)
        })
        .collect();

    for hit in fuzzy_match(q, &builtin_candidates) {
        let cmd = BUILTIN_COMMANDS.iter().find(|c| c.code == hit.id).unwrap();
        let kind_str = kind_to_str(cmd.kind);
        let icon = match cmd.kind {
            mytools_core::BuiltinKind::Plugins => Some(plugins_icon().to_string()),
            mytools_core::BuiltinKind::Store => Some(store_icon().to_string()),
            _ => None,
        };
        results.push(SearchResult {
            kind: kind_str,
            id: cmd.code.to_string(),
            title: hit.title,
            subtitle: format!("type: {}", kind_str),
            score: hit.score,
            icon,
        });
    }

    results.extend(app_search(q));

    let plugin_candidates: Vec<(String, String)> = plugins
        .iter()
        .filter(|p| !p.is_system_plugin())
        .flat_map(|p| {
            p.features.iter().map(move |f| {
                let title = format!("{}: {}", p.name, f.explain);
                (format!("{}:{}", p.name, f.code), title)
            })
        })
        .collect();
    for hit in fuzzy_match(q, &plugin_candidates) {
        if let Some((plugin_name, feature_code)) = hit.id.split_once(':') {
            if let Some(p) = plugins.iter().find(|p| p.name == plugin_name) {
                if let Some(f) = p.features.iter().find(|f| f.code == feature_code) {
                    results.push(SearchResult {
                        kind: "plugin",
                        id: format!("{}:{}", plugin_name, feature_code),
                        title: hit.title,
                        subtitle: format!("plugin · {}", f.explain),
                        score: hit.score - 1,
                        icon: icons.get(plugin_name).cloned().flatten(),
                    });
                }
            }
        }
    }

    results.sort_by_key(|b| std::cmp::Reverse(b.score));
    results
}

fn search_apps(
    query: &str,
    apps: &[crate::services::apps::index::IndexedApp],
) -> Vec<SearchResult> {
    let app_candidates: Vec<(String, String)> = apps
        .iter()
        .map(|e| (e.name.clone(), e.name.clone()))
        .collect();
    let mut results = Vec::new();
    for hit in fuzzy_match(query, &app_candidates) {
        if let Some(app) = apps.iter().find(|e| e.name == hit.title) {
            results.push(SearchResult {
                kind: "app",
                id: app.target.clone(),
                title: app.name.clone(),
                subtitle: app.source.label().to_string(),
                score: hit.score - 1 - i64::from(app.source.rank()),
                icon: None,
            });
        }
    }

    results
}

fn kind_to_str(kind: mytools_core::BuiltinKind) -> &'static str {
    match kind {
        mytools_core::BuiltinKind::Calculator => "calculator",
        mytools_core::BuiltinKind::JsonFormatter => "json-formatter",
        mytools_core::BuiltinKind::Base64Url => "base64-url",
        mytools_core::BuiltinKind::TimestampUuid => "timestamp-uuid",
        mytools_core::BuiltinKind::ColorPicker => "color-picker",
        mytools_core::BuiltinKind::Clipboard => "clipboard",
        mytools_core::BuiltinKind::Plugins => "plugins",
        mytools_core::BuiltinKind::Store => "store",
    }
}

/// Icon for the plugin store entry (a shopping bag).
fn store_icon() -> &'static str {
    "data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2040%2040%22%3E%3Crect%20width%3D%2240%22%20height%3D%2240%22%20rx%3D%228%22%20fill%3D%22%230f766e%22%2F%3E%3Cpath%20d%3D%22M12%2015h16l-1.6%2012.2a2%202%200%200%201-2%201.8h-8.8a2%202%200%200%201-2-1.8L12%2015z%22%20fill%3D%22none%22%20stroke%3D%22%23fff%22%20stroke-width%3D%222%22%2F%3E%3Cpath%20d%3D%22M15.5%2015a4.5%204.5%200%200%201%209%200%22%20fill%3D%22none%22%20stroke%3D%22%23fff%22%20stroke-width%3D%222%22%2F%3E%3C%2Fsvg%3E"
}

/// Icon for the plugin manager entry (a grid of tool tiles).
fn plugins_icon() -> &'static str {
    "data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2040%2040%22%3E%3Crect%20width%3D%2240%22%20height%3D%2240%22%20rx%3D%228%22%20fill%3D%22%234f46e5%22%2F%3E%3Crect%20x%3D%2211%22%20y%3D%2211%22%20width%3D%228%22%20height%3D%228%22%20rx%3D%222%22%20fill%3D%22%23fff%22%2F%3E%3Crect%20x%3D%2221%22%20y%3D%2211%22%20width%3D%228%22%20height%3D%228%22%20rx%3D%222%22%20fill%3D%22%23fff%22%20opacity%3D%220.75%22%2F%3E%3Crect%20x%3D%2211%22%20y%3D%2221%22%20width%3D%228%22%20height%3D%228%22%20rx%3D%222%22%20fill%3D%22%23fff%22%20opacity%3D%220.75%22%2F%3E%3Crect%20x%3D%2221%22%20y%3D%2221%22%20width%3D%228%22%20height%3D%228%22%20rx%3D%222%22%20fill%3D%22%23fff%22%2F%3E%3C%2Fsvg%3E"
}

pub async fn search(
    query: String,
    registry: crate::services::plugins::SharedPluginRegistry,
) -> Result<Vec<SearchResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let loaded = registry.list();
        let icons: HashMap<String, Option<String>> = loaded
            .iter()
            .filter(|p| p.enabled)
            .map(|p| (p.manifest.name.clone(), p.icon.clone()))
            .collect();
        let manifests: Vec<PluginManifest> = loaded
            .into_iter()
            .filter(|p| p.enabled)
            .map(|p| p.manifest)
            .collect();
        run_with_plugins(&query, &manifests, &icons)
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::apps::index::{AppSource, IndexedApp, TargetKind};

    fn fixture_apps() -> Vec<IndexedApp> {
        [
            (
                "记事本",
                "Fixture.Notepad_123!App",
                TargetKind::ShellItem,
                AppSource::AppsFolder,
            ),
            (
                "计算器",
                "Fixture.Calculator_123!App",
                TargetKind::ShellItem,
                AppSource::AppsFolder,
            ),
            (
                "画图",
                "Fixture.Paint_123!App",
                TargetKind::ShellItem,
                AppSource::AppsFolder,
            ),
            (
                "设置",
                "Fixture.Settings_123!App",
                TargetKind::ShellItem,
                AppSource::AppsFolder,
            ),
            (
                "Visual Studio Code",
                "C:/fixture/Visual Studio Code.lnk",
                TargetKind::FilePath,
                AppSource::StartMenu,
            ),
            (
                "Bandizip",
                "C:/fixture/Bandizip.exe",
                TargetKind::FilePath,
                AppSource::Registry,
            ),
        ]
        .into_iter()
        .map(|(name, target, kind, source)| IndexedApp {
            name: name.into(),
            target: target.into(),
            kind,
            source,
        })
        .collect()
    }

    fn run_with_plugins(
        query: &str,
        plugins: &[PluginManifest],
        icons: &HashMap<String, Option<String>>,
    ) -> Vec<SearchResult> {
        run_with_app_search(query, plugins, icons, |query| {
            search_apps(query, &fixture_apps())
        })
    }

    #[test]
    fn empty_and_calculator_queries_do_not_load_apps() {
        assert!(
            run_with_app_search(" ", &[], &HashMap::new(), |_| panic!("unexpected app scan"))
                .is_empty()
        );
        let results = run_with_app_search("=1+2", &[], &HashMap::new(), |_| {
            panic!("unexpected app scan")
        });
        assert_eq!(results[0].title, "3");
    }

    /// 记事本 / 计算器 这类应用没有 `.lnk`、也不注册到 Uninstall，
    /// 只有 `shell:AppsFolder` 能发现它们。这是整个应用索引存在的理由。
    #[test]
    fn store_apps_are_searchable_by_chinese_name() {
        let icons = HashMap::new();
        for query in ["记事本", "计算器", "画图"] {
            let results = run_with_plugins(query, &[], &icons);
            assert!(
                !results.is_empty(),
                "querying {query:?} must return at least one result"
            );
        }
    }

    #[test]
    fn store_apps_are_launchable_targets() {
        let icons = HashMap::new();
        let results = run_with_plugins("记事本", &[], &icons);
        let hit = results
            .iter()
            .find(|r| r.kind == "app" && r.title.contains("记事本"))
            .unwrap_or_else(|| panic!("no 记事本 result in {results:?}"));
        assert_eq!(
            crate::services::apps::launcher::launch_kind(&hit.id),
            crate::services::apps::launcher::LaunchKind::ShellItem,
            "store apps must dispatch through the shell-item path, id={:?}",
            hit.id
        );
    }

    #[test]
    fn start_menu_apps_keep_file_paths_as_ids() {
        let icons = HashMap::new();
        let results = run_with_plugins("Visual Studio Code", &[], &icons);
        let hit = results
            .iter()
            .find(|r| r.kind == "app" && r.title.contains("Visual Studio Code"))
            .unwrap_or_else(|| panic!("no VS Code result in {results:?}"));
        assert!(
            hit.id.to_ascii_lowercase().ends_with(".lnk"),
            "start menu entries should keep their shortcut path, got {:?}",
            hit.id
        );
    }

    #[test]
    fn registry_apps_are_discoverable_without_shortcuts() {
        // Typora / Bandizip etc. are registered in Uninstall. At least one
        // must surface even though some have no Start Menu shortcut.
        let icons = HashMap::new();
        let results = run_with_plugins("Bandizip", &[], &icons);
        assert!(
            results.iter().any(|r| r.kind == "app"),
            "Uninstall-registered apps must be searchable: {results:?}"
        );
    }

    #[test]
    fn system_settings_still_resolve() {
        let icons = HashMap::new();
        let results = run_with_plugins("设置", &[], &icons);
        assert!(
            !results.is_empty(),
            "设置 should still resolve: {results:?}"
        );
    }

    fn manifest(json: &str) -> PluginManifest {
        PluginManifest::from_bytes(json.as_bytes()).unwrap()
    }

    #[test]
    fn plugins_icon_is_decodable_svg_without_duplicate_attrs() {
        let uri = plugins_icon();
        assert!(uri.starts_with("data:image/svg+xml,"));
        let payload = &uri["data:image/svg+xml,".len()..];
        let decoded = percent_decode(payload);
        assert!(decoded.starts_with("<svg"), "got: {decoded}");
        assert!(decoded.ends_with("</svg>"), "got: {decoded}");
        for tag in decoded.split('>').filter(|t| !t.trim().is_empty()) {
            let mut seen = std::collections::HashSet::new();
            for attr in tag
                .split_whitespace()
                .filter_map(|tok| tok.split('=').next())
                .filter(|name| !name.starts_with('<') && !name.is_empty())
            {
                assert!(
                    seen.insert(attr),
                    "duplicate attribute `{attr}` in tag: {tag}"
                );
            }
        }
    }

    fn percent_decode(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or_default();
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn run_with_plugins_skips_system_plugins() {
        let ui: PluginManifest = manifest(
            r#"{
                "name": "ui-clipboard",
                "description": "x",
                "version": "1.0.0",
                "main": "index.html",
                "features": [{ "code": "record", "explain": "临时剪贴板", "cmds": ["clipboard"] }]
            }"#,
        );
        let system = manifest(
            r#"{
                "name": "sys-clipboard",
                "pluginType": "system",
                "description": "x",
                "version": "1.0.0",
                "main": "index.html",
                "features": [{ "code": "record", "explain": "临时剪贴板", "cmds": ["clipboard"] }]
            }"#,
        );
        let plugins = vec![ui.clone(), system.clone()];
        let icons = std::collections::HashMap::new();
        let results = run_with_plugins("clipboard", &plugins, &icons);
        let kinds: Vec<&str> = results.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&"plugin"));
        assert!(
            results.iter().all(|r| !r.id.starts_with("sys-clipboard:")),
            "system plugin must not surface in search: {results:?}"
        );
    }

    #[test]
    fn search_finds_windows_settings_app() {
        let icons = std::collections::HashMap::new();
        let results = run_with_plugins("设置", &[], &icons);
        let hit = results
            .iter()
            .find(|r| r.kind == "app" && r.title == "设置")
            .unwrap_or_else(|| panic!("searching 设置 must surface the Settings app: {results:?}"));
        assert_eq!(hit.subtitle, "app (system)");
        let indexed = fixture_apps()
            .iter()
            .find(|a| a.name == "设置")
            .map(|a| a.target.clone())
            .expect("the Settings fixture should be indexed");
        assert_eq!(hit.id, indexed);
    }
}
