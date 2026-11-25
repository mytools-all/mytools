use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, String>;
const MAX_FILE: u64 = 128 * 1024 * 1024;
const MAX_TOTAL: u64 = 2 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 50_000;
const MAX_MANIFEST: u64 = 16 * 1024 * 1024;

#[cfg(test)]
#[path = "backup_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct Roots {
    pub config: PathBuf,
    pub data: PathBuf,
    pub plugins: PathBuf,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub mode: String,
    #[serde(default)]
    pub plugins: Vec<String>,
    #[serde(default)]
    pub include_programs: bool,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    pub id: String,
    pub title: String,
    pub version: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Entry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: String,
    pub schema_version: u32,
    pub host_version: String,
    pub created_at: String,
    pub selection: Selection,
    pub plugins: Vec<Plugin>,
    pub files: Vec<Entry>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOptions {
    pub host: bool,
    pub clipboard: bool,
    pub plugins: Vec<String>,
    pub programs: bool,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub restored_items: usize,
    pub recovery_backup: String,
}

fn error(value: impl std::fmt::Display) -> String {
    value.to_string()
}
fn json(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    no_link(path)?;
    let metadata = fs::metadata(path).map_err(error)?;
    if metadata.len() > MAX_FILE {
        return Err("配置文件超过备份大小上限".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(error)?).map_err(error)
}
fn object(path: &Path) -> Result<Map<String, Value>> {
    json(path)?
        .as_object()
        .cloned()
        .ok_or_else(|| "插件存储不是有效 JSON 对象".into())
}
fn no_link(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(error)?;
    if metadata.file_type().is_symlink() {
        return Err(format!("不支持符号链接：{}", path.display()));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(format!("不支持重解析点：{}", path.display()));
        }
    }
    Ok(())
}
fn safe_id(id: &str) -> Result<()> {
    if !crate::plugin_mgmt::is_safe_plugin_name(id) || id.len() > 160 || id.contains(['/', '\\']) {
        return Err("插件身份无效".into());
    }
    components(id)
}
fn components(path: &str) -> Result<()> {
    if path.len() > 4096
        || path.is_empty()
        || path.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with(['.', ' '])
                || part.chars().any(|c| c < ' ' || "\\:<>\"|?*".contains(c))
                || matches!(
                    part.split('.')
                        .next()
                        .unwrap_or("")
                        .to_ascii_uppercase()
                        .as_str(),
                    "CON"
                        | "PRN"
                        | "AUX"
                        | "NUL"
                        | "COM1"
                        | "COM2"
                        | "COM3"
                        | "COM4"
                        | "COM5"
                        | "COM6"
                        | "COM7"
                        | "COM8"
                        | "COM9"
                        | "LPT1"
                        | "LPT2"
                        | "LPT3"
                        | "LPT4"
                        | "LPT5"
                        | "LPT6"
                        | "LPT7"
                        | "LPT8"
                        | "LPT9"
                )
        })
    {
        return Err("备份包含不安全的文件名".into());
    }
    Ok(())
}
fn allowed(path: &str) -> Result<()> {
    components(path)?;
    if matches!(
        path,
        "host/settings.json" | "plugins/store.json" | "plugins/state.json" | "clipboard/mytools.db"
    ) {
        return Ok(());
    }
    let parts: Vec<_> = path.split('/').collect();
    match parts.as_slice() {
        ["plugins", "files", id, ..] if parts.len() > 3 => safe_id(id),
        ["programs", id, ..] if parts.len() > 2 => safe_id(id),
        _ => Err("备份包含未知的数据类型".into()),
    }
}

