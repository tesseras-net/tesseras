//! Error types for the embedded node FFI boundary.

#[derive(Debug, thiserror::Error)]
pub enum TesserasError {
    #[error("node not initialized")]
    NotInitialized,

    #[error("node already running")]
    AlreadyRunning,

    #[error("identity not found")]
    IdentityNotFound,

    #[error("identity already exists")]
    IdentityAlreadyExists,

    #[error("storage error: {0}")]
    Storage(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl From<tesseras_core::CoreError> for TesserasError {
    fn from(e: tesseras_core::CoreError) -> Self {
        TesserasError::Storage(e.to_string())
    }
}

impl From<tesseras_replication::ReplicationError> for TesserasError {
    fn from(e: tesseras_replication::ReplicationError) -> Self {
        TesserasError::Network(e.to_string())
    }
}

impl From<std::io::Error> for TesserasError {
    fn from(e: std::io::Error) -> Self {
        TesserasError::Storage(e.to_string())
    }
}
