//! 应用索引 —— 启动时扫描一次，进程内复用，过期后自动刷新。
//!
//! 三路来源合并成一张统一表：
//! 1. 开始菜单 `.lnk`（含桌面）
//! 2. `Uninstall` 注册表 —— 传统第三方应用，装完即出现，零维护
//! 3. `shell:AppsFolder` COM 枚举 —— Store / Windows 内置应用
//!
//! 第三路补上了同一个死角：记事本、计算器、画图这类应用既不在开始菜单
//! 生成 `.lnk`，也不注册到 `Uninstall`，只扫 `.lnk` 的启动器永远搜不到它们。
//!
//! ## 扩展性设计
//!
//! `shell:AppsFolder` 里每条记录的标识符有四种语法形态：
//!
//! | 形态 | 样例 | 本机条目数 |
//! |---|---|---|
//! | 真 AUMID | `Microsoft.WindowsNotepad_8wekyb3d8bbwe!App` | 42 |
//! | 已知文件夹路径 | `{1AC14E77-...}\cmd.exe` | 59 |
//! | 裸伪 AUMID | `Microsoft.VisualStudioCode` | 49 |
//! | 纯路径 | `C:\msys64\msys2.exe` | 68 |
//!
//! 分类判断是个陷阱：靠「含 `!`」只能认出第一类，另外三类会被误判。
//! 但 shell 自己认识全部四种 —— `SHCreateItemFromParsingName` 对每种形态
//! 都能解析成功。所以本模块**不做形态分类**，只记录条目的「身份」和
//! 「怎么启动」，图标和启动各自去问 shell 要答案。
//!
//! 这样新增来源只要能表达成 [`TargetKind`] 里的某一种，就自动被支持，
//! 无需改动任何解析代码。
//!
//! Windows 设置页子项（`ms-settings:display` 之类）无法枚举，也不再硬编码；
//! 设置应用本身由 `shell:AppsFolder` 提供（显示名「设置」）。

use serde::Serialize;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

/// 应用来源，同时决定去重时的优先级。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AppSource {
    StartMenu,
    Desktop,
    AppsFolder,
    Registry,
}

impl AppSource {
    /// 越小越优先。去重时同名/同 target 保留排名靠前的。
    pub fn rank(self) -> u8 {
        match self {
            AppSource::StartMenu => 0,
            AppSource::Desktop => 1,
            AppSource::AppsFolder => 2,
            AppSource::Registry => 3,
        }
    }

    /// 面向用户的一行说明，显示在搜索结果里。
    pub fn label(self) -> &'static str {
        match self {
            AppSource::StartMenu => "app (start menu)",
            AppSource::Desktop => "app (desktop)",
            AppSource::AppsFolder => "app (system)",
            AppSource::Registry => "app (installed)",
        }
    }
}

/// 启动目标的形态。决定「怎么取图标」和「怎么启动」。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    /// 磁盘上的路径 —— `.lnk` / `.exe` / `.msc` 等，走 ShellExecute。
    FilePath,
    /// `shell:AppsFolder` 里的条目标识符。四种语法形态都归此类，
    /// 启动和取图标都交给 shell。
    ShellItem,
}

/// 一条应用记录。
///
/// `target` 是稳定主键：前端用它做 pin key，IPC 用它查图标和启动方式。
/// `name` 是 shell 给的显示名（中文），不能从 `target` 反推 —— AUMID 里
/// 没有分隔符，反推出来的是整串标识符而不是「记事本」。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedApp {
    pub name: String,
    pub target: String,
    pub kind: TargetKind,
    pub source: AppSource,
}

impl IndexedApp {
    fn new(
        name: impl Into<String>,
        target: impl Into<String>,
        kind: TargetKind,
        source: AppSource,
    ) -> Self {
        Self {
            name: name.into(),
            target: target.into(),
            kind,
            source,
        }
    }
}

/// 归一化 target 供去重比较：去空白、统一反斜杠、大小写不敏感。
pub fn normalize_target(t: &str) -> String {
    t.trim().replace('/', "\\").to_ascii_lowercase()
}

/// 归一化名称供去重比较：去空白、大小写不敏感，并剥掉安装器附加的
/// 版本/架构后缀。
///
/// 注册表里的 `DisplayName` 常带后缀（`Microsoft Visual Studio Code (User)`）
/// 而开始菜单里只有 `Visual Studio Code`，两者本该是同一个应用。
pub fn normalize_name(n: &str) -> String {
    let mut s = n.trim().to_lowercase();
    // 剥掉尾部括号里的补充说明：`foo (user)` / `foo (x64)` / `foo (64 位)`
    if let Some(open) = s.rfind('(') {
        if s.ends_with(')') && open > 0 {
            s.truncate(open);
        }
    }
    // 剥掉版本号尾巴：`foo 1.2.3` / `foo v1.2`。
    // 裸数字必须带小数点才算版本 —— 否则 `Visual Studio 2022` 会和
    // `Visual Studio 2019` 一起被削成 `visual studio`，误合并两个不同应用。
    if let Some(pos) = s.rfind(' ') {
        let tail = &s[pos + 1..];
        if tail.starts_with('v') || tail.starts_with('V') {
            let digits = tail.trim_start_matches(['v', 'V']);
            if !digits.is_empty() && digits.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                s.truncate(pos);
            }
        } else if tail.chars().next().is_some_and(|c| c.is_ascii_digit()) && tail.contains('.') {
            s.truncate(pos);
        }
    }
    s.trim().to_string()
}

