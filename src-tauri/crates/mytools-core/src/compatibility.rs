use crate::manifest::PluginManifest;
use node_semver::{Range, Version};

pub fn incompatibility(manifest: &PluginManifest, host: &str, api: u32) -> Option<String> {
    if let Some(reason) = manifest.incompatibility(api) {
        return Some(reason);
    }
    range_incompatibility(manifest.engines.get("mytools").map(String::as_str), host)
}

pub fn range_incompatibility(requirement: Option<&str>, host: &str) -> Option<String> {
    let requirement = requirement?;
    if requirement.len() > 512 {
        return Some("插件声明的 MyTools 版本范围过长".into());
    }
    if requirement.trim().is_empty() {
        return Some("插件声明的 MyTools 版本范围为空".into());
    }
    let range = match Range::parse(requirement) {
        Ok(range) => range,
        Err(_) => return Some(format!("插件声明的 MyTools 版本范围无效：{requirement}")),
    };
    let host_version = match Version::parse(host) {
        Ok(version) => version,
        Err(_) => return Some("无法确定当前 MyTools 版本".into()),
    };
    (!range.satisfies(&host_version)).then(|| format!("需要 MyTools {requirement}，当前为 {host}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_ranges_follow_npm_semantics() {
        assert!(range_incompatibility(Some(">=0.2.0 <1.0.0"), "0.2.0").is_none());
        assert!(range_incompatibility(Some(">=0.2.0 <1.0.0"), "1.0.0").is_some());
        assert!(range_incompatibility(Some("^0.2.0"), "0.2.9").is_none());
        assert!(range_incompatibility(Some("^0.2.0"), "0.3.0").is_some());
        assert!(range_incompatibility(Some("^0.2.0 || ^1.0.0"), "1.5.0").is_none());
        assert!(range_incompatibility(Some(">=0.2.0"), "0.3.0-beta.1").is_some());
        assert!(range_incompatibility(Some(">=0.3.0-beta.1 <0.3.0"), "0.3.0-beta.2").is_none());
    }
    #[test]
    fn old_plugins_are_allowed_but_invalid_declarations_are_rejected() {
        assert!(range_incompatibility(None, "0.2.0").is_none());
        for requirement in ["", "latest", "not-a-version"] {
            assert!(range_incompatibility(Some(requirement), "0.2.0").is_some());
        }
    }
}
