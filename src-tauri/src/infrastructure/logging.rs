//! Release-mode logging to a rolling file under `<data_root>/logs/`
//! (`%LOCALAPPDATA%\MyTools\data\logs`, see `commands/paths.rs`).
//!
//! Initialised once from `lib.rs::run`; tests skip the file sink via the
//! `MYTOOLS_TEST_NO_LOG` env var so they don't pollute the user's data dir.

use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

pub struct LogGuard {
    _file_guard: WorkerGuard,
}

pub fn log_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("logs")
}

#[allow(dead_code)]
pub fn log_file_path(app_data_dir: &Path) -> PathBuf {
    log_dir(app_data_dir).join("mytools.log")
}

pub fn init(app_data_dir: &Path) -> Option<LogGuard> {
    if std::env::var_os("MYTOOLS_TEST_NO_LOG").is_some() {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().with_target(false))
            .try_init();
        return None;
    }

    let dir = log_dir(app_data_dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("mytools: failed to create log dir {}: {e}", dir.display());
        return None;
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let file_appender = tracing_appender::rolling::daily(&dir, "mytools.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let result = tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false)
                .with_target(false),
        )
        .try_init();

    if result.is_err() {
        return None;
    }
    Some(LogGuard { _file_guard: guard })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_dir_under_data_root() {
        let base = PathBuf::from("C:/Users/u/AppData/Local/MyTools/data");
        let p = log_dir(&base);
        assert_eq!(p, base.join("logs"));
    }

    #[test]
    fn log_file_path_under_logs_dir() {
        let p = log_file_path(&PathBuf::from("/tmp/app"));
        assert_eq!(p, PathBuf::from("/tmp/app/logs/mytools.log"));
    }
}