pub fn plugins(roots: &Roots) -> Result<Vec<Plugin>> {
    let mut found: BTreeMap<String, Plugin> = BTreeMap::new();
    let store = object(&roots.config.join("plugin-store.json"))?;
    for key in store.keys() {
        let id = key.split_once(':').ok_or("插件存储缺少作用域")?.0;
        safe_id(id)?;
        found.entry(id.into()).or_insert(Plugin {
            id: id.into(),
            title: id.into(),
            version: None,
        });
    }
    for root in [&roots.plugins, &roots.data.join("plugin-files")] {
        if !root.exists() {
            continue;
        }
        no_link(root)?;
        for entry in fs::read_dir(root).map_err(error)? {
            let entry = entry.map_err(error)?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if id.starts_with('.') || safe_id(&id).is_err() || !entry.path().is_dir() {
                continue;
            }
            no_link(&entry.path())?;
            let manifest = crate::plugin_mgmt::read_manifest(&entry.path()).ok();
            let plugin = Plugin {
                id: id.clone(),
                title: manifest
                    .as_ref()
                    .map(|manifest| manifest.display_name().to_string())
                    .unwrap_or(id.clone()),
                version: manifest.map(|manifest| manifest.version),
            };
            if root == &roots.plugins || !found.contains_key(&id) {
                found.insert(id, plugin);
            }
        }
    }
    Ok(found.into_values().collect())
}

enum Content {
    File(PathBuf),
    Bytes(Vec<u8>),
}
fn collect_tree(root: &Path, prefix: &str, out: &mut Vec<(String, Content)>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    no_link(root)?;
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(error)?;
        no_link(entry.path())?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(root)
                .map_err(error)?
                .to_string_lossy()
                .replace('\\', "/");
            let name = format!("{prefix}/{rel}");
            allowed(&name)?;
            out.push((name, Content::File(entry.path().into())));
            if out.len() > MAX_FILES {
                return Err("备份文件过多".into());
            }
        }
    }
    Ok(())
}
fn serialize(value: &impl Serialize) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(error)
}

