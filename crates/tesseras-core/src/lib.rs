//! tesseras-core: domain types, tessera format, and serialization.

pub mod enums;
pub mod error;
pub mod metadata;
pub mod types;

pub use enums::{ApproximateDate, MemoryType, SchemaVersion, Visibility};
pub use error::CoreError;
pub use metadata::{Location, MemoryMetadata, Person};
pub use types::{ContentHash, NodeId};
