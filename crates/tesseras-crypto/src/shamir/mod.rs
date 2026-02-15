//! Shamir's Secret Sharing over GF(256).
//!
//! Splits a secret byte slice into N shares with threshold T.
//! Any T shares reconstruct the original; T-1 shares reveal nothing
//! (information-theoretic security).

mod gf256;
