#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("tessera not found: {hash}")]
    TesseraNotFound { hash: String },

    #[error("memory not found: {hash}")]
    MemoryNotFound { hash: String },

    #[error("identity not found for algorithm: {0}")]
    IdentityNotFound(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),
}
