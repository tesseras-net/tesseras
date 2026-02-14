//! tesseras-embedded: embeddable node for mobile/desktop via FFI.

pub mod crypto_service;
pub mod dht_adapter;
pub mod error;
pub mod node;
pub mod types;

pub use error::TesserasError;
pub use node::EmbeddedNode;
pub use types::*;
