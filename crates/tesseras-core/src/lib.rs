//! tesseras-core: domain types, tessera format, and serialization.

pub mod enums;
pub mod error;
pub mod manifest;
pub mod memory;
pub mod metadata;
pub mod tessera;
pub mod types;

pub use enums::{ApproximateDate, MemoryType, SchemaVersion, Visibility};
pub use error::CoreError;
pub use manifest::{Manifest, ManifestEntry};
pub use memory::Memory;
pub use metadata::{Location, MemoryMetadata, Person};
pub use tessera::{Tessera, TesseraIdentity};
pub use types::{ContentHash, NodeId};
