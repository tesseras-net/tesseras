//! tesseras-embedded: embeddable node for mobile/desktop via FFI.

pub mod api;
pub mod crypto_service;
pub mod dht_adapter;
pub mod error;
mod frb_generated;
pub mod node;
pub mod reconnect;
pub mod types;

pub use error::TesserasError;
pub use node::EmbeddedNode;
pub use types::*;
