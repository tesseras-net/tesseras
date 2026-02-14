#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("signature verification failed")]
    VerificationFailed,

    #[error("invalid key material: {0}")]
    InvalidKey(String),

    #[error("signing error: {0}")]
    SigningError(String),

    #[error("erasure coding error: {0}")]
    ErasureError(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
