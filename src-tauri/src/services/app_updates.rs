use std::io::Read;
use std::sync::Mutex;
use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use super::settings::{SettingsState, UpdateMode};

const RELEASES: &str = "https://api.github.com/repos/mytools-all/mytools/releases/latest";
const REPOSITORY: &str = "https://github.com/mytools-all/mytools";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    version: String,
    tag: String,
    url: String,
    signed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub status: String,
    pub mode: UpdateMode,
    pub checked_at: Option<String>,
    pub error: Option<String>,
    pub phase: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub can_install: bool,
    pub release_url: Option<String>,
    pub revision: u64,
}

struct Inner {
    snapshot: Snapshot,
    release: Option<Release>,
    #[cfg(windows)]
    downloaded: Option<(tauri_plugin_updater::Update, Vec<u8>)>,
}

pub struct Updates {
    inner: Mutex<Inner>,
    operation: tokio::sync::Mutex<()>,
}

impl Updates {
    pub fn new(app: &tauri::AppHandle) -> Self {
        Self {
            inner: Mutex::new(Inner {
                snapshot: Snapshot {
                    current_version: app.package_info().version.to_string(),
                    latest_version: None,
                    status: "unknown".into(),
                    mode: app.state::<SettingsState>().get().update_mode,
                    checked_at: None,
                    error: None,
                    phase: "idle".into(),
                    downloaded_bytes: 0,
                    total_bytes: None,
                    can_install: false,
                    release_url: None,
                    revision: 0,
                },
                release: None,
                #[cfg(windows)]
                downloaded: None,
            }),
            operation: tokio::sync::Mutex::new(()),
        }
    }

    pub fn snapshot(&self) -> Result<Snapshot, String> {
        Ok(self
            .inner
            .lock()
            .map_err(|e| e.to_string())?
            .snapshot
            .clone())
    }

    fn change(&self, app: &tauri::AppHandle, f: impl FnOnce(&mut Inner)) {
        if let Ok(mut inner) = self.inner.lock() {
            f(&mut inner);
            inner.snapshot.revision += 1;
            for label in ["main", "center"] {
                let _ = app.emit_to(label, "mytools:app-update", inner.snapshot.clone());
            }
        }
    }

    pub fn set_mode(&self, app: &tauri::AppHandle, mode: UpdateMode) -> Result<Snapshot, String> {
        app.state::<SettingsState>().set_update_mode(mode)?;
        self.change(app, |inner| inner.snapshot.mode = mode);
        self.snapshot()
    }

    pub async fn check(&self, app: &tauri::AppHandle) -> Result<Snapshot, String> {
        let Ok(_guard) = self.operation.try_lock() else {
            return self.snapshot();
        };
        self.change(app, |inner| {
            inner.snapshot.status = "checking".into();
            inner.snapshot.error = None;
        });
        let result = tauri::async_runtime::spawn_blocking(fetch_release)
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
        match result {
            Ok(release) => {
                let current = app.package_info().version.clone();
                let latest = Version::parse(&release.version).map_err(|e| e.to_string())?;
                self.change(app, |inner| {
                    if inner.release.as_ref().map(|previous| &previous.version)
                        != Some(&release.version)
                    {
                        #[cfg(windows)]
                        {
                            inner.downloaded = None;
                        }
                        inner.snapshot.phase = "idle".into();
                        inner.snapshot.downloaded_bytes = 0;
                        inner.snapshot.total_bytes = None;
                    }
                    let state = &mut inner.snapshot;
                    state.status = compare(&current, &latest).into();
                    state.latest_version = Some(release.version.clone());
                    state.release_url = Some(release.url.clone());
                    state.checked_at = Some(chrono::Utc::now().to_rfc3339());
                    state.can_install = release.signed && latest > current && cfg!(windows);
                    inner.release = Some(release);
                });
                let snapshot = self.snapshot()?;
                if snapshot.mode == UpdateMode::Download
                    && snapshot.can_install
                    && snapshot.phase != "ready"
                {
                    if let Err(error) = self.download_inner(app).await {
                        self.fail(app, error);
                    }
                }
            }
            Err(error) => self.change(app, |inner| {
                inner.snapshot.status = "error".into();
                inner.snapshot.error = Some(format!("检查更新失败：{error}"));
            }),
        }
        self.snapshot()
    }

