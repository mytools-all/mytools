pub mod app_icons;
pub mod app_identity;
pub mod clipboard_files;
pub mod clipboard_watcher;
pub mod everything;
pub mod file_drop;
pub mod launch_context;
pub mod logging;
pub mod middle_mouse;
pub mod net_ifaces;
pub mod paths;
pub mod process_resources;
#[cfg(any(windows, test))]
pub mod taskbar_identity;

pub mod autostart;
#[cfg(windows)]
pub mod webview_idle;
