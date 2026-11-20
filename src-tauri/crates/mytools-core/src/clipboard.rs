//! Clipboard history persistence.
//!
//! Text and bounded PNG image history. The store
//! is opened lazily via `tauri::State<ClipboardStore>` injected by
//! the app builder; the polling task is launched from `lib.rs`.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub const MAX_ENTRIES: i64 = 500;
pub const MAX_IMAGE_CONTENT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_IMAGE_HISTORY_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content: String,
    pub kind: String,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("剪贴板图片格式无效或超过 4 MB")]
    InvalidImage,
    #[error("置顶图片已达到历史容量上限")]
    ImageHistoryFull,
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
                 kind TEXT NOT NULL DEFAULT 'text',
                 pinned INTEGER NOT NULL DEFAULT 0,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE INDEX IF NOT EXISTS idx_clipboard_created_at
                 ON clipboard(created_at DESC);",
        )?;
        let has_kind: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('clipboard') WHERE name = 'kind')",
            [],
            |row| row.get(0),
        )?;
        if !has_kind {
            conn.execute("ALTER TABLE clipboard ADD COLUMN kind TEXT NOT NULL DEFAULT 'text'", [])?;
        }
        Ok(())
    }

    /// Insert a new entry, or bump the existing one if `content` is
    /// already at the top. Returns the row id of the latest entry.
    pub fn push(&self, content: &str) -> Result<i64, ClipboardError> {
        self.push_content(content, "text")
    }

    pub fn push_image(&self, content: &str) -> Result<i64, ClipboardError> {
        if !content.starts_with("data:image/png;base64,") || content.len() > MAX_IMAGE_CONTENT_BYTES {
            return Err(ClipboardError::InvalidImage);
        }
        self.push_content(content, "image")
    }

    fn push_content(&self, content: &str, kind: &str) -> Result<i64, ClipboardError> {
        let mut conn = self.conn.lock().expect("clipboard mutex");
        let tx = conn.transaction()?;

        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM clipboard WHERE content = ?1 AND kind = ?2 ORDER BY id DESC LIMIT 1",
                params![content, kind],
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
                "INSERT INTO clipboard (content, kind) VALUES (?1, ?2)",
                params![content, kind],
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

        if kind == "image" {
            let images = {
                let mut statement = tx.prepare(
                    "SELECT id, length(content), pinned FROM clipboard WHERE kind = 'image' ORDER BY pinned DESC, created_at DESC, id DESC",
                )?;
                let rows = statement.query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, usize>(1)?, row.get::<_, bool>(2)?))
                })?;
                rows.collect::<Result<Vec<_>, _>>()?
            };
            let mut bytes = 0;
            for (image_id, size, pinned) in images {
                if bytes + size <= MAX_IMAGE_HISTORY_BYTES {
                    bytes += size;
                } else if pinned {
                    return Err(ClipboardError::ImageHistoryFull);
                } else {
                    if image_id == id { return Err(ClipboardError::ImageHistoryFull); }
                    tx.execute("DELETE FROM clipboard WHERE id = ?1", params![image_id])?;
                }
            }
        }

        tx.commit()?;
        Ok(id)
    }

    pub fn list(&self, limit: i64) -> Result<Vec<ClipboardEntry>, ClipboardError> {
        let conn = self.conn.lock().expect("clipboard mutex");
        let mut stmt = conn.prepare(
            "SELECT id, content, pinned, created_at, kind FROM clipboard
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |r| {
                let created_at: String = r.get(3)?;
                Ok(ClipboardEntry {
                    id: r.get(0)?,
                    content: r.get(1)?,
                    kind: r.get(4)?,
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

    #[test]
    fn image_and_text_with_the_same_content_keep_distinct_types() {
        let s = store();
        let png = "data:image/png;base64,aGVsbG8=";
        s.push(png).unwrap();
        s.push_image(png).unwrap();
        s.push_image(png).unwrap();
        let rows = s.list(10).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.kind == "text"));
        assert!(rows.iter().any(|row| row.kind == "image"));
    }

    #[test]
    fn old_text_database_is_migrated_without_losing_records() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE clipboard(id INTEGER PRIMARY KEY, content TEXT NOT NULL, pinned INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now'))); INSERT INTO clipboard(content) VALUES ('old text');").unwrap();
        ClipboardStore::migrate(&conn).unwrap();
        ClipboardStore::migrate(&conn).unwrap();
        let s = ClipboardStore { conn: std::sync::Mutex::new(conn) };
        let entries = s.list(10).unwrap();
        assert_eq!(entries[0].content, "old text");
        assert_eq!(entries[0].kind, "text");
    }

    #[test]
    fn image_limits_preserve_text_and_pinned_images() {
        let s = store();
        s.push("keep this text").unwrap();
        let pinned = s.push_image("data:image/png;base64,cGlubmVk").unwrap();
        s.pin(pinned, true).unwrap();
        for index in 0..10 {
            let image = format!("data:image/png;base64,{index}{}", "A".repeat(MAX_IMAGE_CONTENT_BYTES - 32));
            s.push_image(&image).unwrap();
        }
        let rows = s.list(500).unwrap();
        let bytes: usize = rows.iter().filter(|row| row.kind == "image").map(|row| row.content.len()).sum();
        assert!(bytes <= MAX_IMAGE_HISTORY_BYTES);
        assert!(rows.iter().any(|row| row.id == pinned && row.pinned));
        assert!(rows.iter().any(|row| row.content == "keep this text"));
        assert!(s.push_image("not an image").is_err());
        assert!(s.push_image(&format!("data:image/png;base64,{}", "A".repeat(MAX_IMAGE_CONTENT_BYTES))).is_err());
    }
}