    fn fail(&self, app: &tauri::AppHandle, error: String) {
        self.change(app, |inner| {
            inner.snapshot.phase = "idle".into();
            inner.snapshot.error = Some(error);
        });
    }

    pub async fn download(&self, app: &tauri::AppHandle) -> Result<Snapshot, String> {
        let _guard = self
            .operation
            .try_lock()
            .map_err(|_| "更新操作正在进行，请稍候")?;
        if self.snapshot()?.phase == "ready" {
            return self.snapshot();
        }
        if let Err(error) = self.download_inner(app).await {
            self.fail(app, error.clone());
            return Err(error);
        }
        self.snapshot()
    }

    #[cfg(windows)]
    async fn download_inner(&self, app: &tauri::AppHandle) -> Result<(), String> {
        use tauri_plugin_updater::UpdaterExt;
        let release = self
            .inner
            .lock()
            .map_err(|e| e.to_string())?
            .release
            .clone()
            .filter(|release| release.signed)
            .ok_or("此版本未提供应用内更新包，请前往发布页下载")?;
        let endpoint = format!("{REPOSITORY}/releases/download/{}/latest.json", release.tag)
            .parse()
            .map_err(|e| format!("更新地址无效：{e}"))?;
        self.change(app, |inner| {
            inner.snapshot.phase = "downloading".into();
            inner.snapshot.downloaded_bytes = 0;
            inner.snapshot.total_bytes = None;
            inner.snapshot.error = None;
        });
        let update = app
            .updater_builder()
            .endpoints(vec![endpoint])
            .map_err(|e| e.to_string())?
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| e.to_string())?
            .check()
            .await
            .map_err(|e| format!("读取更新包失败：{e}"))?
            .ok_or("未找到适合当前系统的更新包，请重新检查更新")?;
        if update.version != release.version
            || !official_asset(update.download_url.as_str(), &release.tag)
        {
            return Err("更新包的版本或来源与正式发布不一致，已停止下载".into());
        }
        let mut received = 0u64;
        let mut last_emit = std::time::Instant::now();
        let bytes = update
            .download(
                |size, total| {
                    received += size as u64;
                    if last_emit.elapsed() >= Duration::from_millis(250) {
                        self.change(app, |inner| {
                            inner.snapshot.downloaded_bytes = received;
                            inner.snapshot.total_bytes = total;
                        });
                        last_emit = std::time::Instant::now();
                    }
                },
                || {},
            )
            .await
            .map_err(|e| format!("下载或签名校验失败：{e}"))?;
        self.change(app, |inner| {
            inner.snapshot.phase = "ready".into();
            inner.snapshot.downloaded_bytes = bytes.len() as u64;
            inner.snapshot.total_bytes = Some(bytes.len() as u64);
            inner.downloaded = Some((update, bytes));
        });
        Ok(())
    }

    #[cfg(not(windows))]
    async fn download_inner(&self, _app: &tauri::AppHandle) -> Result<(), String> {
        Err("当前平台暂不支持应用内更新".into())
    }

    #[cfg(windows)]
    pub async fn install(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let _guard = self
            .operation
            .try_lock()
            .map_err(|_| "更新操作正在进行，请稍候")?;
        let downloaded = self
            .inner
            .lock()
            .map_err(|e| e.to_string())?
            .downloaded
            .take()
            .ok_or("请先下载更新包")?;
        self.change(app, |inner| {
            inner.snapshot.phase = "installing".into();
            inner.snapshot.error = None;
        });
        let result = tauri::async_runtime::spawn_blocking(move || {
            downloaded
                .0
                .install(downloaded.1)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
        if let Err(error) = result {
            self.fail(app, format!("安装更新失败：{error}"));
            return Err(error);
        }
        Ok(())
    }

    #[cfg(not(windows))]
    pub async fn install(&self, _app: &tauri::AppHandle) -> Result<(), String> {
        Err("当前平台暂不支持应用内更新".into())
    }
}

fn compare(current: &Version, latest: &Version) -> &'static str {
    match current.cmp(latest) {
        std::cmp::Ordering::Less => "available",
        std::cmp::Ordering::Equal => "upToDate",
        std::cmp::Ordering::Greater => "localNewer",
    }
}

