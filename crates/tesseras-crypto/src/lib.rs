//! tesseras-crypto: cryptographic primitives for Tesseras.

pub mod error;

#[cfg(feature = "classical")]
pub mod dual;
#[cfg(feature = "classical")]
pub mod ed25519;
#[cfg(feature = "classical")]
pub mod hasher;

#[cfg(feature = "erasure")]
pub mod erasure;

pub use error::CryptoError;
