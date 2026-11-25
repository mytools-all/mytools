use crate::services::settings::SettingsState;
use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

pub fn initialize(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let handle = app.clone();
    crate::infrastructure::middle_mouse::install(Box::new(move |x, y| {
        let foreground = crate::infrastructure::launch_context::foreground();
        let task_app = handle.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let payload = crate::infrastructure::launch_context::capture(&task_app, foreground);
            task_app
                .state::<crate::runtime::LaunchContexts>()
                .capture(payload);
            if let Err(error) = show(&task_app, Some((x, y))) {
                tracing::warn!("panel: {error}");
            }
        });
    }))?;
    crate::infrastructure::middle_mouse::set_enabled(enabled);
    Ok(())
}

fn show(app: &tauri::AppHandle, position: Option<(i32, i32)>) -> Result<(), String> {
    let window = if let Some(window) = app.get_webview_window("panel") {
        window
    } else {
        WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
            .disable_drag_drop_handler()
            .title("我的面板")
            .inner_size(460.0, 420.0)
            .decorations(false)
            .resizable(false)
            .transparent(true)
            .shadow(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .visible(false)
            .build()
            .map_err(|e| e.to_string())?
    };
    if let Some((x, y)) = position {
        if let Some(monitor) = app
            .monitor_from_point(f64::from(x), f64::from(y))
            .map_err(|e| e.to_string())?
        {
            let origin = monitor.position();
            let area = monitor.size();
            let size = window.outer_size().map_err(|e| e.to_string())?;
            let left = x.clamp(
                origin.x,
                (origin.x + area.width as i32 - size.width as i32).max(origin.x),
            );
            let top = y.clamp(
                origin.y,
                (origin.y + area.height as i32 - size.height as i32).max(origin.y),
            );
            window
                .set_position(PhysicalPosition::new(left, top))
                .map_err(|e| e.to_string())?;
        }
    }
    let _ = window.emit("panel:shown", ());
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn panel_show(app: tauri::AppHandle) -> Result<(), String> {
    let foreground = crate::infrastructure::launch_context::foreground();
    tauri::async_runtime::spawn_blocking(move || {
        let payload = crate::infrastructure::launch_context::capture(&app, foreground);
        app.state::<crate::runtime::LaunchContexts>()
            .capture(payload);
        show(&app, None)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn panel_middle_enabled(settings: tauri::State<'_, SettingsState>) -> bool {
    settings.get().middle_panel_enabled
}

#[derive(serde::Deserialize)]
pub struct MiddlePanelOptions {
    pub enabled: bool,
}

#[tauri::command]
pub fn panel_set_middle_enabled(
    settings: tauri::State<'_, SettingsState>,
    options: MiddlePanelOptions,
) -> Result<(), String> {
    settings.set_middle_panel_enabled(options.enabled)?;
    crate::infrastructure::middle_mouse::set_enabled(options.enabled);
    Ok(())
}

static CAPTURING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[tauri::command]
pub async fn panel_capture_image(
    window: tauri::Window,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    use std::sync::atomic::Ordering;
    if window.label() != "panel" {
        return Err("截图需从面板发起".into());
    }
    if CAPTURING.swap(true, Ordering::SeqCst) {
        return Err("已有截图任务正在进行".into());
    }
    struct CaptureGuard;
    impl Drop for CaptureGuard {
        fn drop(&mut self) {
            CAPTURING.store(false, Ordering::SeqCst);
        }
    }
    let guard = CaptureGuard;
    window.hide().map_err(|error| error.to_string())?;
    let task_app = app.clone();
    let result=tauri::async_runtime::spawn_blocking(move || {
        let _guard=guard;
        task_app.state::<crate::runtime::LaunchContexts>().capture(serde_json::Value::Null);
        std::thread::sleep(std::time::Duration::from_millis(200));
        match crate::services::screenshot::capture(&task_app)? {
            Some(data_url)=>{task_app.state::<crate::runtime::LaunchContexts>().capture(serde_json::json!({"version":1,"source":"screenshot","image":{"mime":"image/png","dataUrl":data_url}}));Ok(true)},
            None=>Ok(false),
        }
    }).await.map_err(|error|error.to_string()).and_then(|value|value);
    if let Some(panel) = app.get_webview_window("panel") {
        let _ = panel.show();
        let _ = panel.set_focus();
    }
    result
}
