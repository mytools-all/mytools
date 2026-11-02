use mytools_core::capability::CapabilityError;
use mytools_core::dir_grants::DirGrantStore;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    pub path: PathBuf,
    #[serde(default)]
    pub name: String,
    pub content: Option<String>,
    pub max_results: Option<usize>,
    pub max_depth: Option<usize>,
    pub max_file_bytes: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: u64,
}

pub fn search(
    grants: &DirGrantStore,
    scope: &str,
    options: SearchOptions,
) -> Result<serde_json::Value, CapabilityError> {
    let root = options
        .path
        .canonicalize()
        .map_err(|e| CapabilityError::invalid_args(e.to_string()))?;
    require_grant(grants, scope, &root)?;
    if !root.is_dir() {
        return Err(CapabilityError::invalid_args("搜索路径必须是目录"));
    }
    let limit = options.max_results.unwrap_or(500).clamp(1, 2000);
    let depth = options.max_depth.unwrap_or(6).clamp(1, 20);
    let max_bytes = options
        .max_file_bytes
        .unwrap_or(2 * 1024 * 1024)
        .min(2 * 1024 * 1024);
    let name = options.name.to_lowercase();
    let content = options.content.filter(|s| !s.is_empty());
    let mut entries = Vec::new();
    let mut scanned = 0;
    let mut skipped = 0;
    let mut truncated = false;
    for item in walkdir::WalkDir::new(&root)
        .min_depth(1)
        .max_depth(depth)
        .follow_links(false)
        .into_iter()
    {
        let item = match item {
            Ok(item) => item,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        scanned += 1;
        if scanned > 50000 {
            truncated = true;
            break;
        }
        if !item.file_type().is_file() || !grants.is_granted(scope, item.path()) {
            continue;
        }
        let file_name = item.file_name().to_string_lossy().into_owned();
        if !file_name.to_lowercase().contains(&name) {
            continue;
        }
        let metadata = match item.metadata() {
            Ok(m) => m,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if let Some(needle) = &content {
            if metadata.len() > max_bytes {
                skipped += 1;
                continue;
            }
            match std::fs::read_to_string(item.path()) {
                Ok(text) if text.contains(needle) => {}
                _ => {
                    continue;
                }
            }
        }
        if entries.len() == limit {
            truncated = true;
            break;
        }
        entries.push(FileEntry {
            path: item.path().to_string_lossy().into_owned(),
            name: file_name,
            size: metadata.len(),
            modified: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });
    }
    Ok(
        serde_json::json!({ "entries": entries, "truncated": truncated, "scanned": scanned, "skipped": skipped }),
    )
}

#[derive(Clone, Deserialize, Serialize)]
pub struct RenameEntry {
    pub from: PathBuf,
    pub to: PathBuf,
}

fn require_grant(grants: &DirGrantStore, scope: &str, path: &Path) -> Result<(), CapabilityError> {
    if grants.is_granted(scope, path) {
        Ok(())
    } else {
        Err(CapabilityError::Denied {
            capability: "fs".into(),
            reason: "请先选择并授权目录".into(),
        })
    }
}

fn valid_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    !name.is_empty()
        && name.len() <= 240
        && !name.ends_with(['.', ' '])
        && !name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
        && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

pub fn rename_many(
    grants: &DirGrantStore,
    scope: &str,
    entries: Vec<RenameEntry>,
) -> Result<serde_json::Value, CapabilityError> {
    if entries.is_empty() || entries.len() > 1000 {
        return Err(CapabilityError::invalid_args("每次需要 1–1000 个文件"));
    }
    let mut sources = HashSet::new();
    let mut destinations = HashSet::new();
    let mut directories = Vec::new();
    for entry in &entries {
        let source = entry
            .from
            .canonicalize()
            .map_err(|e| CapabilityError::invalid_args(e.to_string()))?;
        let metadata = std::fs::symlink_metadata(&entry.from)
            .map_err(|e| CapabilityError::invalid_args(e.to_string()))?;
        if metadata.is_dir() {
            directories.push(source.clone());
        }
        if !(metadata.is_file() || metadata.is_dir())
            || metadata.file_type().is_symlink()
            || !entry.from.is_absolute()
            || !entry.to.is_absolute()
        {
            return Err(CapabilityError::invalid_args(
                "仅支持普通文件或文件夹，不支持符号链接",
            ));
        }
        let parent = entry
            .to
            .parent()
            .ok_or_else(|| CapabilityError::invalid_args("目标路径无效"))?
            .canonicalize()
            .map_err(|e| CapabilityError::invalid_args(e.to_string()))?;
        if source.parent() != Some(parent.as_path())
            || !entry
                .to
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(valid_name)
        {
            return Err(CapabilityError::invalid_args(
                "只能在同一目录内重命名，文件名必须合法",
            ));
        }
        require_grant(grants, scope, &source)?;
        require_grant(grants, scope, &parent)?;
        if entry.to.exists()
            || !sources.insert(source.to_string_lossy().to_lowercase())
            || !destinations.insert(entry.to.to_string_lossy().to_lowercase())
        {
            return Err(CapabilityError::invalid_args(
                "目标已存在或存在重复条目，请修改预览后重试",
            ));
        }
    }
    for directory in directories {
        let prefix = format!(
            "{}\\",
            directory
                .to_string_lossy()
                .to_lowercase()
                .trim_end_matches('\\')
        );
        if sources.iter().any(|source| source.starts_with(&prefix)) {
            return Err(CapabilityError::invalid_args(
                "不能同时重命名文件夹及其子项，请分批操作",
            ));
        }
    }
    let mut renamed = Vec::new();
    let mut failed = Vec::new();
    for entry in entries {
        if let Err(error) = require_grant(grants, scope, &entry.from) {
            failed.push(serde_json::json!({"path":entry.from,"reason":error.to_string()}));
            break;
        }
        match move_file(&entry.from, &entry.to) {
            Ok(()) => renamed.push(entry),
            Err(e) => {
                failed.push(serde_json::json!({"path":entry.from,"reason":e.to_string()}));
                break;
            }
        }
    }
    Ok(serde_json::json!({"renamed":renamed,"failed":failed}))
}

fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { windows_sys::Win32::Storage::FileSystem::MoveFileW(from.as_ptr(), to.as_ptr()) }
            == 0
        {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    #[cfg(not(windows))]
    {
        std::fs::hard_link(from, to)?;
        std::fs::remove_file(from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folders_can_be_renamed_and_restored_without_losing_contents() {
        let (dir, grants) = fixture();
        grants.grant("test", dir.path()).unwrap();
        let old = dir.path().join("notes.v1");
        let new = dir.path().join("notes.v2");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("note.md"), "hello").unwrap();
        rename_many(
            &grants,
            "test",
            vec![RenameEntry {
                from: old.clone(),
                to: new.clone(),
            }],
        )
        .unwrap();
        assert!(new.join("note.md").exists());
        rename_many(
            &grants,
            "test",
            vec![RenameEntry {
                from: new,
                to: old.clone(),
            }],
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(old.join("note.md")).unwrap(),
            "hello"
        );
    }
    fn fixture() -> (tempfile::TempDir, DirGrantStore) {
        let dir = tempfile::tempdir().unwrap();
        let grants = DirGrantStore::load(dir.path().join("grants.json"));
        (dir, grants)
    }
    #[test]
    fn search_requires_grant_and_bounds_results() {
        let (dir, grants) = fixture();
        for name in ["one.txt", "two.txt", "three.bin"] {
            std::fs::write(dir.path().join(name), "hello world").unwrap();
        }
        let options = || SearchOptions {
            path: dir.path().into(),
            name: ".txt".into(),
            content: Some("world".into()),
            max_results: Some(1),
            max_depth: None,
            max_file_bytes: None,
        };
        assert!(search(&grants, "test", options()).is_err());
        grants.grant("test", dir.path()).unwrap();
        let result = search(&grants, "test", options()).unwrap();
        assert_eq!(result["entries"].as_array().unwrap().len(), 1);
        assert_eq!(result["truncated"], true);
        assert!(search(&grants, "other", options()).is_err());
    }
    #[test]
    fn rename_validates_entire_batch_and_supports_undo() {
        let (dir, grants) = fixture();
        grants.grant("test", dir.path()).unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        let c = dir.path().join("c.txt");
        std::fs::write(&a, "a").unwrap();
        std::fs::write(&b, "b").unwrap();
        assert!(rename_many(
            &grants,
            "test",
            vec![
                RenameEntry {
                    from: a.clone(),
                    to: c.clone()
                },
                RenameEntry {
                    from: b.clone(),
                    to: a.clone()
                }
            ]
        )
        .is_err());
        assert!(a.exists());
        assert!(!c.exists());
        assert!(rename_many(
            &grants,
            "other",
            vec![RenameEntry {
                from: a.clone(),
                to: c.clone()
            }]
        )
        .is_err());
        rename_many(
            &grants,
            "test",
            vec![RenameEntry {
                from: a.clone(),
                to: c.clone(),
            }],
        )
        .unwrap();
        assert!(!a.exists());
        assert!(c.exists());
        rename_many(
            &grants,
            "test",
            vec![RenameEntry {
                from: c,
                to: a.clone(),
            }],
        )
        .unwrap();
        assert!(a.exists());
    }
    #[test]
    fn rejects_windows_reserved_names() {
        for name in ["CON.txt", "a.", "a ", "COM1", "x/y", "x:y", ""] {
            assert!(!valid_name(name));
        }
        assert!(valid_name("报告 01.txt"));
    }
}
