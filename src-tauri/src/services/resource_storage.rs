use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Default, Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Size {
    pub bytes: u64,
    pub files: u64,
    pub skipped: u64,
    pub errors: u64,
    pub limited: bool,
    pub exists: bool,
}
struct Budget {
    deadline: Instant,
    left: usize,
}
fn reparse(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}
fn scan(path: &Path, recursive: bool, budget: &mut Budget) -> Size {
    let mut result = Size::default();
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        if budget.left == 0 || Instant::now() > budget.deadline {
            result.limited = true;
            break;
        }
        budget.left -= 1;
        let metadata = match fs::symlink_metadata(&current) {
            Ok(m) => m,
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    result.errors += 1;
                }
                continue;
            }
        };
        result.exists = true;
        if metadata.file_type().is_symlink() || reparse(&metadata) {
            result.skipped += 1;
            continue;
        }
        if metadata.is_file() {
            result.bytes = result.bytes.saturating_add(metadata.len());
            result.files += 1;
        } else if metadata.is_dir() {
            match fs::read_dir(&current) {
                Ok(entries) => {
                    for entry in entries {
                        if stack.len() >= budget.left || Instant::now() > budget.deadline {
                            result.limited = true;
                            break;
                        }
                        match entry {
                            Ok(entry) => {
                                if recursive || entry.file_type().is_ok_and(|kind| kind.is_file()) {
                                    stack.push(entry.path());
                                }
                            }
                            Err(_) => result.errors += 1,
                        }
                    }
                }
                Err(_) => result.errors += 1,
            }
        }
    }
    result
}
fn store_entries(path: &Path) -> Result<serde_json::Map<String, Value>, String> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Default::default()),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("共享插件存储超过扫描上限".into());
    }
    serde_json::from_slice::<Value>(&bytes)
        .map_err(|e| e.to_string())?
        .as_object()
        .cloned()
        .ok_or_else(|| "共享存储格式无法识别".into())
}
fn logical_bytes(store: &serde_json::Map<String, Value>, plugin: &str) -> u64 {
    let prefix = format!("{plugin}:");
    store
        .iter()
        .filter(|(key, _)| key.starts_with(&prefix))
        .map(|(key, value)| {
            serde_json::to_vec(&(key, value))
                .map(|bytes| bytes.len() as u64)
                .unwrap_or(0)
        })
        .sum()
}
struct Cache {
    key: String,
    time: Instant,
    value: Value,
}
static CACHE: OnceLock<Mutex<Option<Cache>>> = OnceLock::new();

pub fn snapshot<R: Runtime>(app: &AppHandle<R>, force: bool) -> Result<Value, String> {
    let registry = app.state::<crate::services::plugins::SharedPluginRegistry>();
    let plugins = registry.list();
    let data = crate::infrastructure::paths::data_root(app);
    let config = crate::infrastructure::paths::config_root(app);
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let program = exe.parent().ok_or("无法确定程序目录")?.to_path_buf();
    let root = registry.root().unwrap_or_else(|| data.join("plugins"));
    let profile = std::env::var_os("WEBVIEW2_USER_DATA_FOLDER")
        .map(PathBuf::from)
        .or_else(|| {
            app.path()
                .app_local_data_dir()
                .ok()
                .map(|p| p.join("EBWebView"))
        });
    let key = format!(
        "{:?}{:?}{:?}{:?}{:?}",
        root,
        data,
        config,
        profile,
        plugins
            .iter()
            .map(|p| (&p.manifest.name, &p.manifest.version))
            .collect::<Vec<_>>()
    );
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|e| e.to_string())?;
    if !force {
        if let Some(cached) = cache.as_ref() {
            if cached.key == key && cached.time.elapsed() < Duration::from_secs(60) {
                return Ok(cached.value.clone());
            }
        }
    }
    let mut budget = Budget {
        deadline: Instant::now() + Duration::from_secs(10),
        left: 250_000,
    };
    let stores = store_entries(&config.join("plugin-store.json"));
    let mut rows = Vec::new();
    for plugin in &plugins {
        let name = &plugin.manifest.name;
        let install = scan(Path::new(&plugin.dir), true, &mut budget);
        let scope: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let sandbox = scan(&data.join("plugin-files").join(scope), true, &mut budget);
        rows.push(json!({"id":name,"title":plugin.manifest.plugin_name.as_ref().unwrap_or(name),"enabled":plugin.enabled,"version":plugin.manifest.version,"path":plugin.dir,"install":install,"sandbox":sandbox,"settingsLogicalBytes":stores.as_ref().ok().map(|store|logical_bytes(store,name)),"memory":null,"cpu":null}));
    }
    let mut categories = Vec::new();
    for (title, path, recursive) in [
        ("主程序与同级文件", program.clone(), false),
        ("插件目录（含暂存文件）", root.clone(), true),
        ("插件私有文件", data.join("plugin-files"), true),
        ("日志", data.join("logs"), true),
        ("配置与共享插件存储", config, true),
        ("更新与人工备份", program.join("backups"), true),
        ("数据目录同级文件（数据库等）", data.clone(), false),
    ] {
        categories.push(json!({"title":title,"path":path.to_string_lossy(),"size":scan(&path,recursive,&mut budget)}));
    }
    if let Some(profile) = profile {
        categories.push(json!({"title":"共享 WebView 数据（缓存及配置）","path":profile.to_string_lossy(),"size":scan(&profile,true,&mut budget)}));
    }
    if let Ok(entries) = fs::read_dir(&data) {
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir())
                && path != root
                && path != data.join("plugin-files")
                && path != data.join("logs")
            {
                categories.push(json!({"title":format!("其他应用数据：{}",entry.file_name().to_string_lossy()),"path":path.to_string_lossy(),"size":scan(&path,true,&mut budget)}));
            }
        }
    }
    let value = json!({"scannedAt":SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis(),"plugins":rows,"categories":categories,"sharedStoreReadable":stores.is_ok(),"limited":budget.left==0||Instant::now()>budget.deadline,"note":"文件大小合计，非 NTFS 实际分配空间；共享 JSON 条目大小为逻辑大小，不重复加入目录合计。符号链接与重解析点不追随。"});
    *cache = Some(Cache {
        key,
        time: Instant::now(),
        value: value.clone(),
    });
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scan_handles_files_missing_roots_and_budget() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("one"), b"1234").unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested/two"), b"56").unwrap();
        let mut budget = Budget {
            deadline: Instant::now() + Duration::from_secs(5),
            left: 100,
        };
        assert_eq!(scan(dir.path(), true, &mut budget).bytes, 6);
        assert_eq!(scan(dir.path(), false, &mut budget).bytes, 4);
        assert!(!scan(&dir.path().join("missing"), true, &mut budget).exists);
        budget.left = 0;
        assert!(scan(dir.path(), true, &mut budget).limited);
    }
    #[test]
    fn shared_store_counts_only_exact_namespace() {
        let store = serde_json::from_value::<serde_json::Map<String, Value>>(
            json!({"one:key":"value","one-extra:key":"private"}),
        )
        .unwrap();
        assert_eq!(logical_bytes(&store, "unknown"), 0);
        assert!(logical_bytes(&store, "one") > 0);
        assert!(logical_bytes(&store, "one") < logical_bytes(&store, "one-extra"));
    }
}
