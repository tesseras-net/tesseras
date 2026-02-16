//! tesseras-core: domain types, tessera format, and serialization.

pub mod crockford;
pub mod enums;
pub mod error;
pub mod manifest;
pub mod memory;
pub mod metadata;
pub mod network;
pub mod pack;
#[cfg(feature = "service")]
pub mod ports;
pub mod replication;
pub mod search;
#[cfg(feature = "service")]
pub mod service;
pub mod tessera;
pub mod types;

pub use enums::{ApproximateDate, EncryptionContext, MemoryType, SchemaVersion, Visibility};
pub use error::CoreError;
pub use manifest::{Manifest, ManifestEntry};
pub use memory::Memory;
pub use metadata::{Location, MemoryMetadata, Person};
pub use network::*;
#[cfg(feature = "service")]
pub use ports::*;
pub use replication::*;
pub use search::*;
#[cfg(feature = "service")]
pub use service::{CreateInput, FileInput, FileVerification, TesseraService, VerifyReport};
pub use tessera::{HeirShareMeta, HybridEncryptionPublic, Tessera, TesseraIdentity};
pub use types::{ContentHash, HashPrefix, NodeId};