/// 名称噪声特征，决定注册表条目是否丢弃。
///
/// `Uninstall` 键里混着大量补丁和内部组件（219 条实测里将近三成），直接
/// 展示会污染搜索结果。
pub struct UninstallEntry {
    pub display_name: String,
    pub system_component: bool,
    pub has_parent: bool,
    pub release_type: Option<u32>,
    pub is_update: bool,
}

/// 卸载程序本身不是应用。
fn looks_like_uninstaller(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("uninstall")
        || n.contains("卸载")
        || n.contains("卸载程序")
        || n.ends_with("卸载.exe")
}

/// 纯函数过滤规则，便于单测。
pub fn keep_uninstall_entry(e: &UninstallEntry) -> bool {
    let name = e.display_name.trim();
    if name.is_empty() {
        return false;
    }
    // `{7B1C...}-setup` 这类 GUID 内部组件
    if name.starts_with('{') {
        return false;
    }
    if e.system_component {
        return false;
    }
    // 有父键 = 补丁 / 子条目（父键存在即视为从属项）
    if e.has_parent {
        return false;
    }
    // ReleaseType >= 2 是更新 / 安全更新包（0 与 1 都是主程序）
    if e.is_update || e.release_type.is_some_and(|r| r >= 2) {
        return false;
    }
    if looks_like_uninstaller(name) {
        return false;
    }
    true
}

/// 合并去重：同 target 或同名（小写、去空白）只保留优先级最高的一条。
pub fn merge(mut apps: Vec<IndexedApp>) -> Vec<IndexedApp> {
    apps.retain(|a| {
        let n = a.name.trim();
        !n.is_empty() && !a.target.trim().is_empty()
    });
    // 先按 rank 排，保证「保留第一条」= 保留最高优先级
    apps.sort_by(|a, b| {
        a.source
            .rank()
            .cmp(&b.source.rank())
            .then_with(|| normalize_target(&a.target).cmp(&normalize_target(&b.target)))
            .then_with(|| a.name.cmp(&b.name))
    });
    let mut seen_target: HashSet<String> = HashSet::new();
    let mut seen_name: HashSet<String> = HashSet::new();
    let mut out = Vec::with_capacity(apps.len());
    for a in apps {
        let t = normalize_target(&a.target);
        let n = normalize_name(&a.name);
        // 两个集合都要写入。用 `||` 短路会漏写：target 重复时
        // `seen_name` 根本不会被插入，于是「同名但 target 不同」的条目
        // 就能混进来（VSCode 的「Visual Studio Code」和
        // 「Microsoft Visual Studio Code (User)」就是这样）。
        let target_new = seen_target.insert(t);
        let name_new = seen_name.insert(n);
        if !target_new || !name_new {
            continue;
        }
        out.push(a);
    }
    out
}

pub struct AppIndex {
    pub apps: Vec<IndexedApp>,
    /// 完成扫描的时间；`None` 表示尚未扫描。
    scanned_at: Option<Instant>,
}

impl AppIndex {
    fn collect() -> Self {
        let mut raw = Vec::new();
        raw.extend(scan_start_menu());
        raw.extend(scan_registry());
        raw.extend(scan_apps_folder());
        Self {
            apps: merge(raw),
            scanned_at: Some(Instant::now()),
        }
    }

    fn is_stale(&self, ttl: Duration, now: Instant) -> bool {
        is_stale(self.scanned_at, ttl, now)
    }
}

/// `scanned_at` 为空，或距上次扫描已达 `ttl`，即视为过期。
pub fn is_stale(scanned_at: Option<Instant>, ttl: Duration, now: Instant) -> bool {
    match scanned_at {
        None => true,
        Some(t) => now.duration_since(t) >= ttl,
    }
}

static INDEX: OnceLock<RwLock<Arc<AppIndex>>> = OnceLock::new();
/// 保证同一时刻只有一次扫描，避免并发重复扫盘。
static REFRESH_LOCK: Mutex<()> = Mutex::new(());
/// 索引有效期。启动器每次获得焦点时用它判断是否需要后台刷新。
pub const INDEX_TTL: Duration = Duration::from_secs(300);

fn slot() -> &'static RwLock<Arc<AppIndex>> {
    INDEX.get_or_init(|| {
        RwLock::new(Arc::new(AppIndex {
            apps: Vec::new(),
            scanned_at: None,
        }))
    })
}

