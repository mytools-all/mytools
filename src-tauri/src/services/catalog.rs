use std::io::Read;
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSources {
    pub plugin_list: String,
    pub plugin_list_mirror: String,
}

pub fn default_sources() -> CatalogSources {
    serde_json::from_str(include_str!("../../../src/config/catalog.json"))
        .expect("bundled catalog configuration")
}

pub fn validate_url(value: &str) -> Result<(), String> {
    let url = tauri::Url::parse(value).map_err(|e| e.to_string())?;
    if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("插件仓库必须使用不含账号密码的 HTTPS 地址".into());
    }
    Ok(())
}

impl CatalogSources {
    pub fn validate(&self) -> Result<(), String> {
        validate_url(&self.plugin_list)?;
        validate_url(&self.plugin_list_mirror)
    }
}
const MAX_BYTES: u64 = 2 * 1024 * 1024;

pub fn fetch_catalog(url: &str) -> Result<serde_json::Value, String> {
    validate_url(url)?;
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .build();
    let response = agent.get(url).call().map_err(|e| e.to_string())?;
    let mut body = Vec::new();
    response
        .into_reader()
        .take(MAX_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|e| e.to_string())?;
    if body.len() as u64 > MAX_BYTES {
        return Err("plugin catalog exceeds size limit".into());
    }
    serde_json::from_slice(&body).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_sources_are_valid_and_old_settings_receive_defaults() {
        let settings: crate::services::settings::AppSettings = serde_json::from_str("{}").expect("legacy settings");
        settings.catalog_sources.validate().expect("valid defaults");
        assert!(settings.catalog_sources.plugin_list.contains("raw.githubusercontent.com"));
        assert!(settings.catalog_sources.plugin_list_mirror.contains("gitee.com"));
    }

    #[test]
    fn source_urls_require_https_without_credentials() {
        assert!(validate_url("https://example.com/plugins/database.json").is_ok());
        assert!(validate_url("http://example.com/a.json").is_err());
        assert!(validate_url("https://user:password@example.com/a.json").is_err());
    }
}
