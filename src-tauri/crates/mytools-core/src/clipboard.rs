//! Clipboard history persistence.
//!
//! v0.1 stores text only. Images / files arrive in v0.2. The store
//! is opened lazily via `tauri::State<ClipboardStore>` injected by
//! the app builder; the polling task is launched from `lib.rs`.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub const MAX_ENTRIES: i64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content: String,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ClipboardStore {
    conn: std::sync::Mutex<Connection>,
}

impl ClipboardStore {
    pub fn open(path: &std::path::Path) -> Result<Self, ClipboardError> {
        let conn = crate::db::open(path)?;
        Self::migrate(&conn)?;
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<Self, ClipboardError> {
        let conn = Connection::open_in_memory()?;
        Self::migrate(&conn)?;
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    fn migrate(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 content TEXT NOT NULL,
                 pinned INTEGER NOT NULL DEFAULT 0,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE INDEX IF NOT EXISTS idx_clipboard_created_at
                 ON clipboard(created_at DESC);",
        )?;
        Ok(())
    }

    /// Insert a new entry, or bump the existing one if `content` is
    /// already at the top. Returns the row id of the latest entry.
    pub fn push(&self, content: &str) -> Result<i64, ClipboardError> {
        let mut conn = self.conn.lock().expect("clipboard mutex");
        let tx = conn.transaction()?;

        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM clipboard WHERE content = ?1 ORDER BY id DESC LIMIT 1",
                params![content],
                |r| r.get(0),
            )
            .optional()?;

        let id = if let Some(id) = existing {
            tx.execute(
                "UPDATE clipboard SET created_at = datetime('now'), pinned = 0 WHERE id = ?1",
                params![id],
            )?;
            id
        } else {
            tx.execute(
                "INSERT INTO clipboard (content) VALUES (?1)",
                params![content],
            )?;
            tx.last_insert_rowid()
        };

        // Trim to MAX_ENTRIES, keeping pinned + most-recent unpinned.
        // Two-step: compute kept ids first, then delete anything else.
        tx.execute(
            "CREATE TEMP TABLE IF NOT EXISTS _keep_ids (id INTEGER PRIMARY KEY)",
            [],
        )?;
        tx.execute(
            "INSERT INTO _keep_ids (id)
             SELECT id FROM clipboard WHERE pinned = 1",
            [],
        )?;
        tx.execute(
            "INSERT INTO _keep_ids (id)
             SELECT id FROM clipboard
             WHERE pinned = 0
             ORDER BY created_at DESC
             LIMIT ?1",
            params![MAX_ENTRIES],
        )?;
        tx.execute(
            "DELETE FROM clipboard
             WHERE id NOT IN (SELECT id FROM _keep_ids)",
            [],
        )?;
        tx.execute("DROP TABLE _keep_ids", [])?;

        tx.commit()?;
        Ok(id)
    }

    pub fn list(&self, limit: i64) -> Result<Vec<ClipboardEntry>, ClipboardError> {
        let conn = self.conn.lock().expect("clipboard mutex");
        let mut stmt = conn.prepare(
            "SELECT id, content, pinned, created_at FROM clipboard
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |r| {
                let created_at: String = r.get(3)?;
                Ok(ClipboardEntry {
                    id: r.get(0)?,
                    content: r.get(1)?,
                    pinned: r.get::<_, i64>(2)? != 0,
                    created_at: DateTime::parse_from_rfc3339(&format!("{}Z", created_at))
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn pin(&self, id: i64, pinned: bool) -> Result<(), ClipboardError> {
        let conn = self.conn.lock().expect("clipboard mutex");
        conn.execute(
            "UPDATE clipboard SET pinned = ?1 WHERE id = ?2",
            params![pinned as i64, id],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<(), ClipboardError> {
        let conn = self.conn.lock().expect("clipboard mutex");
        conn.execute("DELETE FROM clipboard WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear(&self) -> Result<(), ClipboardError> {
        let conn = self.conn.lock().expect("clipboard mutex");
        conn.execute("DELETE FROM clipboard WHERE pinned = 0", [])?;
        Ok(())
    }

    pub fn latest(&self) -> Result<Option<String>, ClipboardError> {
        let conn = self.conn.lock().expect("clipboard mutex");
        let latest = conn
            .query_row(
                "SELECT content FROM clipboard ORDER BY created_at DESC LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        Ok(latest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> ClipboardStore {
        ClipboardStore::open_in_memory().unwrap()
    }

    #[test]
    fn push_creates_entry() {
        let s = store();
        let id = s.push("hello").unwrap();
        assert!(id > 0);
        let list = s.list(10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].content, "hello");
    }

    #[test]
    fn push_dedup_moves_to_top() {
        let s = store();
        s.push("a").unwrap();
        s.push("b").unwrap();
        s.push("a").unwrap();
        let list = s.list(10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].content, "a");
        assert_eq!(list[1].content, "b");
    }

    #[test]
    fn pin_protects_from_clear() {
        let s = store();
        let id = s.push("keeper").unwrap();
        s.push("transient").unwrap();
        s.pin(id, true).unwrap();
        s.clear().unwrap();
        let list = s.list(10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].content, "keeper");
    }

    #[test]
    fn delete_removes_entry() {
        let s = store();
        let id = s.push("x").unwrap();
        s.delete(id).unwrap();
        assert!(s.list(10).unwrap().is_empty());
    }

    #[test]
    fn trim_keeps_recent_unpinned() {
        let s = store();
        for i in 0..(MAX_ENTRIES + 50) {
            s.push(&format!("item-{}", i)).unwrap();
        }
        let list = s.list(MAX_ENTRIES + 10).unwrap();
        assert!(list.len() as i64 <= MAX_ENTRIES);
    }

    #[test]
    fn latest_returns_most_recent() {
        let s = store();
        s.push("first").unwrap();
        s.push("second").unwrap();
        s.push("first").unwrap();
        assert_eq!(s.latest().unwrap().as_deref(), Some("first"));
    }
}
