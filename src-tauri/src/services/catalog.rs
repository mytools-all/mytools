use std::io::Read;
use std::time::Duration;

pub const CATALOG_URL: &str =
    "https://raw.githubusercontent.com/mytools-all/mytools-home/main/plugins/database.json";
const MAX_BYTES: u64 = 2 * 1024 * 1024;

pub fn fetch_catalog() -> Result<serde_json::Value, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .build();
    let response = agent.get(CATALOG_URL).call().map_err(|e| e.to_string())?;
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