pub fn create(
    roots: &Roots,
    host: &str,
    selection: &Selection,
    destination: &Path,
) -> Result<Manifest> {
    if !destination
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err("备份文件必须使用 .zip 扩展名".into());
    }
    if !matches!(
        selection.mode.as_str(),
        "all" | "host" | "plugins" | "selected"
    ) {
        return Err("未知的备份范围".into());
    }
    let all = if selection.mode == "host" {
        Vec::new()
    } else {
        plugins(roots)?
    };
    let selected: BTreeSet<String> = if selection.mode == "host" {
        BTreeSet::new()
    } else if selection.mode == "selected" {
        selection.plugins.iter().cloned().collect()
    } else {
        all.iter().map(|plugin| plugin.id.clone()).collect()
    };
    if selection.mode == "selected" && selected.is_empty() {
        return Err("请至少选择一个插件".into());
    }
    for id in &selected {
        safe_id(id)?;
        if !all.iter().any(|plugin| &plugin.id == id) {
            return Err(format!("找不到插件数据：{id}"));
        }
    }
    let parent = destination.parent().ok_or("备份路径无效")?;
    fs::create_dir_all(parent).map_err(error)?;
    for protected in [
        &roots.config,
        &roots.plugins,
        &roots.data.join("plugin-files"),
    ] {
        if let (Ok(parent), Ok(protected)) = (parent.canonicalize(), protected.canonicalize()) {
            if parent.starts_with(protected) {
                return Err("请将备份保存到配置和插件目录之外".into());
            }
        }
    }
    if destination == roots.data.join("mytools.db") {
        return Err("备份路径不能覆盖应用数据库".into());
    }
    let temp = tempfile::tempdir().map_err(error)?;
    let mut payload = Vec::new();
    if matches!(selection.mode.as_str(), "all" | "host") {
        let mut settings = object(&roots.config.join("settings.json"))?;
        settings.remove("pluginRoot");
        payload.push((
            "host/settings.json".into(),
            Content::Bytes(serialize(&settings)?),
        ));
    }
    if selection.mode != "host" {
        let store = object(&roots.config.join("plugin-store.json"))?
            .into_iter()
            .filter(|(key, _)| {
                key.split_once(':')
                    .is_some_and(|(id, _)| selected.contains(id))
            })
            .collect::<Map<_, _>>();
        payload.push((
            "plugins/store.json".into(),
            Content::Bytes(serialize(&store)?),
        ));
        let state_path = roots.config.join("plugin-state.json");
        let state = if state_path.exists() {
            serde_json::from_value::<Vec<String>>(json(&state_path)?).map_err(error)?
        } else {
            Vec::new()
        };
        let state: Vec<_> = state
            .into_iter()
            .filter(|id| selected.contains(id))
            .collect();
        payload.push((
            "plugins/state.json".into(),
            Content::Bytes(serialize(&state)?),
        ));
        for id in &selected {
            collect_tree(
                &roots.data.join("plugin-files").join(id),
                &format!("plugins/files/{id}"),
                &mut payload,
            )?;
            if selection.include_programs {
                collect_tree(
                    &roots.plugins.join(id),
                    &format!("programs/{id}"),
                    &mut payload,
                )?;
            }
        }
    }
    let database = roots.data.join("mytools.db");
    if selection.mode == "all" && database.exists() {
        no_link(&database)?;
        let snapshot = temp.path().join("mytools.db");
        let connection = rusqlite::Connection::open_with_flags(
            &database,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(error)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(error)?;
        connection
            .backup(rusqlite::DatabaseName::Main, &snapshot, None)
            .map_err(error)?;
        payload.push(("clipboard/mytools.db".into(), Content::File(snapshot)));
    }
    let mut manifest = Manifest {
        format: "MyToolsBackup".into(),
        schema_version: 1,
        host_version: host.into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        selection: selection.clone(),
        plugins: all
            .into_iter()
            .filter(|plugin| selected.contains(&plugin.id))
            .collect(),
        files: Vec::new(),
    };
    let mut output = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    {
        let mut writer = zip::ZipWriter::new(output.as_file_mut());
        let mut total = 0u64;
        for (name, content) in payload {
            allowed(&name)?;
            writer
                .start_file(
                    &name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .map_err(error)?;
            let mut input: Box<dyn Read> = match content {
                Content::File(path) => Box::new(fs::File::open(path).map_err(error)?),
                Content::Bytes(bytes) => Box::new(std::io::Cursor::new(bytes)),
            };
            let mut hash = Sha256::new();
            let mut size = 0u64;
            let mut buffer = [0u8; 65536];
            loop {
                let count = input.read(&mut buffer).map_err(error)?;
                if count == 0 {
                    break;
                }
                size += count as u64;
                total += count as u64;
                if size > MAX_FILE || total > MAX_TOTAL {
                    return Err("备份超过大小上限（单文件 128 MiB，合计 2 GiB）".into());
                }
                hash.update(&buffer[..count]);
                writer.write_all(&buffer[..count]).map_err(error)?;
            }
            manifest.files.push(Entry {
                path: name,
                size,
                sha256: format!("{:x}", hash.finalize()),
            });
        }
        writer
            .start_file("manifest.json", zip::write::SimpleFileOptions::default())
            .map_err(error)?;
        writer.write_all(&serialize(&manifest)?).map_err(error)?;
        writer.finish().map_err(error)?;
    }
    output.as_file().sync_all().map_err(error)?;
    output.persist(destination).map_err(error)?;
    Ok(manifest)
}

pub fn inspect(path: &Path, host: &str) -> Result<Manifest> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).map_err(error)?).map_err(error)?;
    if archive.len() > MAX_FILES + 1 {
        return Err("备份文件过多".into());
    }
    let manifest: Manifest = {
        let mut file = archive
            .by_name("manifest.json")
            .map_err(|_| "不是 MyTools 备份文件")?;
        if file.size() > MAX_MANIFEST {
            return Err("备份清单过大".into());
        }
        let mut bytes = Vec::new();
        file.by_ref()
            .take(MAX_MANIFEST + 1)
            .read_to_end(&mut bytes)
            .map_err(error)?;
        serde_json::from_slice(&bytes).map_err(error)?
    };
    if manifest.format != "MyToolsBackup"
        || manifest.schema_version != 1
        || !matches!(
            manifest.selection.mode.as_str(),
            "all" | "host" | "plugins" | "selected"
        )
    {
        return Err("不支持的备份格式".into());
    }
    let source = node_semver::Version::parse(&manifest.host_version).map_err(error)?;
    let target = node_semver::Version::parse(host).map_err(error)?;
    if source > target {
        return Err(format!(
            "备份来自 MyTools {}，请先更新主程序",
            manifest.host_version
        ));
    }
    let mut ids = BTreeSet::new();
    let mut folded_ids = BTreeSet::new();
    for plugin in &manifest.plugins {
        safe_id(&plugin.id)?;
        if !ids.insert(plugin.id.clone()) || !folded_ids.insert(plugin.id.to_lowercase()) {
            return Err("备份包含重复插件".into());
        }
    }
    let expected: BTreeMap<_, _> = manifest
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect();
    if expected.len() != manifest.files.len() || archive.len() != expected.len() + 1 {
        return Err("备份文件与清单不一致".into());
    }
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(error)?;
        let name = file.name().to_string();
        if !seen.insert(name.to_lowercase()) {
            return Err("备份包含重复文件名".into());
        }
        if name == "manifest.json" {
            continue;
        }
        allowed(&name)?;
        if file.is_dir()
            || file
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("备份不允许目录项或符号链接".into());
        }
        let expected = expected
            .get(name.as_str())
            .ok_or("备份存在清单之外的文件")?;
        if file.size() != expected.size || file.size() > MAX_FILE {
            return Err("备份文件大小不正确".into());
        }
        total = total.checked_add(file.size()).ok_or("备份大小溢出")?;
        if total > MAX_TOTAL {
            return Err("备份解压后超过 2 GiB".into());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut size = 0u64;
        loop {
            let count = file.read(&mut buffer).map_err(error)?;
            if count == 0 {
                break;
            }
            size += count as u64;
            if size > expected.size {
                return Err("备份文件超过声明大小".into());
            }
            hash.update(&buffer[..count]);
        }
        if size != expected.size || format!("{:x}", hash.finalize()) != expected.sha256 {
            return Err(format!("备份校验失败：{name}"));
        }
    }
    if manifest.selection.mode != "host"
        && (!expected.contains_key("plugins/store.json")
            || !expected.contains_key("plugins/state.json"))
    {
        return Err("备份缺少插件存储或启用状态".into());
    }
    if matches!(manifest.selection.mode.as_str(), "host" | "all")
        && !expected.contains_key("host/settings.json")
    {
        return Err("备份缺少主程序配置".into());
    }
    for file in &manifest.files {
        let parts: Vec<_> = file.path.split('/').collect();
        let id = match parts.as_slice() {
            ["plugins", "files", id, ..] | ["programs", id, ..] => Some(*id),
            _ => None,
        };
        if id.is_some_and(|id| !ids.contains(id)) {
            return Err("备份文件的插件身份未在清单中声明".into());
        }
    }
    Ok(manifest)
}

