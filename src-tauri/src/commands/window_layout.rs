use crate::services::settings::SettingsState;
use mytools_core::window_layout::Bounds;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;

#[derive(Default)]
pub struct LayoutState {
    inner: Mutex<HashMap<String, (String, u64)>>,
    sequence: std::sync::atomic::AtomicU64,
    normal: Mutex<HashMap<String, Bounds>>,
}

pub fn restore(window: &tauri::WebviewWindow, key: &str) -> Result<(), String> {
    let state = window.state::<LayoutState>();
    state
        .inner
        .lock()
        .map_err(|e| e.to_string())?
        .remove(window.label());
    if let Some(bounds) = window
        .state::<SettingsState>()
        .get()
        .window_bounds
        .get(key)
        .cloned()
    {
        let monitors = window.available_monitors().map_err(|e| e.to_string())?;
        let monitor = monitors
            .iter()
            .find(|monitor| {
                let p = monitor.position();
                let s = monitor.size();
                bounds.x >= p.x
                    && bounds.x < p.x + s.width as i32
                    && bounds.y >= p.y
                    && bounds.y < p.y + s.height as i32
            })
            .or(monitors.first());
        if let Some(monitor) = monitor {
            let p = monitor.position();
            let s = monitor.size();
            let fitted = mytools_core::window_layout::fit(
                &bounds,
                (p.x, p.y, s.width, s.height),
                monitor.scale_factor(),
                if key == "launcher" {
                    (480, 200)
                } else {
                    (640, 400)
                },
            );
            let _ = window.unmaximize();
            window
                .set_size(tauri::PhysicalSize::new(fitted.width, fitted.height))
                .map_err(|e| e.to_string())?;
            window
                .set_position(tauri::PhysicalPosition::new(fitted.x, fitted.y))
                .map_err(|e| e.to_string())?;
            if fitted.maximized {
                window.maximize().map_err(|e| e.to_string())?;
            }
        }
    }
    if let (Ok(position), Ok(size), Ok(scale)) = (
        window.outer_position(),
        window.inner_size(),
        window.scale_factor(),
    ) {
        let normal = window
            .state::<SettingsState>()
            .get()
            .window_bounds
            .get(key)
            .cloned()
            .unwrap_or(Bounds {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
                scale,
                maximized: false,
            });
        state
            .normal
            .lock()
            .map_err(|e| e.to_string())?
            .insert(window.label().into(), normal);
    }
    state
        .inner
        .lock()
        .map_err(|e| e.to_string())?
        .insert(window.label().into(), (key.into(), 0));
    Ok(())
}

pub fn changed(window: &tauri::Window) {
    let app = window.app_handle();
    let state = app.state::<LayoutState>();
    let Ok(mut records) = state.inner.lock() else {
        return;
    };
    let Some((key, sequence)) = records.get_mut(window.label()) else {
        return;
    };
    *sequence = state
        .sequence
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        + 1;
    let sequence = *sequence;
    let key = key.clone();
    let Ok(position) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.inner_size() else {
        return;
    };
    let Ok(scale) = window.scale_factor() else {
        return;
    };
    let maximized = window.is_maximized().unwrap_or(false);
    let mut bounds = Bounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        scale,
        maximized,
    };
    if let Ok(mut normal) = state.normal.lock() {
        if maximized {
            if let Some(previous) = normal.get(window.label()).cloned() {
                bounds = Bounds {
                    maximized: true,
                    ..previous
                };
            }
        } else {
            normal.insert(window.label().into(), bounds.clone());
        }
    }
    let label = window.label().to_string();
    let app = app.clone();
    drop(records);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let current = app
            .state::<LayoutState>()
            .inner
            .lock()
            .ok()
            .and_then(|records| records.get(&label).cloned());
        if current
            .as_ref()
            .is_some_and(|(current_key, current_sequence)| {
                current_key == &key && *current_sequence == sequence
            })
        {
            if let Err(error) = app.state::<SettingsState>().set_window_bounds(key, bounds) {
                tracing::warn!("window layout save: {error}");
            }
        }
    });
}

pub fn forget(window: &tauri::Window) {
    if let Ok(mut values) = window.state::<LayoutState>().normal.lock() {
        values.remove(window.label());
    }
    if let Ok(mut values) = window.state::<LayoutState>().inner.lock() {
        values.remove(window.label());
    }
}

pub fn suspend(app: &tauri::AppHandle, label: &str) {
    if let Ok(mut values) = app.state::<LayoutState>().inner.lock() {
        values.remove(label);
    }
}
