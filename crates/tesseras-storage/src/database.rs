use std::path::Path;

use crate::error::StorageError;

/// Configuration for SQLite pragmas and storage behavior.
///
/// `cache_size` is per-connection. Current architecture uses a single
/// `Arc<Mutex<Connection>>` per process. If this changes, revisit the default.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Use FULL synchronous mode instead of NORMAL. Enable on unstable hardware
    /// (RPi + SD card) where kernel crashes may lose un-checkpointed WAL
    /// transactions. The `reciprocity` table accumulates incremental bilateral
    /// state not recoverable from other sources.
    pub sqlite_synchronous_full: bool,
    /// SQLite page cache size in KB (per-connection). Default: 64000 (64MB).
    pub sqlite_cache_size_kb: u32,
    /// Timeout in ms before returning SQLITE_BUSY. Default: 5000.
    pub sqlite_busy_timeout_ms: u32,
    /// In-memory LRU cache size for fragment blobs in MB. Default: 128.
    pub fragment_cache_size_mb: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            sqlite_synchronous_full: false,
            sqlite_cache_size_kb: 64000,
            sqlite_busy_timeout_ms: 5000,
            fragment_cache_size_mb: 128,
        }
    }
}

/// Open a database at `path` with pragmas and migrations applied.
pub fn open_database(
    path: &Path,
    config: &StorageConfig,
) -> Result<rusqlite::Connection, StorageError> {
    let conn =
        rusqlite::Connection::open(path).map_err(|e| StorageError::Database(e.to_string()))?;
    configure_pragmas(&conn, config)?;
    crate::run_migrations(&conn)?;
    Ok(conn)
}

/// Open an in-memory database with pragmas and migrations applied.
/// WAL is a no-op for in-memory DBs (SQLite returns journal_mode=memory).
pub fn open_in_memory(config: &StorageConfig) -> Result<rusqlite::Connection, StorageError> {
    let conn = rusqlite::Connection::open_in_memory()
        .map_err(|e| StorageError::Database(e.to_string()))?;
    configure_pragmas(&conn, config)?;
    crate::run_migrations(&conn)?;
    Ok(conn)
}

fn configure_pragmas(
    conn: &rusqlite::Connection,
    config: &StorageConfig,
) -> Result<(), StorageError> {
    let synchronous = if config.sqlite_synchronous_full {
        "FULL"
    } else {
        "NORMAL"
    };
    conn.execute_batch(&format!(
        "PRAGMA journal_mode = WAL;\
         PRAGMA foreign_keys = ON;\
         PRAGMA synchronous = {synchronous};\
         PRAGMA busy_timeout = {timeout};\
         PRAGMA cache_size = -{cache_kb};\
         PRAGMA wal_autocheckpoint = 1000;",
        timeout = config.sqlite_busy_timeout_ms,
        cache_kb = config.sqlite_cache_size_kb,
    ))
    .map_err(|e| StorageError::Database(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_sets_foreign_keys() {
        let conn = open_in_memory(&StorageConfig::default()).unwrap();
        let fk: i32 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn open_in_memory_sets_busy_timeout() {
        let conn = open_in_memory(&StorageConfig::default()).unwrap();
        let timeout: u32 = conn
            .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
            .unwrap();
        assert_eq!(timeout, 5000);
    }

    #[test]
    fn open_in_memory_journal_mode_is_memory() {
        // WAL is a no-op for in-memory DBs; SQLite silently returns "memory"
        let conn = open_in_memory(&StorageConfig::default()).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "memory");
    }

    #[test]
    fn open_in_memory_runs_migrations() {
        let conn = open_in_memory(&StorageConfig::default()).unwrap();
        let exists: bool = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='tesseras'")
            .unwrap()
            .exists([])
            .unwrap();
        assert!(exists);
    }

    #[test]
    fn synchronous_full_when_configured() {
        let config = StorageConfig {
            sqlite_synchronous_full: true,
            ..StorageConfig::default()
        };
        let conn = open_in_memory(&config).unwrap();
        let sync_val: i32 = conn
            .query_row("PRAGMA synchronous", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sync_val, 2); // FULL = 2
    }

    #[test]
    fn synchronous_normal_by_default() {
        let conn = open_in_memory(&StorageConfig::default()).unwrap();
        let sync_val: i32 = conn
            .query_row("PRAGMA synchronous", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sync_val, 1); // NORMAL = 1
    }
}
