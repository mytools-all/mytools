//! Built-in command catalog.
//!
//! These are first-class features of MyTools (vs. external plugins in
//! `plugins/`). Day 3 seeds the calculator; Day 4 adds the JSON /
//! Base64 / UUID / timestamp commands; Day 5 adds the app launcher
//! and color picker entries.
//!
//! The list is exposed as a static slice so the frontend can use the
//! same data for hints and the Tauri search command can rerank by
//! recency / frequency.

use serde::Serialize;

/// One row in the launcher — `code` is stable, `explain` is the label
/// shown in the result list, `cmds` are the search keywords.
#[derive(Debug, Clone, Serialize)]
pub struct BuiltinCommand {
    pub code: &'static str,
    pub explain: &'static str,
    pub cmds: &'static [&'static str],
    pub kind: BuiltinKind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltinKind {
    Calculator,
    JsonFormatter,
    Base64Url,
    TimestampUuid,
    ColorPicker,
    Clipboard,
    Plugins,
    Store,
}

pub const BUILTIN_COMMANDS: &[BuiltinCommand] = &[
    BuiltinCommand {
        code: "calculator",
        explain: "Calculator  (= prefix to evaluate)",
        cmds: &["=", "calc", "calculator", "计算", "计算器"],
        kind: BuiltinKind::Calculator,
    },
    BuiltinCommand {
        code: "json-formatter",
        explain: "JSON Formatter",
        cmds: &["json", "format", "格式化"],
        kind: BuiltinKind::JsonFormatter,
    },
    BuiltinCommand {
        code: "base64-url",
        explain: "Base64 / URL encode & decode",
        cmds: &["base64", "url", "encode", "decode", "编码"],
        kind: BuiltinKind::Base64Url,
    },
    BuiltinCommand {
        code: "timestamp-uuid",
        explain: "Timestamp ⇄ Date and UUID generator",
        cmds: &["timestamp", "uuid", "时间戳"],
        kind: BuiltinKind::TimestampUuid,
    },
    BuiltinCommand {
        code: "color-picker",
        explain: "Color Picker (screen sample)",
        cmds: &["color", "picker", "颜色"],
        kind: BuiltinKind::ColorPicker,
    },
    BuiltinCommand {
        code: "clipboard",
        explain: "Clipboard history",
        cmds: &["clipboard", "剪贴板", "paste", "历史"],
        kind: BuiltinKind::Clipboard,
    },
    BuiltinCommand {
        code: "plugins",
        explain: "Plugin manager",
        cmds: &["插件", "插件中心", "插件管理", "plugins"],
        kind: BuiltinKind::Plugins,
    },
    BuiltinCommand {
        code: "store",
        explain: "Plugin store",
        cmds: &["插件商店", "商店", "工具", "store"],
        kind: BuiltinKind::Store,
    },
];

/// Try to evaluate a calculator expression directly from the search
/// input. Returns `Some(formatted)` when the input is a `=...` style
/// math expression, `None` otherwise.
pub fn try_inline_calculator(input: &str) -> Option<String> {
    let body = input.strip_prefix('=')?.trim();
    if body.is_empty() {
        return None;
    }
    super::calculator::eval(body)
        .ok()
        .map(super::calculator::format_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_catalog_has_eight_entries() {
        assert_eq!(BUILTIN_COMMANDS.len(), 8);
    }

    #[test]
    fn builtin_catalog_includes_the_plugin_manager() {
        assert!(BUILTIN_COMMANDS
            .iter()
            .any(|c| matches!(c.kind, BuiltinKind::Plugins)));
        assert!(BUILTIN_COMMANDS
            .iter()
            .find(|c| matches!(c.kind, BuiltinKind::Plugins))
            .unwrap()
            .cmds
            .iter()
            .any(|k| k.contains("插件中心")));
    }

    #[test]
    fn builtin_catalog_includes_the_store() {
        assert!(BUILTIN_COMMANDS
            .iter()
            .any(|c| matches!(c.kind, BuiltinKind::Store)));
        assert!(BUILTIN_COMMANDS
            .iter()
            .find(|c| matches!(c.kind, BuiltinKind::Store))
            .unwrap()
            .cmds
            .iter()
            .any(|k| k.contains("插件商店")));
    }

    #[test]
    fn calculator_prefix_evaluates() {
        assert_eq!(try_inline_calculator("=1+2*3"), Some("7".to_string()));
    }

    #[test]
    fn calculator_prefix_with_whitespace() {
        assert_eq!(try_inline_calculator("= 10 / 4 "), Some("2.5".to_string()));
    }

    #[test]
    fn calculator_without_prefix_returns_none() {
        assert_eq!(try_inline_calculator("1+2"), None);
        assert_eq!(try_inline_calculator("="), None);
        assert_eq!(try_inline_calculator("= "), None);
    }

    #[test]
    fn builtin_search_finds_keywords() {
        let hits: Vec<_> = BUILTIN_COMMANDS
            .iter()
            .filter(|c| c.cmds.iter().any(|k| k.contains("json")))
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].code, "json-formatter");
    }
}
