use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Config error: {0}")]
    Config(#[from] crate::config::ConfigError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct SyncState {
    pub file_path: String,
    pub content_hash: String,
    pub last_synced_at: Option<i64>,
    pub last_modified_at: i64,
    pub workflow_id: Option<String>,
    pub status: SyncStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Pending,
    Syncing,
    Complete,
    Error,
}

impl SyncStatus {
    fn as_str(&self) -> &'static str {
        match self {
            SyncStatus::Pending => "pending",
            SyncStatus::Syncing => "syncing",
            SyncStatus::Complete => "complete",
            SyncStatus::Error => "error",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "pending" => SyncStatus::Pending,
            "syncing" => SyncStatus::Syncing,
            "complete" => SyncStatus::Complete,
            "error" => SyncStatus::Error,
            _ => SyncStatus::Pending,
        }
    }
}

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database at the default location
    pub fn open() -> Result<Self, DatabaseError> {
        let db_path = crate::config::get_database_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Self::open_at(&db_path)
    }

    /// Open or create the database at a specific path
    pub fn open_at(path: &Path) -> Result<Self, DatabaseError> {
        let conn = Connection::open(path)?;

        let db = Self { conn };
        db.initialize()?;

        tracing::debug!("Database opened at {:?}", path);
        Ok(db)
    }

    /// Initialize the database schema
    fn initialize(&self) -> SqliteResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_state (
                file_path TEXT PRIMARY KEY,
                content_hash TEXT NOT NULL,
                last_synced_at INTEGER,
                last_modified_at INTEGER NOT NULL,
                workflow_id TEXT,
                status TEXT NOT NULL DEFAULT 'pending'
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sync_state_status ON sync_state(status)",
            [],
        )?;

        Ok(())
    }

    /// Get sync state for a file
    pub fn get_sync_state(&self, file_path: &str) -> SqliteResult<Option<SyncState>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, content_hash, last_synced_at, last_modified_at, workflow_id, status
             FROM sync_state WHERE file_path = ?1",
        )?;

        let mut rows = stmt.query([file_path])?;

        if let Some(row) = rows.next()? {
            Ok(Some(SyncState {
                file_path: row.get(0)?,
                content_hash: row.get(1)?,
                last_synced_at: row.get(2)?,
                last_modified_at: row.get(3)?,
                workflow_id: row.get(4)?,
                status: SyncStatus::from_str(&row.get::<_, String>(5)?),
            }))
        } else {
            Ok(None)
        }
    }

    /// Upsert sync state for a file
    pub fn upsert_sync_state(&self, state: &SyncState) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT INTO sync_state (file_path, content_hash, last_synced_at, last_modified_at, workflow_id, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(file_path) DO UPDATE SET
                content_hash = excluded.content_hash,
                last_synced_at = excluded.last_synced_at,
                last_modified_at = excluded.last_modified_at,
                workflow_id = excluded.workflow_id,
                status = excluded.status",
            (
                &state.file_path,
                &state.content_hash,
                &state.last_synced_at,
                &state.last_modified_at,
                &state.workflow_id,
                state.status.as_str(),
            ),
        )?;

        Ok(())
    }

    /// Update just the status of a sync state
    pub fn update_status(&self, file_path: &str, status: SyncStatus) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE sync_state SET status = ?1 WHERE file_path = ?2",
            (status.as_str(), file_path),
        )?;

        Ok(())
    }

    /// Update status and workflow_id after starting sync
    pub fn mark_syncing(&self, file_path: &str) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE sync_state SET status = 'syncing' WHERE file_path = ?1",
            [file_path],
        )?;

        Ok(())
    }

    /// Update status and workflow_id after sync completes
    pub fn mark_complete(&self, file_path: &str, workflow_id: &str) -> SqliteResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn.execute(
            "UPDATE sync_state SET status = 'complete', workflow_id = ?1, last_synced_at = ?2 WHERE file_path = ?3",
            (workflow_id, now, file_path),
        )?;

        Ok(())
    }

    /// Get all pending sync states
    pub fn get_pending(&self) -> SqliteResult<Vec<SyncState>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, content_hash, last_synced_at, last_modified_at, workflow_id, status
             FROM sync_state WHERE status = 'pending' ORDER BY last_modified_at ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SyncState {
                file_path: row.get(0)?,
                content_hash: row.get(1)?,
                last_synced_at: row.get(2)?,
                last_modified_at: row.get(3)?,
                workflow_id: row.get(4)?,
                status: SyncStatus::from_str(&row.get::<_, String>(5)?),
            })
        })?;

        rows.collect()
    }

    /// Get count of items by status
    pub fn get_status_counts(&self) -> SqliteResult<StatusCounts> {
        let mut stmt = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM sync_state GROUP BY status")?;

        let mut counts = StatusCounts::default();
        let rows = stmt.query_map([], |row| {
            let status: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((status, count))
        })?;

        for row in rows {
            let (status, count) = row?;
            match status.as_str() {
                "pending" => counts.pending = count as usize,
                "syncing" => counts.syncing = count as usize,
                "complete" => counts.complete = count as usize,
                "error" => counts.error = count as usize,
                _ => {}
            }
        }

        Ok(counts)
    }
}

#[derive(Debug, Default)]
pub struct StatusCounts {
    pub pending: usize,
    pub syncing: usize,
    pub complete: usize,
    pub error: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let db = Database::open_at(&db_path).unwrap();

        // Test insert
        let state = SyncState {
            file_path: "/test/file.jsonl".to_string(),
            content_hash: "abc123".to_string(),
            last_synced_at: None,
            last_modified_at: 1234567890,
            workflow_id: None,
            status: SyncStatus::Pending,
        };

        db.upsert_sync_state(&state).unwrap();

        // Test get
        let retrieved = db.get_sync_state("/test/file.jsonl").unwrap().unwrap();
        assert_eq!(retrieved.content_hash, "abc123");
        assert_eq!(retrieved.status, SyncStatus::Pending);

        // Test update status
        db.mark_complete("/test/file.jsonl", "workflow-123")
            .unwrap();
        let updated = db.get_sync_state("/test/file.jsonl").unwrap().unwrap();
        assert_eq!(updated.status, SyncStatus::Complete);
        assert_eq!(updated.workflow_id, Some("workflow-123".to_string()));
    }
}