/// Cheap snapshot handle: cloning an `Arc` costs one atomic increment, which
/// matters because `search` runs this on every keystroke.
fn current() -> Arc<AppIndex> {
    slot().read().unwrap_or_else(|e| e.into_inner()).clone()
}

fn lock_refresh() -> std::sync::MutexGuard<'static, ()> {
    REFRESH_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Scan and swap the snapshot in one step. Must be called with `REFRESH_LOCK`
/// held. Swapping after the scan (instead of clearing first) keeps readers on
/// the previous snapshot instead of briefly seeing an empty index.
fn scan_and_store() -> Arc<AppIndex> {
    let scanned = Arc::new(AppIndex::collect());
    *slot().write().unwrap_or_else(|e| e.into_inner()) = scanned.clone();
    scanned
}

/// 进程内只扫描一次；首次调用阻塞完成，后续直接返回。
pub fn get_or_scan() -> Arc<AppIndex> {
    let existing = current();
    if existing.scanned_at.is_some() {
        return existing;
    }
    let _guard = lock_refresh();
    let existing = current();
    if existing.scanned_at.is_some() {
        return existing;
    }
    scan_and_store()
}

/// 启动时后台预热，让首次搜索不必等扫描。
pub fn warm() {
    tauri::async_runtime::spawn_blocking(|| {
        let _ = get_or_scan();
    });
}

/// 强制重新扫描（供手动刷新用）。
pub fn refresh() -> Arc<AppIndex> {
    let _guard = lock_refresh();
    scan_and_store()
}

/// 过期才重扫；返回是否真的扫描了。另一次刷新正在进行时直接返回
/// `false` —— 对方扫完就是最新结果。
pub fn refresh_if_stale(ttl: Duration) -> bool {
    if !current().is_stale(ttl, Instant::now()) {
        return false;
    }
    let Ok(_guard) = REFRESH_LOCK.try_lock() else {
        return false;
    };
    if !current().is_stale(ttl, Instant::now()) {
        return false;
    }
    scan_and_store();
    true
}

fn scan_start_menu() -> Vec<IndexedApp> {
    let mut out: Vec<IndexedApp> = mytools_core::scan_start_menu()
        .into_iter()
        .map(|e| IndexedApp::new(e.name, e.path, TargetKind::FilePath, AppSource::StartMenu))
        .collect();
    out.extend(scan_desktop());
    out
}

fn scan_desktop() -> Vec<IndexedApp> {
    let mut dirs = Vec::new();
    if let Ok(home) = std::env::var("USERPROFILE") {
        dirs.push(format!("{home}\\Desktop"));
        dirs.push(format!("{home}\\OneDrive\\Desktop"));
    }
    if let Ok(pub_) = std::env::var("PUBLIC") {
        dirs.push(format!("{pub_}\\Desktop"));
    }
    let mut out = Vec::new();
    for dir in dirs {
        for entry in walkdir::WalkDir::new(&dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".lnk")
            {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            out.push(IndexedApp::new(
                stem,
                path.to_string_lossy().into_owned(),
                TargetKind::FilePath,
                AppSource::Desktop,
            ));
        }
    }
    out
}

// ─────────────────────────── Uninstall 注册表 ───────────────────────────

/// First `.exe` sitting directly inside `dir`, used as a last-resort icon
/// source for registry entries that carry no `DisplayIcon`.
fn first_exe_in(dir: &std::path::Path) -> Option<String> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut fallback = None;
    for entry in rd.flatten() {
        let p = entry.path();
        let Some(ext) = p.extension() else { continue };
        if !ext.eq_ignore_ascii_case("exe") {
            continue;
        }
        let s = p.to_string_lossy().into_owned();
        // Prefer something that looks like the main program over `setup.exe`
        // or an updater shim.
        let stem = p
            .file_stem()
            .map(|x| x.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if stem.contains("setup") || stem.contains("update") || stem.contains("uninstall") {
            fallback.get_or_insert(s);
            continue;
        }
        return Some(s);
    }
    fallback
}