fn official_asset(url: &str, tag: &str) -> bool {
    url.starts_with(&format!("{REPOSITORY}/releases/download/{tag}/")) && !url.contains(['?', '#'])
}

fn parse_release(value: &serde_json::Value) -> Result<Release, String> {
    if value["draft"].as_bool() != Some(false) || value["prerelease"].as_bool() != Some(false) {
        return Err("更新源未返回正式版本".into());
    }
    let tag = value["tag_name"].as_str().ok_or("发布版本号缺失")?;
    let raw_version = tag.strip_prefix('v').unwrap_or(tag);
    let version = Version::parse(raw_version).map_err(|_| "发布版本号无效")?;
    if !version.pre.is_empty() || !version.build.is_empty() || raw_version != version.to_string() {
        return Err("发布版本号不是正式版本".into());
    }
    let url = format!("{REPOSITORY}/releases/tag/{tag}");
    let assets = value["assets"].as_array().ok_or("发布文件列表缺失")?;
    let signed = assets.iter().any(|asset| {
        asset["name"].as_str() == Some("latest.json")
            && asset["browser_download_url"].as_str()
                == Some(format!("{REPOSITORY}/releases/download/{tag}/latest.json").as_str())
    });
    Ok(Release {
        version: version.to_string(),
        tag: tag.into(),
        url,
        signed,
    })
}

fn fetch_release() -> Result<Release, String> {
    let response = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build()
        .get(RELEASES)
        .set("User-Agent", "MyTools-update-check")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("发布信息过大".into());
    }
    let value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    parse_release(&value)
}

pub fn initialize(app: &tauri::AppHandle) {
    app.manage(Updates::new(app));
    if app.state::<SettingsState>().get().update_mode != UpdateMode::Disabled {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = app.state::<Updates>().check(&app).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> serde_json::Value {
        serde_json::json!({"tag_name":tag,"draft":false,"prerelease":false,"assets":[]})
    }

    #[test]
    fn distinguishes_newer_same_and_local_versions() {
        let current = Version::parse("0.2.0").expect("version");
        assert_eq!(
            compare(&current, &Version::parse("0.1.0").expect("version")),
            "localNewer"
        );
        assert_eq!(compare(&current, &current), "upToDate");
        assert_eq!(
            compare(&current, &Version::parse("0.3.0").expect("version")),
            "available"
        );
    }

    #[test]
    fn rejects_invalid_or_preview_releases() {
        for tag in ["../../payload", "v0.2.0-beta.1", "v0.2.0+test", "01.2.0"] {
            assert!(parse_release(&release(tag)).is_err());
        }
        let mut value = release("v0.2.0");
        value["prerelease"] = true.into();
        assert!(parse_release(&value).is_err());
    }

    #[test]
    fn legacy_release_does_not_claim_signed_updates() {
        assert!(!parse_release(&release("v0.1.0")).expect("release").signed);
        let mut value = release("v0.2.0");
        value["assets"] = serde_json::json!([{"name":"latest.json","browser_download_url":"https://evil.test/latest.json"}]);
        assert!(!parse_release(&value).expect("release").signed);
        value["assets"][0]["browser_download_url"] =
            format!("{REPOSITORY}/releases/download/v0.2.0/latest.json").into();
        assert!(parse_release(&value).expect("release").signed);
        assert!(!official_asset(
            "https://github.com/mytools-all/mytools/releases/download/v0.2.0/x.exe?redirect=x",
            "v0.2.0"
        ));
    }
}
