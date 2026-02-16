use tesseras_core::types::NodeId;
use tesseras_core::{ContentHash, CoreError};

#[derive(Debug, thiserror::Error)]
pub enum ReplicationError {
    #[error("tessera not found: {hash}")]
    TesseraNotFound { hash: ContentHash },

    #[error("tessera exceeds maximum size: {size} > {max}")]
    TesseraTooBig { size: u64, max: u64 },

    #[error("fragment checksum mismatch: expected {expected}, got {got}")]
    ChecksumMismatch {
        expected: ContentHash,
        got: ContentHash,
    },

    #[error("insufficient storage capacity")]
    InsufficientStorage,

    #[error("no suitable peers found for replication")]
    NoPeersAvailable,

    #[error("peer storage limit exceeded for {peer}")]
    PeerLimitExceeded { peer: NodeId },

    #[error("no fragments available for tessera {tessera_hash}")]
    NoFragmentsAvailable { tessera_hash: ContentHash },

    #[error("erasure coding error: {0}")]
    ErasureCoding(String),

    #[error(transparent)]
    Core(#[from] CoreError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
