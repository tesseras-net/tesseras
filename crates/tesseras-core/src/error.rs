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

    #[error("invalid hash prefix: {0}")]
    InvalidHashPrefix(String),

    #[error("ambiguous hash prefix '{prefix}': matches {count} tesseras")]
    AmbiguousPrefix { prefix: String, count: usize },

    #[error("no tessera found matching prefix: {0}")]
    PrefixNotFound(String),

    #[error("tessera too big: {size} bytes exceeds maximum {max} bytes")]
    TesseraTooBig { size: u64, max: u64 },

    #[error("database error: {0}")]
    Database(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
