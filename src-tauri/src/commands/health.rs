//! Tracks the WebView2 child processes spawned by the running Tauri app
//! and exposes a snapshot via `webview_health`. Surfacing the count of
//! lost webview processes to the frontend lets the launcher render a
//! visible error in `usePluginErrors("system:webview", ...)` instead of
//! failing silently when a renderer dies.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
#[allow(unused_imports)]
use sysinfo::{ProcessStatus, ProcessesToUpdate, System};

#[derive(Default)]
struct HealthState {
    known_pids: HashSet<u32>,
    crash_count: u32,
    last_crash_at_unix_ms: Option<u64>,
}

#[derive(Default)]
pub struct WebviewHealth {
    inner: Mutex<HealthState>,
    my_pid: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct WebviewHealthSnapshot {
    pub my_pid: u32,
    pub live_pids: Vec<u32>,
    pub crash_count: u32,
    #[serde(rename = "last_crash_at")]
    pub last_crash_at_unix_ms: Option<u64>,
}

#[tauri::command]
pub async fn webview_health(
    health: tauri::State<'_, std::sync::Arc<WebviewHealth>>,
) -> Result<WebviewHealthSnapshot, String> {
    let health = health.inner().clone();
    tauri::async_runtime::spawn_blocking(move || health.sample())
        .await
        .map_err(|e| e.to_string())
}

impl WebviewHealth {
    pub fn new(my_pid: u32) -> Self {
        Self {
            inner: Mutex::new(HealthState::default()),
            my_pid,
        }
    }

    /// Sample current msedgewebview2.exe processes and compare against the
    /// previously known set. New PIDs are appended; missing PIDs bump the
    /// crash counter. Returns a snapshot suitable for `webview_health`.
    pub fn sample(&self) -> WebviewHealthSnapshot {
        let mut sys = System::new();
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            sysinfo::ProcessRefreshKind::default(),
        );

        let mut live: HashSet<u32> = HashSet::new();
        for (pid, process) in sys.processes() {
            if process.name() == "msedgewebview2.exe" {
                live.insert(pid.as_u32());
            }
        }

        let mut guard = self.inner.lock().expect("health mutex poisoned");
        let new_pids: Vec<u32> = live.difference(&guard.known_pids).copied().collect();
        let lost_pids: Vec<u32> = guard.known_pids.difference(&live).copied().collect();
        let new_crashes = lost_pids.len() as u32;

        for pid in &new_pids {
            let _ = pid;
        }

        if new_crashes > 0 {
            guard.crash_count = guard.crash_count.saturating_add(new_crashes);
            guard.last_crash_at_unix_ms = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            );
        }
        guard.known_pids = live.clone();

        WebviewHealthSnapshot {
            my_pid: self.my_pid,
            live_pids: {
                let mut v: Vec<u32> = live.into_iter().collect();
                v.sort_unstable();
                v
            },
            crash_count: guard.crash_count,
            last_crash_at_unix_ms: guard.last_crash_at_unix_ms,
        }
    }
}

#[allow(dead_code)]
pub fn child_count(parent_pid: u32) -> usize {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        sysinfo::ProcessRefreshKind::default(),
    );
    let mut n = 0usize;
    for process in sys.processes().values() {
        if process.name() == "msedgewebview2.exe" {
            let ppid = process.parent().map(|p| p.as_u32()).unwrap_or(0);
            if ppid == parent_pid || process.status() == ProcessStatus::Run {
                n += 1;
            }
        }
    }
    n.max(1)
}

pub async fn sample_periodically(health: std::sync::Arc<WebviewHealth>, period: Duration) {
    let mut ticker = tokio::time::interval(period);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let snapshot = health.sample();
        if snapshot.crash_count > 0 {
            tracing::warn!(
                "webview_health: {} live webview(s), crash_count={}, last_crash_at={:?}",
                snapshot.live_pids.len(),
                snapshot.crash_count,
                snapshot.last_crash_at_unix_ms
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_state_returns_zero_crashes() {
        let h = WebviewHealth::new(std::process::id());
        let s = h.sample();
        assert_eq!(s.crash_count, 0);
        assert!(s.last_crash_at_unix_ms.is_none());
    }

    #[test]
    fn repeated_sample_is_idempotent() {
        let h = WebviewHealth::new(std::process::id());
        let s1 = h.sample();
        let s2 = h.sample();
        assert_eq!(s1.crash_count, s2.crash_count);
    }
}
