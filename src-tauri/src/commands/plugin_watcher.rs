//! Background watcher that reloads the plugin registry (and notifies
//! the frontend) whenever the plugins directory changes on disk.
//! Debounced so editor save bursts collapse into one reload, and
//! retargeted when the user picks a different plugins root.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use tauri::Emitter;

use crate::services::plugins::SharedPluginRegistry;

const DEBOUNCE_MS: u64 = 400;
const POLL_MS: u64 = 300;

/// Spawn the watcher and return the channel used to retarget it.
pub fn spawn(
    app: tauri::AppHandle,
    registry: SharedPluginRegistry,
    initial_root: PathBuf,
) -> Sender<PathBuf> {
    let (root_tx, root_rx) = mpsc::channel::<PathBuf>();
    std::thread::spawn(move || run(app, registry, initial_root, root_rx));
    root_tx
}

fn run(
    app: tauri::AppHandle,
    registry: SharedPluginRegistry,
    mut root: PathBuf,
    root_rx: Receiver<PathBuf>,
) {
    let (event_tx, event_rx) = mpsc::channel::<()>();
    let mut _debouncer = watch_root(&root, &event_tx);
    tracing::info!("plugin watcher watching {}", root.display());
    loop {
        match event_rx.recv_timeout(Duration::from_millis(POLL_MS)) {
            Ok(()) => reload_and_notify(&app, &registry, &root),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if let Ok(new_root) = root_rx.try_recv() {
            if new_root != root {
                root = new_root;
                _debouncer = watch_root(&root, &event_tx);
                tracing::info!("plugin watcher retargeted to {}", root.display());
                reload_and_notify(&app, &registry, &root);
            }
        }
    }
}

fn watch_root(root: &Path, tx: &Sender<()>) -> Option<Debouncer<RecommendedWatcher>> {
    let tx = tx.clone();
    let mut debouncer = match new_debouncer(
        Duration::from_millis(DEBOUNCE_MS),
        move |res: DebounceEventResult| {
            if let Ok(events) = res {
                if !events.is_empty() {
                    let _ = tx.send(());
                }
            }
        },
    ) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("plugin watcher init failed: {e}");
            return None;
        }
    };
    if let Err(e) = debouncer.watcher().watch(root, RecursiveMode::Recursive) {
        tracing::warn!("cannot watch {}: {}", root.display(), e);
        return None;
    }
    Some(debouncer)
}

fn reload_and_notify(app: &tauri::AppHandle, registry: &SharedPluginRegistry, root: &Path) {
    match registry.reload(root) {
        Ok(n) => {
            tracing::info!("plugin change detected, reloaded {n} plugin(s)");
            let _ = app.emit("plugin:changed", ());
        }
        Err(e) => tracing::warn!("plugin reload on change failed: {e}"),
    }
}
