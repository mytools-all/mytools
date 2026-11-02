//! App launcher — exposes Start Menu scanning and lets the frontend
//! invoke a `.lnk` shortcut via the shell.

use mytools_core::{fuzzy_match, scan_start_menu, FuzzyHit, StartMenuEntry};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub name: String,
    pub path: String,
    pub source: String,
}

fn source_to_str(source: mytools_core::StartMenuSource) -> &'static str {
    match source {
        mytools_core::StartMenuSource::User => "user",
        mytools_core::StartMenuSource::Common => "common",
    }
}

/// How a search target must be launched.
#[derive(Debug, PartialEq, Eq)]
pub enum LaunchKind {
    /// A `shell:AppsFolder` entry — activation, file path or the shell's own
    /// "open" verb, resolved at launch time.
    ShellItem,
    /// File path *or* `scheme:` URI — ShellExecute handles both, so one
    /// branch covers `C:\app.lnk`, `C:\app.exe` and `ms-settings:display`.
    ShellExecute,
}

pub fn launch_kind(target: &str) -> LaunchKind {
    // The index knows what every target actually is; only fall back to
    // string sniffing for targets it has never seen (e.g. a pin left behind
    // by an uninstalled app).
    if let Some(entry) = crate::services::apps::index::find_by_target(target) {
        return match entry.kind {
            crate::services::apps::index::TargetKind::ShellItem => LaunchKind::ShellItem,
            crate::services::apps::index::TargetKind::FilePath => LaunchKind::ShellExecute,
        };
    }
    if crate::infrastructure::app_icons::is_aumid_target(target) {
        LaunchKind::ShellItem
    } else {
        LaunchKind::ShellExecute
    }
}

/// Open a launch target without going through Tauri's command wrapper.
/// Shared by `app_launcher_open` and the `app.launch` capability.
pub fn open_target<R: tauri::Runtime>(app: &tauri::AppHandle<R>, path: &str) -> Result<(), String> {
    match launch_kind(path) {
        LaunchKind::ShellItem => open_shell_item(path),
        LaunchKind::ShellExecute => {
            use tauri_plugin_opener::OpenerExt;
            app.opener()
                .open_path(path.to_string(), None::<&str>)
                .map_err(|e| e.to_string())
        }
    }
}

/// Open a `shell:AppsFolder` entry.
///
/// The shell understands every identifier shape it hands out, so we ask it
/// what the entry really is instead of classifying the string ourselves:
/// packaged apps go through the activation manager, file-backed entries run
/// the file, and anything else is opened through Explorer's shell namespace
/// (the same route a Start Menu click takes).
#[cfg(windows)]
fn open_shell_item(id: &str) -> Result<(), String> {
    let mut activation_error = None;
    if crate::infrastructure::app_icons::is_aumid_target(id) {
        match activate_aumid(id) {
            Ok(()) => return Ok(()),
            Err(e) => activation_error = Some(e),
        }
    }
    if let Some(path) = crate::services::apps::index::shell_item_file_path(id) {
        return shell_execute(&path);
    }
    let parsing = crate::services::apps::index::shell_parsing_name(id);
    shell_execute_with_arg("explorer.exe", &parsing).map_err(|e| match activation_error {
        Some(activation) => format!("{e}（激活失败：{activation}）"),
        None => e,
    })
}

/// Run a target through the shell.
#[cfg(windows)]
fn shell_execute(target: &str) -> Result<(), String> {
    shell_execute_with_args(target, None)
}

/// Run `exe` with `arg`, or a bare target when `arg` is `None`.
#[cfg(windows)]
fn shell_execute_with_args(target: &str, arg: Option<&str>) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let wide_target: Vec<u16> = target.encode_utf16().chain(Some(0)).collect();
    let wide_arg: Vec<u16> = arg
        .map(|a| a.encode_utf16().chain(Some(0)).collect())
        .unwrap_or_default();
    let params = if arg.is_some() {
        PCWSTR(wide_arg.as_ptr())
    } else {
        PCWSTR::null()
    };
    let ok = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            PCWSTR(wide_target.as_ptr()),
            params,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if ok.0 as isize <= 32 {
        return Err(format!("无法启动: {target}"));
    }
    Ok(())
}

#[cfg(windows)]
fn shell_execute_with_arg(exe: &str, arg: &str) -> Result<(), String> {
    shell_execute_with_args(exe, Some(arg))
}

#[cfg(not(windows))]
fn shell_execute(_target: &str) -> Result<(), String> {
    Err("shell execution is only available on Windows".into())
}

