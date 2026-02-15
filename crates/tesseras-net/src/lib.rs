//! tesseras-net: QUIC transport, NAT traversal, local discovery.

pub mod codec;
pub mod error;
pub mod mem;
#[cfg(feature = "quic")]
pub mod quinn_transport;
pub mod stun;
pub mod transport;

pub use error::NetError;
pub use mem::{MemTransport, SimNetwork};
#[cfg(feature = "quic")]
pub use quinn_transport::QuinnTransport;
pub use transport::{Envelope, PeerAddr, Transport};
