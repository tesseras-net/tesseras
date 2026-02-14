//! In-memory transport for testing. MemTransport implements Transport using
//! tokio::sync::mpsc channels routed through a shared SimNetwork.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};

use crate::error::NetError;
use crate::transport::{Envelope, PeerAddr, Transport};

/// Simulated network that routes messages between MemTransport instances.
pub struct SimNetwork {
    routes: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Envelope>>>>,
}

impl SimNetwork {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new MemTransport connected to this network.
    pub async fn create_transport(&self, addr: SocketAddr, capacity: usize) -> MemTransport {
        let (tx, rx) = mpsc::channel(capacity);
        self.routes.lock().await.insert(addr, tx);
        MemTransport {
            addr,
            rx: Mutex::new(rx),
            routes: Arc::clone(&self.routes),
        }
    }
}

impl Default for SimNetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory Transport implementation for testing.
pub struct MemTransport {
    addr: SocketAddr,
    rx: Mutex<mpsc::Receiver<Envelope>>,
    routes: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Envelope>>>>,
}

#[async_trait]
impl Transport for MemTransport {
    async fn send(&self, peer: &PeerAddr, data: &[u8]) -> Result<(), NetError> {
        let routes = self.routes.lock().await;
        let sender = routes
            .get(&peer.addr)
            .ok_or_else(|| NetError::ConnectionFailed(format!("no route to {}", peer.addr)))?;
        sender
            .send(Envelope {
                peer: PeerAddr {
                    node_id: None,
                    addr: self.addr,
                },
                payload: data.to_vec(),
                response_tx: None,
            })
            .await
            .map_err(|_| NetError::SendFailed("channel closed".into()))
    }

    async fn recv(&self) -> Result<Envelope, NetError> {
        let mut rx = self.rx.lock().await;
        rx.recv().await.ok_or(NetError::Closed)
    }

    async fn disconnect(&self, peer: &PeerAddr) {
        self.routes.lock().await.remove(&peer.addr);
    }

    fn local_addrs(&self) -> Vec<SocketAddr> {
        vec![self.addr]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    fn peer(port: u16) -> PeerAddr {
        PeerAddr {
            node_id: None,
            addr: addr(port),
        }
    }

    #[tokio::test]
    async fn send_recv_basic() {
        let net = SimNetwork::new();
        let t1 = net.create_transport(addr(1001), 16).await;
        let t2 = net.create_transport(addr(1002), 16).await;

        t1.send(&peer(1002), b"hello").await.unwrap();
        let env = t2.recv().await.unwrap();
        assert_eq!(env.payload, b"hello");
        assert_eq!(env.peer.addr, addr(1001));
    }

    #[tokio::test]
    async fn send_to_unknown_peer_fails() {
        let net = SimNetwork::new();
        let t1 = net.create_transport(addr(2001), 16).await;
        let result = t1.send(&peer(9999), b"hi").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn disconnect_removes_route() {
        let net = SimNetwork::new();
        let t1 = net.create_transport(addr(3001), 16).await;
        let _t2 = net.create_transport(addr(3002), 16).await;

        t1.disconnect(&peer(3002)).await;
        let result = t1.send(&peer(3002), b"hi").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn multiple_messages_in_order() {
        let net = SimNetwork::new();
        let t1 = net.create_transport(addr(4001), 16).await;
        let t2 = net.create_transport(addr(4002), 16).await;

        t1.send(&peer(4002), b"first").await.unwrap();
        t1.send(&peer(4002), b"second").await.unwrap();

        let e1 = t2.recv().await.unwrap();
        let e2 = t2.recv().await.unwrap();
        assert_eq!(e1.payload, b"first");
        assert_eq!(e2.payload, b"second");
    }

    #[tokio::test]
    async fn backpressure_when_channel_full() {
        let net = SimNetwork::new();
        let t1 = net.create_transport(addr(5001), 1).await;
        let t2 = net.create_transport(addr(5002), 1).await;

        // Fill the channel (capacity 1)
        t1.send(&peer(5002), b"msg1").await.unwrap();

        // Next send should block (we use try_send semantics via timeout)
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            t1.send(&peer(5002), b"msg2"),
        )
        .await;
        // Should timeout because channel is full
        assert!(result.is_err());

        // Drain one message, then the blocked send should succeed
        let _ = t2.recv().await.unwrap();
    }
}
