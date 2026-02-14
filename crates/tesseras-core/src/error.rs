#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid hex string: {0}")]
    InvalidHex(String),

    #[error("invalid length: expected {expected}, got {got}")]
    InvalidLength { expected: usize, got: usize },

    #[error("invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("invalid tessera: {0}")]
    InvalidTessera(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
