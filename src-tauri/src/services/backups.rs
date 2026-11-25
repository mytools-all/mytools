use crate::services::settings::{AppSettings, SettingsState};
use mytools_core::backup::{self, Manifest, RestoreOptions, Roots, Selection};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

struct Prepared {
    token: String,
    file: tempfile::NamedTempFile,
    manifest: Manifest,
}
#[derive(Default)]
pub struct Backups {
    prepared: Mutex<Option<Prepared>>,
    pub operation: tokio::sync::Mutex<()>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pending {
    version: u32,
    options: RestoreOptions,
}

pub fn roots(app: &AppHandle) -> Roots {
    let config = crate::infrastructure::paths::config_root(app);
    let data = crate::infrastructure::paths::data_root(app);
    let plugins = crate::services::plugins::effective_plugins_root(
        &app.state::<SettingsState>().get(),
        &data,
    );
    Roots {
        config,
        data,
        plugins,
    }
}
fn logs(app: &AppHandle) -> PathBuf {
    crate::infrastructure::paths::data_root(app).join("logs")
}
pub fn audit(app: &AppHandle, operation: &str, target: Option<&str>, success: bool) {
    super::audit::record(&logs(app), operation, target, success)
}

pub fn info(app: &AppHandle) -> Result<serde_json::Value, String> {
    let roots = roots(app);
    let result = roots.data.join("restore-result.json");
    let result = if result.exists() {
        serde_json::from_slice::<serde_json::Value>(&fs::read(result).map_err(|e| e.to_string())?)
            .ok()
    } else {
        None
    };
    Ok(
        serde_json::json!({"plugins":backup::plugins(&roots)?,"lastRestore":result,"logs":logs(app),"recentOperations":super::audit::recent(&logs(app))}),
    )
}

pub fn export(app: &AppHandle, selection: Selection, path: PathBuf) -> Result<Manifest, String> {
    let roots = roots(app);
    app.store(roots.config.join("plugin-store.json"))
        .map_err(|e| e.to_string())?
        .save()
        .map_err(|e| e.to_string())?;
    let registry = app.state::<super::plugins::SharedPluginRegistry>();
    let _guard = registry.lock_mutation()?;
    let settings = app.state::<SettingsState>();
    let _settings_guard = settings.snapshot_guard()?;
    let result = backup::create(
        &roots,
        &app.package_info().version.to_string(),
        &selection,
        &path,
    );
    audit(app, "backup.export", None, result.is_ok());
    result
}

impl Backups {
    pub fn preview(&self, app: &AppHandle, path: &Path) -> Result<serde_json::Value, String> {
        let mut file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        let mut input = fs::File::open(path).map_err(|e| e.to_string())?;
        if input.metadata().map_err(|e| e.to_string())?.len() > 2 * 1024 * 1024 * 1024 {
            return Err("备份文件超过 2 GiB".into());
        }
        std::io::copy(&mut input, &mut file).map_err(|e| e.to_string())?;
        let manifest = backup::inspect(file.path(), &app.package_info().version.to_string())?;
        let token = file
            .path()
            .file_name()
            .ok_or("预览文件无效")?
            .to_string_lossy()
            .into_owned();
        let registry = app.state::<super::plugins::SharedPluginRegistry>();
        let installed = registry.list();
        let warnings: Vec<String> = manifest
            .plugins
            .iter()
            .filter_map(|plugin| {
                match installed
                    .iter()
                    .find(|installed| installed.manifest.name == plugin.id)
                {
                    None => Some(format!("{} 尚未安装，恢复后需要安装相应插件", plugin.title)),
                    Some(installed)
                        if plugin
                            .version
                            .as_ref()
                            .is_some_and(|version| version != &installed.manifest.version) =>
                    {
                        Some(format!(
                            "{} 的备份版本为 {}，当前为 {}",
                            plugin.title,
                            plugin.version.as_deref().unwrap_or("未知"),
                            installed.manifest.version
                        ))
                    }
                    _ => None,
                }
            })
            .collect();
        let roots = roots(app);
        let existing = roots.config.join("settings.json").exists()
            || roots.config.join("plugin-store.json").exists()
            || roots.data.join("mytools.db").exists();
        let value = serde_json::json!({"token":token,"manifest":{"hostVersion":manifest.host_version,"createdAt":manifest.created_at,"plugins":manifest.plugins,"filesCount":manifest.files.len(),"bytes":manifest.files.iter().map(|file|file.size).sum::<u64>(),"hasHost":manifest.files.iter().any(|file|file.path=="host/settings.json"),"hasClipboard":manifest.files.iter().any(|file|file.path=="clipboard/mytools.db"),"hasPrograms":manifest.files.iter().any(|file|file.path.starts_with("programs/"))},"warnings":warnings,"hasExistingData":existing});
        *self.prepared.lock().map_err(|e| e.to_string())? = Some(Prepared {
            token,
            file,
            manifest,
        });
        audit(app, "backup.preview", None, true);
        Ok(value)
    }

    pub fn cancel(&self, token: &str) {
        if let Ok(mut prepared) = self.prepared.lock() {
            if prepared
                .as_ref()
                .is_some_and(|preview| preview.token == token)
            {
                prepared.take();
            }
        }
    }

