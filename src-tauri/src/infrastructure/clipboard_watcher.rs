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
        let mut last_seen: Option<(bool, String)> = None;
        let mut last_sequence = None;
        loop {
            tick.tick().await;
            let sequence = clipboard_sequence();
            if sequence.is_some() && sequence == last_sequence {
                continue;
            }
            let read_app = app.clone();
            let current = match tauri::async_runtime::spawn_blocking(move || {
                crate::services::clipboard::read_history_content(&read_app)
            }).await {
                Ok(Ok(value)) => value,
                _ => continue,
            };
            let Some(current) = current else {
                last_sequence = sequence;
                last_seen = None;
                continue;
            };
            if last_seen.as_ref().is_some_and(|(image, content)| *image == current.image && content == &current.content) {
                last_sequence = sequence;
                continue;
            }
            let pushed = if current.image { store.push_image(&current.content) } else { store.push(&current.content) };
            if let Err(e) = pushed {
                tracing::warn!("clipboard push failed: {}", e);
                last_sequence = sequence;
                continue;
            }
            last_sequence = sequence;
            last_seen = Some((current.image, current.content));
            let _ = app.emit("clipboard:changed", ());
        }
    });
}

fn clipboard_sequence() -> Option<u32> {
    #[cfg(windows)]
    {
        let sequence = unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() };
        (sequence != 0).then_some(sequence)
    }
    #[cfg(not(windows))]
    { None }
}
