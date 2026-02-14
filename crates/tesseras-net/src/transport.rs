use std::net::SocketAddr;

use async_trait::async_trait;
use tesseras_core::NodeId;

use crate::error::NetError;

/// Peer address with optional NodeId (None until handshake completes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerAddr {
    pub node_id: Option<NodeId>,
    pub addr: SocketAddr,
}

/// Incoming message envelope.
#[derive(Debug, Clone)]
pub struct Envelope {
    pub peer: PeerAddr,
    pub payload: Vec<u8>,
}

/// Transport port for the DHT engine.
///
/// `recv()` must be called from a single task. Concurrent calls are not supported.
/// `send()` is implicitly connect-and-send: the adapter maintains a connection pool.
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, peer: &PeerAddr, data: &[u8]) -> Result<(), NetError>;
    async fn recv(&self) -> Result<Envelope, NetError>;
    async fn disconnect(&self, peer: &PeerAddr);
    fn local_addr(&self) -> SocketAddr;
}
