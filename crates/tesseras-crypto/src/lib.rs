//! tesseras-crypto: cryptographic primitives for Tesseras.

pub mod error;

#[cfg(feature = "classical")]
pub mod dual;
#[cfg(feature = "classical")]
pub mod ed25519;
#[cfg(feature = "classical")]
pub mod hasher;

#[cfg(feature = "encryption")]
pub mod encryption;
#[cfg(feature = "encryption")]
pub mod kem;
#[cfg(feature = "encryption")]
pub mod sealed;

#[cfg(feature = "erasure")]
pub mod erasure;

#[cfg(feature = "shamir")]
pub mod shamir;

pub mod secret_blob;

pub use error::CryptoError;