#[cfg(not(windows))]
fn shell_execute_with_arg(_exe: &str, _arg: &str) -> Result<(), String> {
    Err("shell execution is only available on Windows".into())
}

#[cfg(not(windows))]
fn open_shell_item(_id: &str) -> Result<(), String> {
    Err("shell items can only be opened on Windows".into())
}

/// Store / packaged apps are launched through `IApplicationActivationManager`
/// rather than ShellExecute — there is no executable path to hand to it.
#[cfg(windows)]
fn activate_aumid(aumid: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        ApplicationActivationManager, IApplicationActivationManager, ACTIVATEOPTIONS,
    };

    const RPC_E_CHANGED_MODE: i32 = 0x8001_010Eu32 as i32;

    let mut uninit = false;
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_ok() {
        uninit = true;
    } else if hr.0 != RPC_E_CHANGED_MODE {
        return Err("COM unavailable".into());
    }

    let out = (|| {
        let mgr: IApplicationActivationManager =
            unsafe { CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_INPROC_SERVER) }
                .map_err(|e| e.to_string())?;
        // ActivateApplication wants a NUL-terminated wide string; it returns
        // the launched process id, which we discard.
        let wide: Vec<u16> = aumid.encode_utf16().chain(Some(0)).collect();
        unsafe { mgr.ActivateApplication(PCWSTR(wide.as_ptr()), None, ACTIVATEOPTIONS(0)) }
            .map(|_| ())
            .map_err(|e| e.to_string())
    })();

    if uninit {
        unsafe { CoUninitialize() };
    }
    out
}

#[cfg(not(windows))]
fn activate_aumid(_aumid: &str) -> Result<(), String> {
    Err("Store apps can only be launched on Windows".into())
}

fn scan_entries() -> Vec<AppEntry> {
    scan_start_menu()
        .into_iter()
        .map(|e: StartMenuEntry| AppEntry {
            name: e.name,
            path: e.path,
            source: source_to_str(e.source).to_string(),
        })
        .collect()
}

fn search_entries(query: String) -> Vec<AppEntry> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let entries = scan_start_menu();
    let candidates: Vec<(String, String)> = entries
        .iter()
        .map(|e| (e.name.clone(), e.name.clone()))
        .collect();
    let ranked: Vec<FuzzyHit> = fuzzy_match(&query, &candidates);
    ranked
        .into_iter()
        .filter_map(|hit| {
            entries
                .iter()
                .find(|e| e.name == hit.title)
                .map(|e| AppEntry {
                    name: e.name.clone(),
                    path: e.path.clone(),
                    source: source_to_str(e.source).to_string(),
                })
        })
        .collect()
}

pub async fn scan() -> Vec<AppEntry> {
    tauri::async_runtime::spawn_blocking(scan_entries)
        .await
        .unwrap_or_default()
}

pub async fn search(query: String) -> Vec<AppEntry> {
    tauri::async_runtime::spawn_blocking(move || search_entries(query))
        .await
        .unwrap_or_default()
}

pub async fn launch(app: tauri::AppHandle, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open_target(&app, &path))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_apps_dispatch_to_shell_items() {
        // Store apps are identified by AUMID, but the index reports them as
        // shell items; how to actually start one is resolved at launch time.
        assert_eq!(
            launch_kind("Microsoft.WindowsNotepad_8wekyb3d8bbwe!App"),
            LaunchKind::ShellItem
        );
        assert_eq!(
            launch_kind("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
            LaunchKind::ShellItem
        );
    }

    #[test]
    fn paths_and_uris_dispatch_to_shellexecute() {
        for target in [
            r"C:\Users\x\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Notepad.lnk",
            "C:/x.lnk",
            r"C:\Windows\System32\notepad.exe",
            "ms-settings:display",
            "ms-settings:",
            "explorer",
        ] {
            assert_eq!(
                launch_kind(target),
                LaunchKind::ShellExecute,
                "{target} should go through ShellExecute"
            );
        }
    }

    #[test]
    fn aumid_detection_rejects_paths_and_uris() {
        use crate::infrastructure::app_icons::is_aumid_target;
        // AUMIDs are `PackageFamily!AppId`; anything with a path separator or
        // a scheme is something else entirely.
        assert!(is_aumid_target("Microsoft.Paint_8wekyb3d8bbwe!App"));
        assert!(!is_aumid_target(r"C:\x\y.exe"));
        assert!(!is_aumid_target("C:/x/y.exe"));
        assert!(!is_aumid_target("ms-settings:display"));
        assert!(!is_aumid_target("shell:AppsFolder\\Foo!Bar"));
        assert!(!is_aumid_target(""));
    }
}
