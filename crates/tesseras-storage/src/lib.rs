//! tesseras-storage: SQLite index, blob filesystem, import/export.

pub mod blob;
pub mod cache;
pub mod database;
pub mod error;
pub mod fragment;
pub mod identity;
pub mod metrics;
pub mod reciprocity;
pub mod sqlite;

pub use blob::FsBlobStore;
pub use cache::CachedFragmentStore;
pub use database::{open_database, open_in_memory, StorageConfig};
pub use error::StorageError;
pub use fragment::FsFragmentStore;
pub use metrics::StorageMetrics;
pub use identity::FsIdentityStore;
pub use reciprocity::SqliteReciprocityLedger;
pub use sqlite::{SqliteMemoryRepository, SqliteTesseraRepository};

/// Run database migrations on the given connection.
pub fn run_migrations(conn: &rusqlite::Connection) -> Result<(), StorageError> {
    conn.execute_batch(include_str!("../migrations/001_initial.sql"))
        .map_err(|e| StorageError::Database(e.to_string()))?;
    conn.execute_batch(include_str!("../migrations/002_replication.sql"))
        .map_err(|e| StorageError::Database(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn migrations_run_clean() {
        let conn = crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
        // Verify tables exist
        let tesseras_exists: bool = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='tesseras'")
            .unwrap()
            .exists([])
            .unwrap();
        assert!(tesseras_exists);
        let memories_exists: bool = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='memories'")
            .unwrap()
            .exists([])
            .unwrap();
        assert!(memories_exists);
    }

    #[test]
    fn replication_tables_exist() {
        let conn = crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
        for table in [
            "fragments",
            "fragment_plans",
            "holders",
            "holder_fragments",
            "reciprocity",
        ] {
            let exists: bool = conn
                .prepare(&format!(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"
                ))
                .unwrap()
                .exists([])
                .unwrap();
            assert!(exists, "table {table} should exist");
        }
    }

    #[test]
    fn reciprocity_generated_column() {
        let conn = crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
        conn.execute(
            "INSERT INTO reciprocity (peer_id, bytes_stored_for_them, bytes_they_store_for_us, last_updated)
             VALUES ('peer1', 500, 300, '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let balance: i64 = conn
            .query_row(
                "SELECT balance FROM reciprocity WHERE peer_id = 'peer1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(balance, -200); // 300 - 500 = -200
    }
}
