//! tesseras-storage: SQLite index, blob filesystem, import/export.

pub mod blob;
pub mod error;
pub mod identity;
pub mod sqlite;

pub use blob::FsBlobStore;
pub use error::StorageError;
pub use identity::FsIdentityStore;
pub use sqlite::{SqliteMemoryRepository, SqliteTesseraRepository};

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn migrations_run_clean() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        // Verify tables exist
        let result = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='tesseras'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(result.is_some());
        let result =
            sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='memories'")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(result.is_some());
    }
}
