//! Background task that polls the system clipboard at ~5 Hz, pushes
//! fresh entries into the SQLite store, and emits a Tauri event so
//! the frontend can refresh its view.

use mytools_core::clipboard::ClipboardStore;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::interval;

const POLL_INTERVAL_MS: u64 = 200;

pub fn spawn(app: AppHandle, store: Arc<ClipboardStore>) {
    tauri::async_runtime::spawn(async move {
        let mut tick = interval(Duration::from_millis(POLL_INTERVAL_MS));
        let mut last_seen: Option<String> = None;
        loop {
            tick.tick().await;
            let current = match crate::services::clipboard::read_text(&app) {
                Ok(t) => t,
                Err(_) => continue,
            };
            if current.is_empty() {
                continue;
            }
            if last_seen.as_deref() == Some(current.as_str()) {
                continue;
            }
            last_seen = Some(current.clone());
            if let Err(e) = store.push(&current) {
                tracing::warn!("clipboard push failed: {}", e);
                continue;
            }
            let _ = app.emit("clipboard:changed", &current);
        }
    });
}