fn extract(path: &Path, root: &Path, manifest: &Manifest) -> Result<()> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).map_err(error)?).map_err(error)?;
    for entry in &manifest.files {
        let target = root.join(&entry.path);
        fs::create_dir_all(target.parent().ok_or("解压路径无效")?).map_err(error)?;
        let mut input = archive.by_name(&entry.path).map_err(error)?;
        let mut output = fs::File::create(target).map_err(error)?;
        std::io::copy(&mut input, &mut output).map_err(error)?;
        output.sync_all().map_err(error)?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Plan {
    target: String,
    workspace: String,
    existed: bool,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    version: u32,
    committed: bool,
    plans: Vec<Plan>,
}
fn target(roots: &Roots, key: &str) -> Result<PathBuf> {
    components(key)?;
    match key {
        "config/settings.json" => Ok(roots.config.join("settings.json")),
        "config/plugin-store.json" => Ok(roots.config.join("plugin-store.json")),
        "config/plugin-state.json" => Ok(roots.config.join("plugin-state.json")),
        "data/mytools.db" => Ok(roots.data.join("mytools.db")),
        _ => {
            if let Some(id) = key.strip_prefix("data/plugin-files/") {
                safe_id(id)?;
                return Ok(roots.data.join("plugin-files").join(id));
            }
            if let Some(id) = key.strip_prefix("programs/") {
                safe_id(id)?;
                return Ok(roots.plugins.join(id));
            }
            Err("恢复目标不受支持".into())
        }
    }
}
fn guard_target(roots: &Roots, key: &str) -> Result<PathBuf> {
    let path = target(roots, key)?;
    let root = if key.starts_with("config/") {
        &roots.config
    } else if key.starts_with("programs/") {
        &roots.plugins
    } else {
        &roots.data
    };
    let mut current = root.clone();
    if fs::symlink_metadata(&current).is_ok() {
        no_link(&current)?;
    }
    for component in path.strip_prefix(root).map_err(error)?.components() {
        current.push(component);
        if fs::symlink_metadata(&current).is_ok() {
            no_link(&current)?;
        }
    }
    Ok(path)
}
fn remove(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    no_link(path)?;
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(error)
    } else {
        fs::remove_file(path).map_err(error)
    }
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().ok_or("写入路径无效")?;
    fs::create_dir_all(parent).map_err(error)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    temp.write_all(&serialize(value)?).map_err(error)?;
    temp.as_file().sync_all().map_err(error)?;
    temp.persist(path).map_err(error)?;
    Ok(())
}

