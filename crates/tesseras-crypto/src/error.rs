#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("signature verification failed")]
    VerificationFailed,

    #[error("invalid key material: {0}")]
    InvalidKey(String),

    #[error("signing error: {0}")]
    SigningError(String),

    #[error("encryption failed: {0}")]
    EncryptFailed(String),

    #[error("decryption failed: invalid key or tampered ciphertext")]
    DecryptFailed,

    #[error("key encapsulation failed: {0}")]
    KemFailed(String),

    #[error("erasure coding error: {0}")]
    ErasureError(String),

    #[error("shamir: invalid config — {0}")]
    ShamirInvalidConfig(String),

    #[error("shamir: split failed — {0}")]
    ShamirSplitFailed(String),

    #[error("shamir: reconstruction failed — {0}")]
    ShamirReconstructFailed(String),

    #[error("shamir: share validation failed — {0}")]
    ShareValidationFailed(String),

    #[error(
        "shamir: owner verification failed — reconstructed key does not match expected fingerprint"
    )]
    ShamirOwnerMismatch,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
