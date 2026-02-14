//! tesseras-crypto: cryptographic primitives for Tesseras.

pub mod error;

#[cfg(feature = "classical")]
pub mod hasher;

pub use error::CryptoError;