/// 从一条 Uninstall 记录构造应用条目；`display_icon` 形如
/// `C:\app\app.exe,0`，由调用方解析出图标来源。
pub fn registry_app(
    display_name: &str,
    display_icon: Option<&str>,
    install_location: Option<&str>,
    source_icon_index: i32,
) -> Option<IndexedApp> {
    if !keep_uninstall_entry(&UninstallEntry {
        display_name: display_name.to_string(),
        system_component: false,
        has_parent: false,
        release_type: None,
        is_update: false,
    }) {
        return None;
    }
    let icon = display_icon
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            let (file, idx) = crate::infrastructure::app_icons::split_icon_location_pub(s);
            (file, if idx == 0 { source_icon_index } else { idx })
        })
        .or_else(|| {
            // No DisplayIcon. `DisplayName` is almost never the executable's
            // file name ("CC Switch" ships `cc-switch.exe`), so try the naive
            // guess first and then fall back to whatever executable actually
            // sits in the install folder.
            install_location
                .filter(|l| !l.trim().is_empty())
                .and_then(|l| {
                    let base = l.trim_end_matches(['\\', '/']);
                    let guess = format!("{base}\\{}.exe", display_name);
                    if std::fs::metadata(&guess).is_ok() {
                        return Some((guess, 0));
                    }
                    first_exe_in(std::path::Path::new(base)).map(|f| (f, 0))
                })
        })?;
    if std::fs::metadata(&icon.0).is_err() {
        return None;
    }
    Some(IndexedApp::new(
        display_name.trim(),
        icon.0,
        TargetKind::FilePath,
        AppSource::Registry,
    ))
}

#[cfg(windows)]
fn scan_registry() -> Vec<IndexedApp> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
        KEY_READ,
    };

    const ROOTS: &[(&str, HKEY)] = &[
        (
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            HKEY_LOCAL_MACHINE,
        ),
        (
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            HKEY_LOCAL_MACHINE,
        ),
        (
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            HKEY_CURRENT_USER,
        ),
    ];

    fn read_wide(hkey: HKEY, name: &str) -> Option<String> {
        use windows_sys::Win32::System::Registry::{RegQueryValueExW, REG_VALUE_TYPE};
        let wname: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut size: u32 = 0;
        let mut kind: REG_VALUE_TYPE = 0;
        let rc = unsafe {
            RegQueryValueExW(
                hkey,
                wname.as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut size,
            )
        };
        if rc != 0 || !(2..=64 * 1024).contains(&size) {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let mut size2: u32 = size;
        let rc2 = unsafe {
            RegQueryValueExW(
                hkey,
                wname.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                buf.as_mut_ptr(),
                &mut size2,
            )
        };
        if rc2 != 0 {
            return None;
        }
        // REG_SZ is UTF-16LE; odd trailing byte (shouldn't happen) is dropped.
        let bytes = &buf[..(size2 as usize).min(buf.len()) & !1];
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let s = String::from_utf16_lossy(&units);
        let s = s.trim_end_matches('\0').trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    fn read_u32(hkey: HKEY, name: &str) -> Option<u32> {
        use windows_sys::Win32::System::Registry::RegQueryValueExW;
        let wname: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut out: u32 = 0;
        let mut size: u32 = std::mem::size_of::<u32>() as u32;
        let rc = unsafe {
            RegQueryValueExW(
                hkey,
                wname.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                (&mut out as *mut u32).cast::<u8>(),
                &mut size,
            )
        };
        (rc == 0).then_some(out)
    }

    let mut out = Vec::new();
    for (subpath, root) in ROOTS {
        let wpath: Vec<u16> = subpath.encode_utf16().chain(Some(0)).collect();
        let mut hkey: HKEY = std::ptr::null_mut();
        if unsafe { RegOpenKeyExW(*root, wpath.as_ptr(), 0, KEY_READ, &mut hkey) } != 0 {
            continue;
        }
        let mut index: u32 = 0;
        loop {
            let mut name_buf = [0u16; 512];
            let mut name_len: u32 = name_buf.len() as u32;
            let rc = unsafe {
                RegEnumKeyExW(
                    hkey,
                    index,
                    name_buf.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if rc != 0 {
                break;
            }
            index += 1;
            let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            let wname: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let mut sub: HKEY = std::ptr::null_mut();
            if unsafe { RegOpenKeyExW(hkey, wname.as_ptr(), 0, KEY_READ, &mut sub) } != 0 {
                continue;
            }
            let display_name = read_wide(sub, "DisplayName");
            let has_parent = read_wide(sub, "ParentKeyName").is_some();
            let system_component = read_u32(sub, "SystemComponent").unwrap_or(0) == 1;
            let release_type = read_u32(sub, "ReleaseType");
            let is_update = read_u32(sub, "NoModify").unwrap_or(0) == 1;
            let display_icon = read_wide(sub, "DisplayIcon");
            let install_location = read_wide(sub, "InstallLocation");
            unsafe { RegCloseKey(sub) };

            let Some(display_name) = display_name else {
                continue;
            };
            let keep = keep_uninstall_entry(&UninstallEntry {
                display_name: display_name.clone(),
                system_component,
                has_parent,
                release_type,
                is_update,
            });
            if !keep {
                continue;
            }
            if let Some(app) = registry_app(
                &display_name,
                display_icon.as_deref(),
                install_location.as_deref(),
                0,
            ) {
                out.push(app);
            }
        }
        unsafe { RegCloseKey(hkey) };
    }
    out
}

#[cfg(not(windows))]
fn scan_registry() -> Vec<IndexedApp> {
    Vec::new()
}

// ─────────────────────── shell:AppsFolder（Store 应用） ───────────────────────

/// `shell:AppsFolder` 里每条记录都带一个 `PKEY_AppUserModel_ID`，但那个值
/// 的语法形态有四种（真 AUMID / 已知文件夹路径 / 裸伪 AUMID / 纯路径）。
///
/// 我们不分类 —— shell 自己全认。存下这个标识符，取图标和启动时都原样
/// 交给 `shell:AppsFolder\<id>` 解析即可。
#[cfg(windows)]
fn scan_apps_folder() -> Vec<IndexedApp> {
    use windows::core::{Interface, PWSTR};
    use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
    use windows::Win32::System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, IBindCtx, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, IShellItem2,
        SHGetKnownFolderItem, KF_FLAG_DEFAULT, SIGDN_NORMALDISPLAY,
    };

    const RPC_E_CHANGED_MODE: i32 = 0x8001_010Eu32 as i32;

    fn pwstr_to_string(p: PWSTR) -> Option<String> {
        let s = unsafe { p.to_string() }.ok()?;
        unsafe { CoTaskMemFree(Some(p.0.cast())) };
        Some(s)
    }

    let mut uninit = false;
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_ok() {
        uninit = true;
    } else if hr.0 != RPC_E_CHANGED_MODE {
        return Vec::new();
    }

    let out = (|| {
        let folder: IShellItem =
            match unsafe { SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) } {
                Ok(i) => i,
                Err(_) => return Vec::new(),
            };
        let items: IEnumShellItems =
            match unsafe { folder.BindToHandler(None::<&IBindCtx>, &BHID_EnumItems) } {
                Ok(i) => i,
                Err(_) => return Vec::new(),
            };

        let mut acc = Vec::new();
        loop {
            let mut one: [Option<IShellItem>; 1] = [None];
            let mut got: u32 = 0;
            if unsafe { items.Next(&mut one, Some(&mut got)) }.is_err() || got == 0 {
                break;
            }
            let Some(item) = one.into_iter().next().flatten() else {
                continue;
            };
            // 显示名由 shell 给 —— 「记事本」「设置」「命令提示符」这些
            // 中文名只能从这里来，从标识符反推是反推不出来的。
            let name = unsafe { item.GetDisplayName(SIGDN_NORMALDISPLAY) }
                .ok()
                .and_then(pwstr_to_string);
            let Some(name) = name.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) else {
                continue;
            };
            let id = match item.cast::<IShellItem2>() {
                Ok(item2) => unsafe { item2.GetString(&PKEY_AppUserModel_ID) }
                    .ok()
                    .and_then(pwstr_to_string),
                Err(_) => None,
            };
            let Some(id) = id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) else {
                continue;
            };
            acc.push(IndexedApp::new(
                name,
                id,
                TargetKind::ShellItem,
                AppSource::AppsFolder,
            ));
        }
        acc
    })();

    if uninit {
        unsafe { CoUninitialize() };
    }
    out
}

