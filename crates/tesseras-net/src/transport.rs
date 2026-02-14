use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tesseras_core::NodeId;
use tokio::sync::oneshot;

use crate::error::NetError;

/// Peer address with optional NodeId (None until handshake completes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerAddr {
    pub node_id: Option<NodeId>,
    pub addr: SocketAddr,
}

/// Incoming message envelope.
///
/// When received from a bidirectional stream (QUIC), `response_tx` allows the
/// handler to write a response back on the **same** stream instead of opening a
/// new connection.
pub struct Envelope {
    pub peer: PeerAddr,
    pub payload: Vec<u8>,
    /// Optional channel for sending a response on the same bidirectional stream.
    pub response_tx: Option<oneshot::Sender<Vec<u8>>>,
}

impl std::fmt::Debug for Envelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Envelope")
            .field("peer", &self.peer)
            .field("payload_len", &self.payload.len())
            .field("has_response_tx", &self.response_tx.is_some())
            .finish()
    }
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
    fn local_addrs(&self) -> Vec<SocketAddr>;
    fn local_addr(&self) -> SocketAddr {
        self.local_addrs()[0]
    }
}

#[async_trait]
impl<T: Transport> Transport for Arc<T> {
    async fn send(&self, peer: &PeerAddr, data: &[u8]) -> Result<(), NetError> {
        (**self).send(peer, data).await
    }

    async fn recv(&self) -> Result<Envelope, NetError> {
        (**self).recv().await
    }

    async fn disconnect(&self, peer: &PeerAddr) {
        (**self).disconnect(peer).await
    }

    fn local_addrs(&self) -> Vec<SocketAddr> {
        (**self).local_addrs()
    }

    fn local_addr(&self) -> SocketAddr {
        (**self).local_addr()
    }
}
