//! Windows Start Menu scanner — walks known Start Menu directories
//! and yields the shortcut entries found there.
//!
//! Results are cached in-process for [`SCAN_CACHE_TTL`] so that
//! per-keystroke searches reuse the previous walk instead of hitting
//! the disk again; newly installed apps show up after the TTL expires
//! or after [`clear_scan_cache`] is called.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

const SCAN_CACHE_TTL: Duration = Duration::from_secs(30);

struct ScanCache {
    at: Instant,
    entries: Vec<StartMenuEntry>,
}

static SCAN_GENERATION: AtomicU64 = AtomicU64::new(0);

fn scan_cache() -> &'static Mutex<Option<ScanCache>> {
    static CACHE: OnceLock<Mutex<Option<ScanCache>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn lock_cache() -> MutexGuard<'static, Option<ScanCache>> {
    scan_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn scan() -> Vec<StartMenuEntry> {
    {
        let guard = lock_cache();
        if let Some(cached) = guard.as_ref() {
            if cached.at.elapsed() < SCAN_CACHE_TTL {
                return cached.entries.clone();
            }
        }
    }
    let entries = scan_uncached();
    *lock_cache() = Some(ScanCache {
        at: Instant::now(),
        entries: entries.clone(),
    });
    entries
}

pub fn clear_scan_cache() {
    *lock_cache() = None;
}

#[cfg(test)]
fn scan_generation() -> u64 {
    SCAN_GENERATION.load(Ordering::SeqCst)
}

fn scan_uncached() -> Vec<StartMenuEntry> {
    SCAN_GENERATION.fetch_add(1, Ordering::SeqCst);
    let mut out = Vec::new();
    for source in [StartMenuSource::User, StartMenuSource::Common] {
        if let Some(dir) = source.dir() {
            scan_dir(&dir, source, &mut out);
        }
    }
    out
}

#[derive(Debug, Clone, Serialize)]
pub struct StartMenuEntry {
    pub name: String,
    pub path: String,
    pub source: StartMenuSource,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StartMenuSource {
    User,
    Common,
}

impl StartMenuSource {
    fn dir(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var_os("APPDATA")?;
            let programdata = std::env::var_os("PROGRAMDATA")?;
            let start = Path::new("Microsoft\\Windows\\Start Menu\\Programs");
            let dir = match self {
                StartMenuSource::User => Path::new(&appdata).join(start),
                StartMenuSource::Common => Path::new(&programdata).join(start),
            };
            Some(dir)
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }
}

fn scan_dir(dir: &Path, source: StartMenuSource, out: &mut Vec<StartMenuEntry>) {
    if !dir.exists() {
        return;
    }
    for entry in WalkDir::new(dir).max_depth(4).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !ext.eq_ignore_ascii_case("lnk") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        out.push(StartMenuEntry {
            name: stem.to_string(),
            path: path.to_string_lossy().into_owned(),
            source,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static SCAN_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn scan_returns_empty_for_missing_dirs() {
        let _guard = SCAN_TEST_LOCK.lock().unwrap();
        // On non-Windows or with cleared APPDATA, the scanner must
        // simply return an empty Vec rather than panic.
        let entries = scan();
        for e in &entries {
            assert!(e.path.ends_with(".lnk"));
        }
    }

    #[test]
    fn scan_result_is_cached_until_cleared() {
        let _guard = SCAN_TEST_LOCK.lock().unwrap();
        // Generation only advances on real disk walks, so deltas prove
        // whether the cache short-circuited the rescan.
        clear_scan_cache();
        let first = scan();
        let generation = scan_generation();
        let second = scan();
        assert_eq!(
            scan_generation(),
            generation,
            "cached scan must not re-walk"
        );
        assert_eq!(first.len(), second.len());
        clear_scan_cache();
        let third = scan();
        assert_eq!(
            scan_generation(),
            generation + 1,
            "clear must force a re-walk"
        );
        assert_eq!(third.len(), first.len());
        clear_scan_cache();
    }

    #[test]
    fn entry_serializes_as_kebab_case_source() {
        let e = StartMenuEntry {
            name: "Notepad".into(),
            path: "C:/x.lnk".into(),
            source: StartMenuSource::Common,
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"source\":\"common\""));
    }
}