pub fn recover(roots: &Roots) -> Result<()> {
    let path = roots.data.join("restore-journal.json");
    if !path.exists() {
        return Ok(());
    }
    let journal: Journal = serde_json::from_value(json(&path)?).map_err(error)?;
    if journal.version != 1 || journal.plans.len() > 520 {
        return Err("恢复事务记录无效".into());
    }
    for plan in journal.plans.iter().rev() {
        let destination = guard_target(roots, &plan.target)?;
        if !plan.workspace.starts_with(".mytools-restore-")
            || !plan
                .workspace
                .trim_start_matches(".mytools-restore-")
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
            || plan.workspace.len() > 100
        {
            return Err("恢复事务工作目录无效".into());
        }
        let workspace = destination
            .parent()
            .ok_or("恢复路径无效")?
            .join(&plan.workspace);
        if !workspace.exists() {
            continue;
        }
        no_link(&workspace)?;
        let previous = workspace.join("previous");
        let next = workspace.join("next");
        if !journal.committed {
            if previous.exists() {
                remove(&destination)?;
                fs::rename(&previous, &destination).map_err(error)?;
            } else if !plan.existed && !next.exists() {
                remove(&destination)?;
            }
        }
        remove(&workspace)?;
    }
    fs::remove_file(path).map_err(error)
}

