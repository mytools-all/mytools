use tauri::Manager;

pub fn resume(window: &tauri::Window) {
    if !matches!(window.label(), "main" | "panel") { return; }
    let Some(webview) = window.app_handle().get_webview_window(window.label()) else { return; };
    let _ = webview.with_webview(|webview| {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_3;
        use windows_core_webview::Interface;
        unsafe {
            let controller = webview.controller();
            if let Err(error) = controller.SetIsVisible(true) { tracing::debug!("host webview visibility: {error}"); }
            if let Ok(view) = controller.CoreWebView2().and_then(|view| view.cast::<ICoreWebView2_3>()) {
                if let Err(error) = view.Resume() { tracing::debug!("host webview resume: {error}"); }
            }
        }
    });
}

pub fn start(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut hidden = std::collections::HashMap::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            for label in ["main", "panel"] {
                let Some(window) = app.get_webview_window(label) else {
                    hidden.remove(label);
                    continue;
                };
                let active = app
                    .state::<crate::runtime::PluginSessions>()
                    .active_windows()
                    .iter()
                    .any(|(window, _)| window == label);
                if window.is_visible().unwrap_or(true) || active {
                    hidden.remove(label);
                    continue;
                }
                let since = hidden.entry(label).or_insert_with(std::time::Instant::now);
                if since.elapsed() < std::time::Duration::from_secs(30) {
                    continue;
                }
                let app = app.clone();
                let _ = window.with_webview(move |webview| {
                    use webview2_com::{
                        Microsoft::Web::WebView2::Win32::ICoreWebView2_3,
                        TrySuspendCompletedHandler,
                    };
                    use windows_core_webview::Interface;
                    let Some(window) = app.get_webview_window(label) else { return; };
                    if window.is_visible().unwrap_or(true) { return; }
                    if app
                        .state::<crate::runtime::PluginSessions>()
                        .active_windows()
                        .iter()
                        .any(|(window, _)| window == label)
                    {
                        return;
                    }
                    let result = (|| -> windows_core_webview::Result<()> {
                        unsafe {
                            let controller = webview.controller();
                            let mut visible = Default::default();
                            controller.IsVisible(&mut visible)?;
                            if visible.as_bool() { controller.SetIsVisible(false)?; }
                            let view = controller.CoreWebView2()?.cast::<ICoreWebView2_3>()?;
                            let mut suspended = Default::default();
                            view.IsSuspended(&mut suspended)?;
                            if suspended.as_bool() {
                                return Ok(());
                            }
                            let handler = TrySuspendCompletedHandler::create(Box::new(
                                move |status, success| {
                                    if status.is_ok() && success {
                                        tracing::info!(
                                            window = label,
                                            "idle host webview suspended"
                                        );
                                    }
                                    Ok(())
                                },
                            ));
                            view.TrySuspend(&handler)
                        }
                    })();
                    if let Err(error) = result {
                        tracing::debug!(
                            window = label,
                            "idle webview suspension unavailable: {error}"
                        );
                    }
                });
            }
        }
    });
}
