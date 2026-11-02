//! Sqlite connection management.
//!
//! The actual schema (clipboard history, app index, plugin index) is
//! defined in Day 6. For Day 1 we expose an `open` helper that
//! creates an empty database at the given path, useful both in tests
//! (via `tempfile`) and at runtime (under `$APPDATA/MyTools/`).

use std::path::Path;

pub fn open(path: &Path) -> rusqlite::Result<rusqlite::Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    rusqlite::Connection::open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_memory_and_runs_pragma() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();
    }

    #[test]
    fn opens_tempfile_and_creates_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b/c.sqlite");
        let conn = open(&nested).unwrap();
        conn.execute_batch("CREATE TABLE t(x INTEGER);").unwrap();
        assert!(nested.exists());
    }
}
