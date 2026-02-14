//! tesseras-core: domain types, tessera format, and serialization.

pub mod enums;
pub mod error;
pub mod manifest;
pub mod memory;
pub mod metadata;
pub mod network;
#[cfg(feature = "service")]
pub mod ports;
#[cfg(feature = "service")]
pub mod service;
pub mod tessera;
pub mod types;

pub use enums::{ApproximateDate, MemoryType, SchemaVersion, Visibility};
pub use error::CoreError;
pub use manifest::{Manifest, ManifestEntry};
pub use memory::Memory;
pub use metadata::{Location, MemoryMetadata, Person};
pub use network::*;
#[cfg(feature = "service")]
pub use ports::*;
#[cfg(feature = "service")]
pub use service::{CreateInput, FileInput, FileVerification, TesseraService, VerifyReport};
pub use tessera::{Tessera, TesseraIdentity};
pub use types::{ContentHash, NodeId};