#[cfg(not(windows))]
fn scan_apps_folder() -> Vec<IndexedApp> {
    Vec::new()
}

/// Cheap read-only handle for `search` / `app_launcher` to iterate.
pub fn apps() -> Arc<AppIndex> {
    get_or_scan()
}

/// 按 target 查一条记录。
///
/// 前端的 pin key 就是 target（`app:<target>`），磁贴要显示的名字必须从
/// 索引里查回来 —— 不能从 target 字符串反推：AUMID 里没有分隔符，
/// `split(/[\\/]/).pop()` 取出来的是整串标识符而不是「记事本」。
pub fn find_by_target(target: &str) -> Option<IndexedApp> {
    let want = normalize_target(target);
    apps()
        .apps
        .iter()
        .find(|a| normalize_target(&a.target) == want)
        .cloned()
}

/// 一条应用的展示信息：显示名 + 图标。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedApp {
    pub name: String,
    pub subtitle: String,
    pub icon: Option<String>,
}

/// 解析一条应用的展示信息。
pub fn resolve(target: &str) -> Option<ResolvedApp> {
    let entry = find_by_target(target)?;
    Some(ResolvedApp {
        name: entry.name.clone(),
        subtitle: entry.source.label().to_string(),
        icon: resolve_icon_for(&entry),
    })
}

/// 取一条应用的图标。
///
/// 顺序即优先级，每一步都是「能力」而不是「形态判断」：
/// 1. 磁盘上有真实文件 → 直接读那个文件的图标（`.lnk` 会先解出真实
///    图标源，不会带上快捷方式箭头）
/// 2. 否则交给 shell —— `shell:AppsFolder\<id>` 对真 AUMID、已知文件夹
///    路径、裸伪 AUMID、纯路径四种形态都能解析出图标
pub fn resolve_icon_for(entry: &IndexedApp) -> Option<String> {
    match entry.kind {
        TargetKind::FilePath => crate::infrastructure::app_icons::resolve_icon(&entry.target),
        TargetKind::ShellItem => {
            crate::infrastructure::app_icons::cached_shell_item_icon(&entry.target)
        }
    }
}