pub fn restore(
    archive: &Path,
    roots: &Roots,
    host: &str,
    options: &RestoreOptions,
    validate_host: &dyn Fn(&Value) -> Result<()>,
) -> Result<Report> {
    recover(roots)?;
    let manifest = inspect(archive, host)?;
    let selected: BTreeSet<_> = options.plugins.iter().cloned().collect();
    for id in &selected {
        safe_id(id)?;
        if !manifest.plugins.iter().any(|plugin| &plugin.id == id) {
            return Err("恢复选择包含备份之外的插件".into());
        }
    }
    if !options.host && !options.clipboard && selected.is_empty() {
        return Err("没有选择恢复内容".into());
    }
    let extracted = tempfile::tempdir().map_err(error)?;
    extract(archive, extracted.path(), &manifest)?;
    let mut changes: Vec<(String, Content)> = Vec::new();
    if options.host {
        let source = extracted.path().join("host/settings.json");
        if !source.exists() {
            return Err("备份不包含主程序配置".into());
        }
        let incoming = object(&source)?;
        let mut existing = object(&roots.config.join("settings.json"))?;
        for key in [
            "theme",
            "hotkey",
            "trayEnabled",
            "floatballEnabled",
            "middlePanelEnabled",
            "updateMode",
            "pinnedPlugins",
            "recentPlugins",
            "detachedWindowWidth",
            "detachedWindowHeight",
            "floatballX",
            "floatballY",
            "mainX",
            "mainY",
            "mainWidth",
            "mainHeight",
            "uiState",
            "windowBounds",
            "catalogSources",
        ] {
            existing.remove(key);
            if let Some(value) = incoming.get(key) {
                existing.insert(key.into(), value.clone());
            }
        }
        validate_host(&Value::Object(existing.clone()))?;
        changes.push((
            "config/settings.json".into(),
            Content::Bytes(serialize(&existing)?),
        ));
    }
    if !selected.is_empty() {
        let incoming = object(&extracted.path().join("plugins/store.json"))?;
        let mut existing = object(&roots.config.join("plugin-store.json"))?;
        existing.retain(|key, _| {
            !key.split_once(':')
                .is_some_and(|(id, _)| selected.contains(id))
        });
        for (key, value) in incoming {
            if key
                .split_once(':')
                .is_some_and(|(id, _)| selected.contains(id))
            {
                existing.insert(key, value);
            }
        }
        changes.push((
            "config/plugin-store.json".into(),
            Content::Bytes(serialize(&existing)?),
        ));
        let state_path = roots.config.join("plugin-state.json");
        let mut state = if state_path.exists() {
            serde_json::from_value::<BTreeSet<String>>(json(&state_path)?).map_err(error)?
        } else {
            BTreeSet::new()
        };
        state.retain(|id| !selected.contains(id));
        let incoming: BTreeSet<String> =
            serde_json::from_value(json(&extracted.path().join("plugins/state.json"))?)
                .map_err(error)?;
        state.extend(incoming.into_iter().filter(|id| selected.contains(id)));
        changes.push((
            "config/plugin-state.json".into(),
            Content::Bytes(serialize(&state)?),
        ));
        for id in &selected {
            let source = extracted.path().join("plugins/files").join(id);
            fs::create_dir_all(&source).map_err(error)?;
            changes.push((format!("data/plugin-files/{id}"), Content::File(source)));
            if options.programs {
                let source = extracted.path().join("programs").join(id);
                if source.exists() {
                    let plugin = crate::plugin_mgmt::read_manifest(&source).map_err(error)?;
                    if plugin.name != *id {
                        return Err("插件程序身份不一致".into());
                    }
                    if let Some(reason) = crate::compatibility::incompatibility(
                        &plugin,
                        host,
                        crate::capability::HOST_API_VERSION,
                    ) {
                        return Err(reason);
                    }
                    if manifest
                        .plugins
                        .iter()
                        .find(|plugin| &plugin.id == id)
                        .and_then(|plugin| plugin.version.as_deref())
                        != Some(plugin.version.as_str())
                    {
                        return Err("插件程序版本与备份清单不一致".into());
                    }
                    changes.push((format!("programs/{id}"), Content::File(source)));
                }
            }
        }
    }
    if options.clipboard {
        let source = extracted.path().join("clipboard/mytools.db");
        if !source.exists() {
            return Err("备份不包含剪贴板数据".into());
        }
        let connection = rusqlite::Connection::open_with_flags(
            &source,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(error)?;
        let valid: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(error)?;
        if valid != "ok" {
            return Err("剪贴板数据库校验失败".into());
        }
        connection
            .prepare("SELECT id,content,kind,pinned,created_at FROM clipboard LIMIT 0")
            .map_err(error)?;
        drop(connection);
        changes.push(("data/mytools.db".into(), Content::File(source)));
    }
    for (key, _) in &changes {
        guard_target(roots, key)?;
    }
    let backups = roots.data.join("backups");
    fs::create_dir_all(&backups).map_err(error)?;
    let recovery = tempfile::Builder::new()
        .prefix("before-restore-")
        .suffix(".zip")
        .tempfile_in(&backups)
        .map_err(error)?
        .into_temp_path();
    let recovery_path = recovery.to_path_buf();
    create(
        roots,
        host,
        &Selection {
            mode: "all".into(),
            plugins: Vec::new(),
            include_programs: options.programs,
        },
        &recovery_path,
    )?;
    recovery.keep().map_err(error)?;
    let mut journal = Journal {
        version: 1,
        committed: false,
        plans: Vec::new(),
    };
    let mut workspaces = Vec::new();
    for (key, content) in changes {
        let destination = guard_target(roots, &key)?;
        let parent = destination.parent().ok_or("恢复路径无效")?;
        fs::create_dir_all(parent).map_err(error)?;
        let workspace = tempfile::Builder::new()
            .prefix(".mytools-restore-")
            .tempdir_in(parent)
            .map_err(error)?;
        let next = workspace.path().join("next");
        match content {
            Content::Bytes(bytes) => fs::write(&next, bytes).map_err(error)?,
            Content::File(source) => {
                if source.is_dir() {
                    fs::create_dir(&next).map_err(error)?;
                    copy_tree(&source, &next)?;
                } else {
                    fs::copy(&source, &next).map_err(error)?;
                }
            }
        }
        if next.is_file() {
            fs::OpenOptions::new()
                .write(true)
                .open(&next)
                .map_err(error)?
                .sync_all()
                .map_err(error)?;
        }
        journal.plans.push(Plan {
            target: key,
            workspace: workspace
                .path()
                .file_name()
                .ok_or("工作目录无效")?
                .to_string_lossy()
                .into_owned(),
            existed: destination.exists(),
        });
        workspaces.push(workspace);
    }
    let journal_path = roots.data.join("restore-journal.json");
    write_json(&journal_path, &journal)?;
    for workspace in workspaces {
        let _ = workspace.keep();
    }
    let result = (|| -> Result<()> {
        for plan in &journal.plans {
            let destination = guard_target(roots, &plan.target)?;
            if plan.target == "data/mytools.db" && destination.exists() {
                let connection = rusqlite::Connection::open(&destination).map_err(error)?;
                connection
                    .busy_timeout(std::time::Duration::from_secs(5))
                    .map_err(error)?;
                connection
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
                    .map_err(error)?;
                drop(connection);
                for suffix in ["-wal", "-shm"] {
                    let sidecar =
                        PathBuf::from(format!("{}{suffix}", destination.to_string_lossy()));
                    if sidecar.exists() {
                        no_link(&sidecar)?;
                        fs::remove_file(sidecar).map_err(error)?;
                    }
                }
            }
            let workspace = destination
                .parent()
                .ok_or("路径无效")?
                .join(&plan.workspace);
            if plan.existed {
                fs::rename(&destination, workspace.join("previous")).map_err(error)?;
            }
            fs::rename(workspace.join("next"), &destination).map_err(error)?;
        }
        journal.committed = true;
        write_json(&journal_path, &journal)?;
        Ok(())
    })();
    if let Err(failure) = result {
        recover(roots).map_err(|rollback| {
            format!(
                "恢复失败：{failure}；回滚失败：{rollback}；安全备份：{}",
                recovery_path.display()
            )
        })?;
        return Err(format!("恢复失败，已回滚：{failure}"));
    }
    let count = journal.plans.len();
    recover(roots)?;
    Ok(Report {
        restored_items: count,
        recovery_backup: recovery_path.to_string_lossy().into_owned(),
    })
}

fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(error)?;
        no_link(entry.path())?;
        let rel = entry.path().strip_prefix(source).map_err(error)?;
        let path = target.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(path).map_err(error)?;
        } else {
            fs::copy(entry.path(), &path).map_err(error)?;
            fs::OpenOptions::new()
                .write(true)
                .open(path)
                .map_err(error)?
                .sync_all()
                .map_err(error)?;
        }
    }
    Ok(())
}

pub fn diagnostic_zip(destination: &Path, summary: &Value, events: &Value) -> Result<()> {
    if !destination
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err("诊断文件必须使用 .zip 扩展名".into());
    }
    let parent = destination.parent().ok_or("诊断文件路径无效")?;
    fs::create_dir_all(parent).map_err(error)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    {
        let mut writer = zip::ZipWriter::new(temp.as_file_mut());
        for (name, value) in [("summary.json", summary), ("operations.json", events)] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .map_err(error)?;
            writer.write_all(&serialize(value)?).map_err(error)?;
        }
        writer.finish().map_err(error)?;
    }
    temp.as_file().sync_all().map_err(error)?;
    temp.persist(destination).map_err(error)?;
    Ok(())
}
