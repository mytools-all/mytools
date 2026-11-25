use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufRead, Write},
    path::Path,
    sync::Mutex,
};
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub at: String,
    pub operation: String,
    pub target: Option<String>,
    pub success: bool,
}

pub fn record(root: &Path, operation: &str, target: Option<&str>, success: bool) {
    let Ok(_guard) = LOCK.lock() else { return };
    let result = (|| -> std::io::Result<()> {
        fs::create_dir_all(root)?;
        let date = chrono::Utc::now();
        let path = root.join(format!("operations.{}.jsonl", date.format("%Y-%m-%d")));
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let event = Event {
            at: date.to_rfc3339(),
            operation: operation.into(),
            target: target
                .filter(|value| {
                    mytools_core::plugin_mgmt::is_safe_plugin_name(value) && value.len() <= 160
                })
                .map(String::from),
            success,
        };
        serde_json::to_writer(&mut file, &event)?;
        file.write_all(b"\n")?;
        let mut files = fs::read_dir(root)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("operations.")
                    && entry.path().extension().is_some_and(|ext| ext == "jsonl")
            })
            .collect::<Vec<_>>();
        files.sort_by_key(|entry| entry.file_name());
        let remove = files.len().saturating_sub(14);
        for entry in files.into_iter().take(remove) {
            if entry.file_type()?.is_file() {
                let _ = fs::remove_file(entry.path());
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        tracing::warn!("operation log unavailable: {error}");
    }
}

pub fn recent(root: &Path) -> Vec<Event> {
    let Ok(_guard) = LOCK.lock() else {
        return Vec::new();
    };
    let Ok(files) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut files = files
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("operations.")
                && entry.path().extension().is_some_and(|ext| ext == "jsonl")
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    files.sort();
    let mut result = std::collections::VecDeque::new();
    for path in files.into_iter().rev().take(3).rev() {
        let Ok(file) = fs::File::open(path) else {
            continue;
        };
        for line in std::io::BufReader::new(file)
            .lines()
            .map_while(std::result::Result::ok)
        {
            if line.len() > 4096 {
                continue;
            }
            if let Ok(event) = serde_json::from_str(&line) {
                result.push_back(event);
                if result.len() > 200 {
                    result.pop_front();
                }
            }
        }
    }
    result.into_iter().rev().collect()
}
