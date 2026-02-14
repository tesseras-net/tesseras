//! tesseras-storage: SQLite index, blob filesystem, import/export.

pub mod blob;
pub mod error;
pub mod identity;
pub mod sqlite;

pub use blob::FsBlobStore;
pub use error::StorageError;
pub use identity::FsIdentityStore;
pub use sqlite::{SqliteMemoryRepository, SqliteTesseraRepository};

/// Run database migrations on the given connection.
pub fn run_migrations(conn: &rusqlite::Connection) -> Result<(), StorageError> {
    conn.execute_batch(include_str!("../migrations/001_initial.sql"))
        .map_err(|e| StorageError::Database(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_run_clean() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
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
}