/// `shell:AppsFolder` 里某个条目的解析名。
pub fn shell_parsing_name(id: &str) -> String {
    format!("shell:AppsFolder\\{id}")
}

/// The file on disk behind a `shell:AppsFolder` entry, if there is one.
///
/// Known-folder entries (`{GUID}\cmd.exe`) are resolved by asking the shell
/// for the folder behind the GUID and appending the relative path. Plain
/// paths are already files. Packaged Store apps (`Package!App`) and bare
/// pseudo-AUMIDs (`Microsoft.Windows.Explorer`) have no file and yield `None`,
/// so callers fall back to shell activation.
///
/// Note: `IShellItem::GetDisplayName(SIGDN_FILESYSPATH)` does **not** expose
/// these paths for AppsFolder children, which is why we resolve them here.
#[cfg(windows)]
pub fn shell_item_file_path(id: &str) -> Option<String> {
    use windows::core::GUID;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    let id = id.trim().trim_matches('"');
    if id.is_empty() {
        return None;
    }

    let path = if let Some(rest) = id.strip_prefix('{') {
        let (guid_text, relative) = rest.split_once('}')?;
        if guid_text.len() != 36 || !guid_text.is_ascii() {
            return None;
        }
        let well_formed = guid_text
            .as_bytes()
            .iter()
            .enumerate()
            .all(|(i, b)| match i {
                8 | 13 | 18 | 23 => *b == b'-',
                _ => b.is_ascii_hexdigit(),
            });
        if !well_formed {
            return None;
        }
        let guid = GUID::from(guid_text);
        let relative = relative.trim_start_matches(['\\', '/']);
        if relative.is_empty() {
            return None;
        }
        let base = unsafe { SHGetKnownFolderPath(&guid, KF_FLAG_DEFAULT, None) }.ok()?;
        let base_text = unsafe { base.to_string() }.ok()?;
        unsafe { CoTaskMemFree(Some(base.0.cast())) };
        let joined = std::path::Path::new(&base_text).join(relative);
        joined.to_string_lossy().into_owned()
    } else if id.len() >= 3 && id.as_bytes()[1] == b':' {
        id.to_string()
    } else {
        return None;
    };

    if std::path::Path::new(&path).exists() {
        Some(path)
    } else {
        None
    }
}

#[cfg(not(windows))]
pub fn shell_item_file_path(_id: &str) -> Option<String> {
    None
}

/// 暴露给前端的轻量条目：只要展示名与主键，不含图标。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedApp {
    pub name: String,
    pub target: String,
    pub subtitle: String,
}

/// 当前索引快照的精简列表。`apps()` 只在从未扫描时才会真正扫盘。
pub fn listing() -> Vec<ListedApp> {
    apps()
        .apps
        .iter()
        .map(|a| ListedApp {
            name: a.name.clone(),
            target: a.target.clone(),
            subtitle: a.source.label().to_string(),
        })
        .collect()
}

pub async fn list_async() -> Vec<ListedApp> {
    tauri::async_runtime::spawn_blocking(listing)
        .await
        .unwrap_or_default()
}

pub async fn refresh_if_stale_async() -> bool {
    tauri::async_runtime::spawn_blocking(|| refresh_if_stale(INDEX_TTL))
        .await
        .unwrap_or(false)
}

pub async fn refresh_async() -> Vec<IndexedApp> {
    tauri::async_runtime::spawn_blocking(refresh)
        .await
        .map(|i| i.apps.clone())
        .unwrap_or_default()
}