    pub fn queue(
        &self,
        app: &AppHandle,
        token: &str,
        options: RestoreOptions,
    ) -> Result<(), String> {
        let mut guard = self.prepared.lock().map_err(|e| e.to_string())?;
        let prepared = guard.as_ref().ok_or("请先选择并预览备份")?;
        if prepared.token != token {
            return Err("备份预览已变化，请重新预览".into());
        }
        if options.plugins.iter().any(|id| {
            !prepared
                .manifest
                .plugins
                .iter()
                .any(|plugin| &plugin.id == id)
        }) {
            return Err("选择中包含备份之外的插件".into());
        }
        if !options.host && !options.clipboard && options.plugins.is_empty() {
            return Err("请至少选择一项恢复内容".into());
        }
        if options.host
            && !prepared
                .manifest
                .files
                .iter()
                .any(|file| file.path == "host/settings.json")
        {
            return Err("备份不含主程序配置".into());
        }
        if options.clipboard
            && !prepared
                .manifest
                .files
                .iter()
                .any(|file| file.path == "clipboard/mytools.db")
        {
            return Err("备份不含剪贴板记录".into());
        }
        let roots = roots(app);
        let directory = roots.data.join("pending-restore");
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        if directory.join("request.json").exists() {
            return Err("已有等待执行的恢复任务".into());
        }
        let mut archive = tempfile::NamedTempFile::new_in(&directory).map_err(|e| e.to_string())?;
        let mut source = fs::File::open(prepared.file.path()).map_err(|e| e.to_string())?;
        std::io::copy(&mut source, &mut archive).map_err(|e| e.to_string())?;
        archive.as_file().sync_all().map_err(|e| e.to_string())?;
        archive
            .persist(directory.join("backup.zip"))
            .map_err(|e| e.to_string())?;
        let mut request = tempfile::NamedTempFile::new_in(&directory).map_err(|e| e.to_string())?;
        request
            .write_all(
                &serde_json::to_vec(&Pending {
                    version: 1,
                    options,
                })
                .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        request.as_file().sync_all().map_err(|e| e.to_string())?;
        request
            .persist(directory.join("request.json"))
            .map_err(|e| e.to_string())?;
        guard.take();
        audit(app, "backup.restore_queued", None, true);
        Ok(())
    }
}

pub fn last_result(app: &AppHandle) -> Option<serde_json::Value> {
    fs::read(crate::infrastructure::paths::data_root(app).join("restore-result.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

pub fn diagnostics(app: &AppHandle, path: &Path) -> Result<(), String> {
    let roots = roots(app);
    if let Some(parent) = path.parent() {
        if let Ok(parent) = parent.canonicalize() {
            for root in [
                &roots.config,
                &roots.plugins,
                &roots.data.join("plugin-files"),
            ] {
                if root
                    .canonicalize()
                    .is_ok_and(|root| parent.starts_with(root))
                {
                    return Err("请将诊断文件保存到配置和插件目录之外".into());
                }
            }
        }
    }
    let registry = app.state::<super::plugins::SharedPluginRegistry>();
    let summary = serde_json::json!({"app":"MyTools","version":app.package_info().version.to_string(),"createdAt":chrono::Utc::now().to_rfc3339(),"pluginCount":registry.list().len(),"os":std::env::consts::OS,"architecture":std::env::consts::ARCH});
    let result = backup::diagnostic_zip(
        path,
        &summary,
        &serde_json::to_value(super::audit::recent(&logs(app))).map_err(|e| e.to_string())?,
    );
    audit(app, "backup.diagnostics", None, result.is_ok());
    result
}

pub fn apply_pending(config: &Path, data: &Path, host: &str) -> Result<(), String> {
    let settings = SettingsState::load(config.join("settings.json"));
    let roots = Roots {
        config: config.into(),
        data: data.into(),
        plugins: super::plugins::effective_plugins_root(&settings.get(), data),
    };
    backup::recover(&roots)?;
    let directory = data.join("pending-restore");
    let request = directory.join("request.json");
    if !request.exists() {
        return Ok(());
    }
    let pending: Pending = serde_json::from_slice(&fs::read(&request).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if pending.version != 1 {
        return Err("待恢复任务的版本不受支持".into());
    }
    let result = backup::restore(
        &directory.join("backup.zip"),
        &roots,
        host,
        &pending.options,
        &|value| {
            let settings: AppSettings =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            settings.catalog_sources.validate()?;
            if settings.hotkey.trim().is_empty() {
                return Err("备份中的快捷键无效".into());
            }
            Ok(())
        },
    );
    let report = match &result {
        Ok(report) => {
            serde_json::json!({"success":true,"at":chrono::Utc::now().to_rfc3339(),"report":report})
        }
        Err(error) => {
            serde_json::json!({"success":false,"at":chrono::Utc::now().to_rfc3339(),"error":error})
        }
    };
    fs::write(
        data.join("restore-result.json"),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    super::audit::record(&data.join("logs"), "backup.restore", None, result.is_ok());
    fs::remove_file(request).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(directory.join("backup.zip"));
    if result.is_err() && data.join("restore-journal.json").exists() {
        return Err("恢复回滚尚未完成，已停止启动以保留恢复现场".into());
    }
    Ok(())
}