pub async fn resolve_async(target: String) -> Option<ResolvedApp> {
    tauri::async_runtime::spawn_blocking(move || resolve(&target))
        .await
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, target: &str, source: AppSource) -> IndexedApp {
        IndexedApp::new(name, target, TargetKind::FilePath, source)
    }

    #[test]
    fn merge_prefers_start_menu_over_registry() {
        let merged = merge(vec![
            app(
                "7-Zip",
                r"C:\Program Files\7-Zip\7zFM.exe",
                AppSource::Registry,
            ),
            app(
                "7-Zip",
                r"C:\Program Files\7-Zip\7zFM.exe",
                AppSource::StartMenu,
            ),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, AppSource::StartMenu);
    }

    #[test]
    fn merge_dedups_same_target_across_sources() {
        let merged = merge(vec![
            app("A", r"C:\p\app.exe", AppSource::AppsFolder),
            app("B", "c:/p/app.exe", AppSource::Registry),
        ]);
        assert_eq!(merged.len(), 1, "same target regardless of slash/case");
        assert_eq!(merged[0].name, "A");
    }

    #[test]
    fn merge_dedups_same_name() {
        let merged = merge(vec![
            app("记事本", "mspaint.exe", AppSource::Registry),
            app("记事本", "notepad.exe", AppSource::StartMenu),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].target, "notepad.exe");
    }

    #[test]
    fn merge_drops_empty_entries() {
        let merged = merge(vec![
            app("  ", "x.exe", AppSource::StartMenu),
            app("ok", "   ", AppSource::StartMenu),
            app("keep", r"C:\a\b.exe", AppSource::StartMenu),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "keep");
    }

    #[test]
    fn keeps_real_uninstall_entries() {
        // ReleaseType 0/1 are both primary installs (1.2.3 exists in practice),
        // so neither may be filtered out.
        for rt in [None, Some(0), Some(1)] {
            assert!(
                keep_uninstall_entry(&UninstallEntry {
                    display_name: "7-Zip 26.02 (x64)".into(),
                    system_component: false,
                    has_parent: false,
                    release_type: rt,
                    is_update: false,
                }),
                "ReleaseType {rt:?} should be kept"
            );
        }
    }

    #[test]
    fn drops_release_type_updates() {
        for rt in [Some(2), Some(3)] {
            assert!(
                !keep_uninstall_entry(&UninstallEntry {
                    display_name: "Visual C++ Redistributable".into(),
                    system_component: false,
                    has_parent: false,
                    release_type: rt,
                    is_update: false,
                }),
                "ReleaseType {rt:?} should be dropped"
            );
        }
    }

    #[test]
    fn drops_system_component() {
        assert!(!keep_uninstall_entry(&UninstallEntry {
            display_name: "Hidden Thing".into(),
            system_component: true,
            has_parent: false,
            release_type: None,
            is_update: false,
        }));
    }

    #[test]
    fn drops_child_entries() {
        assert!(!keep_uninstall_entry(&UninstallEntry {
            display_name: "Some Patch".into(),
            system_component: false,
            has_parent: true,
            release_type: None,
            is_update: false,
        }));
    }

    #[test]
    fn drops_guid_named_components() {
        assert!(!keep_uninstall_entry(&UninstallEntry {
            display_name: "{7B1C1234-0000-0000-0000-000000000000}".into(),
            system_component: false,
            has_parent: false,
            release_type: None,
            is_update: false,
        }));
    }

    #[test]
    fn drops_uninstallers() {
        for name in ["Uninstall Foo", "Foo 卸载", "卸载程序", "FooUninstaller"] {
            assert!(
                !keep_uninstall_entry(&UninstallEntry {
                    display_name: name.into(),
                    system_component: false,
                    has_parent: false,
                    release_type: None,
                    is_update: false,
                }),
                "should drop {name}"
            );
        }
    }

    #[test]
    fn drops_blank_names() {
        assert!(!keep_uninstall_entry(&UninstallEntry {
            display_name: "   ".into(),
            system_component: false,
            has_parent: false,
            release_type: None,
            is_update: false,
        }));
    }

    #[test]
    fn registry_app_requires_existing_icon_file() {
        assert!(registry_app("Nonexistent App", Some(r"C:\no\such\app.exe"), None, 0).is_none());
    }

    #[test]
    fn registry_app_accepts_real_exe_icon() {
        let exe = std::env::current_exe().unwrap();
        let got = registry_app("MyTools Test", Some(&exe.to_string_lossy()), None, 0);
        assert!(
            got.is_some(),
            "should accept the running exe as icon source"
        );
        assert_eq!(got.unwrap().source, AppSource::Registry);
    }

    #[test]
    fn registry_app_splits_icon_index() {
        let exe = std::env::current_exe().unwrap();
        let spec = format!("{},3", exe.to_string_lossy());
        let got = registry_app("MyTools Test2", Some(&spec), None, 0);
        assert!(got.is_some());
    }

    #[test]
    fn source_rank_orders_start_menu_first() {
        assert!(AppSource::StartMenu.rank() < AppSource::Desktop.rank());
        assert!(AppSource::Desktop.rank() < AppSource::AppsFolder.rank());
        assert!(AppSource::AppsFolder.rank() < AppSource::Registry.rank());
    }

    #[test]
    fn index_is_scanned_only_once() {
        // The index is process-wide; a second request must hand back the same
        // snapshot instead of re-scanning.
        let _ = get_or_scan();
        let a = get_or_scan();
        let b = get_or_scan();
        assert!(
            Arc::ptr_eq(&a, &b),
            "repeated calls must reuse the snapshot"
        );
    }

    #[test]
    fn normalize_name_strips_installer_suffixes() {
        assert_eq!(normalize_name("Foo (User)"), "foo");
        assert_eq!(normalize_name("Foo (x64)"), "foo");
        assert_eq!(normalize_name("7-Zip 26.02"), "7-zip");
        assert_eq!(normalize_name("Foo v1.2"), "foo");
    }

    #[test]
    fn normalize_name_keeps_years_distinct() {
        // `2022` has no dot, so it must not be treated as a version tail —
        // otherwise Visual Studio 2019 and 2022 collapse into one entry.
        assert_ne!(
            normalize_name("Visual Studio 2022"),
            normalize_name("Visual Studio 2019")
        );
        assert_eq!(normalize_name("Visual Studio 2022"), "visual studio 2022");
    }

    #[test]
    fn is_stale_handles_missing_and_fresh_snapshots() {
        let now = Instant::now();
        assert!(is_stale(None, Duration::from_secs(300), now));
        assert!(!is_stale(Some(now), Duration::from_secs(300), now));
        let old = now - Duration::from_secs(301);
        assert!(is_stale(Some(old), Duration::from_secs(300), now));
    }

    #[test]
    fn refresh_if_stale_skips_a_fresh_index() {
        let _ = get_or_scan();
        assert!(
            !refresh_if_stale(Duration::from_secs(3600)),
            "a just-scanned index must not be rescanned"
        );
    }

    #[test]
    fn listing_matches_the_snapshot() {
        let listed = listing();
        assert!(!listed.is_empty(), "listing must expose the scanned apps");
        assert_eq!(listed.len(), apps().apps.len());
        assert!(listed
            .iter()
            .all(|a| !a.name.trim().is_empty() && !a.target.trim().is_empty()));
    }

    #[test]
    fn app_resolve_returns_display_name_for_a_known_target() {
        let target = apps().apps[0].target.clone();
        let resolved = resolve(&target).expect("known target should resolve");
        assert!(!resolved.name.trim().is_empty());
        assert!(!resolved.subtitle.trim().is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn index_covers_every_source_and_scans_quickly() {
        let start = std::time::Instant::now();
        let idx = AppIndex::collect();
        let elapsed = start.elapsed();

        let by_source = |s: AppSource| idx.apps.iter().filter(|a| a.source == s).count();
        let start_menu = by_source(AppSource::StartMenu);
        let appsfolder = by_source(AppSource::AppsFolder);
        let registry = by_source(AppSource::Registry);

        eprintln!(
            "scan took {:?}: start_menu={start_menu} appsfolder={appsfolder} registry={registry} total={}",
            elapsed,
            idx.apps.len()
        );
        assert!(start_menu > 0, "Start Menu should contribute entries");
        assert!(appsfolder > 0, "shell:AppsFolder should contribute entries");
        assert!(registry > 0, "Uninstall registry should contribute entries");
        assert!(
            elapsed.as_secs() < 5,
            "startup sweep took {elapsed:?}; it must stay well under the debounce"
        );
    }

    #[cfg(windows)]
    #[test]
    fn real_index_contains_windows_builtin_apps() {
        // Notepad / Calculator ship no .lnk and register nothing in Uninstall;
        // only the AppsFolder sweep can surface them.
        let idx = get_or_scan();
        let has = |needle: &str| {
            idx.apps.iter().any(|a| {
                a.name.contains(needle)
                    || (a.kind == TargetKind::ShellItem
                        && a.target.to_lowercase().contains(&needle.to_lowercase()))
            })
        };
        assert!(has("记事本") || has("notepad"), "记事本 should be indexed");
        assert!(
            has("计算器") || has("calculator"),
            "计算器 should be indexed"
        );
    }

    #[cfg(windows)]
    #[test]
    fn real_index_contains_start_menu_apps() {
        let idx = get_or_scan();
        assert!(idx
            .apps
            .iter()
            .any(|a| a.name.contains("Visual Studio Code")));
    }

    #[cfg(windows)]
    #[test]
    fn system32_known_folder_resolves_to_cmd() {
        // `{1AC14E77-...}` is the System32 known folder — stable on every
        // Windows install, so this pins down the resolution mechanism itself
        // instead of relying on whatever AppsFolder happens to enumerate.
        let id = "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\cmd.exe";
        let path = shell_item_file_path(id).expect("System32 known folder should resolve");
        assert!(
            path.to_ascii_lowercase().ends_with("cmd.exe"),
            "unexpected path: {path}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn known_folder_entries_resolve_to_real_files() {
        // `{GUID}\cmd.exe` entries are file-backed; `shell_item_file_path`
        // must hand back the real path so launching can skip activation.
        // Not every `{GUID}` entry is file-backed (some are virtual folders),
        // so accept the first one that resolves.
        let candidates: Vec<IndexedApp> = apps()
            .apps
            .iter()
            .filter(|a| a.kind == TargetKind::ShellItem && a.target.starts_with('{'))
            .cloned()
            .collect();
        if candidates.is_empty() {
            return;
        }
        let resolved = candidates
            .iter()
            .find_map(|a| shell_item_file_path(&a.target).map(|p| (a.target.clone(), p)));
        let Some((target, path)) = resolved else {
            return;
        };
        assert!(
            std::path::Path::new(&path).exists(),
            "resolved path for {target} should exist: {path}"
        );
    }
}
