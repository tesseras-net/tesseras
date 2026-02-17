use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::config::{DataDir, NodeConfig};
use crate::crypto::{self, Identity};
use crate::dht::{Dht, DhtMessage, PeerInfo};
use crate::net::{self, QuicTransport};
use crate::storage::Storage;
use crate::types::{ContentHash, MediaType, Memory, NodeId, Tessera, Visibility};

/// Kademlia replication factor — how many nodes to query/store to.
const K: usize = 20;

/// Interval between routing table refresh rounds.
const REFRESH_INTERVAL: Duration = Duration::from_secs(60);

/// Timeout for a single DHT RPC round-trip.
const RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// The Node orchestrator: ties storage, DHT, QUIC, and replication together.
pub struct Node {
    pub storage: Storage,
    pub identity: Identity,
    pub config: NodeConfig,
    pub dht: Arc<Mutex<Dht>>,
    transport: Option<Arc<QuicTransport>>,
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl Node {
    /// Create a new node with the given data directory.
    pub fn new(
        data_dir: DataDir,
        identity: Identity,
        config: NodeConfig,
    ) -> Result<Self, NodeError> {
        let storage = Storage::open(data_dir).map_err(|e| NodeError::Storage(e.to_string()))?;
        let node_id = identity.node_id();
        let dht = Arc::new(Mutex::new(Dht::new(node_id)));

        Ok(Self {
            storage,
            identity,
            config,
            dht,
            transport: None,
            shutdown_tx: None,
        })
    }

    /// Start the QUIC transport and spawn the accept loop.
    /// Returns the local address this node is listening on.
    pub async fn start(&mut self) -> Result<SocketAddr, NodeError> {
        let transport = Arc::new(
            QuicTransport::bind(self.config.listen)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?,
        );
        let addr = transport
            .local_addr()
            .map_err(|e| NodeError::Network(e.to_string()))?;

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);
        self.transport = Some(transport.clone());

        // Spawn the accept loop
        let dht = self.dht.clone();
        let mut rx = shutdown_rx.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = transport.accept() => {
                        match result {
                            Ok(conn) => {
                                let dht = dht.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(conn, dht).await {
                                        debug!("connection handler error: {e}");
                                    }
                                });
                            }
                            Err(e) => {
                                debug!("accept error: {e}");
                                break;
                            }
                        }
                    }
                    _ = rx.changed() => {
                        info!("accept loop shutting down");
                        break;
                    }
                }
            }
        });

        info!("node listening on {addr}");
        Ok(addr)
    }

    /// Bootstrap the DHT by contacting configured bootstrap nodes.
    /// Sends FindNode(self) to each bootstrap peer to populate the routing table.
    pub async fn bootstrap(&self) -> Result<usize, NodeError> {
        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| NodeError::Network("transport not started".into()))?;

        let node_id = self.identity.node_id();
        let mut discovered = 0usize;

        for addr_str in &self.config.bootstrap {
            let addr: SocketAddr = match addr_str.parse() {
                Ok(a) => a,
                Err(e) => {
                    warn!("invalid bootstrap address {addr_str}: {e}");
                    continue;
                }
            };

            let msg = DhtMessage::FindNode {
                sender: node_id,
                target: node_id,
            };

            match self.send_rpc(transport, addr, &msg).await {
                Ok(Some(DhtMessage::FindNodeResponse { sender, closest })) => {
                    let mut dht = self.dht.lock().unwrap();
                    dht.routing_table.insert(PeerInfo {
                        node_id: sender,
                        addr,
                    });
                    for peer in &closest {
                        dht.routing_table.insert(peer.clone());
                    }
                    discovered += closest.len() + 1;
                    info!(
                        "bootstrapped from {addr}: learned {} peers",
                        closest.len() + 1
                    );
                }
                Ok(Some(other)) => {
                    debug!("unexpected bootstrap response from {addr}: {other:?}");
                }
                Ok(None) => {
                    debug!("no response from bootstrap {addr}");
                }
                Err(e) => {
                    warn!("bootstrap failed for {addr}: {e}");
                }
            }
        }

        Ok(discovered)
    }

    /// Spawn a periodic routing table refresh task.
    /// Every REFRESH_INTERVAL, picks a random NodeId and does FindNode to keep
    /// the routing table fresh and discover new peers.
    pub fn start_refresh_loop(&self) {
        let transport = match self.transport.as_ref() {
            Some(t) => t.clone(),
            None => return,
        };
        let dht = self.dht.clone();
        let node_id = self.identity.node_id();
        let mut shutdown_rx = self
            .shutdown_tx
            .as_ref()
            .expect("start() must be called first")
            .subscribe();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(REFRESH_INTERVAL);
            interval.tick().await; // skip immediate first tick

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        refresh_routing_table(&transport, &dht, node_id).await;
                    }
                    _ = shutdown_rx.changed() => {
                        info!("refresh loop shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Announce a tessera to the DHT by publishing Store messages to the
    /// K closest nodes to the tessera's content hash.
    pub async fn announce_tessera(&self, hash: &ContentHash) -> Result<usize, NodeError> {
        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| NodeError::Network("transport not started".into()))?;

        let node_id = self.identity.node_id();
        let local_addr = self
            .local_addr()
            .ok_or_else(|| NodeError::Network("no local address".into()))?;

        // Find the K closest nodes to this content hash
        let target_id = NodeId::new(*hash.as_bytes());
        let closest = {
            let dht = self.dht.lock().unwrap();
            dht.routing_table.find_closest(&target_id, K)
        };

        if closest.is_empty() {
            return Ok(0);
        }

        let provider = PeerInfo {
            node_id,
            addr: local_addr,
        };

        let mut stored = 0usize;
        for peer in &closest {
            let msg = DhtMessage::Store {
                sender: node_id,
                key: *hash,
                provider: provider.clone(),
            };

            match self.send_rpc(transport, peer.addr, &msg).await {
                Ok(Some(DhtMessage::StoreResponse { success: true, .. })) => {
                    stored += 1;
                }
                Ok(resp) => {
                    debug!("unexpected store response from {}: {resp:?}", peer.addr);
                }
                Err(e) => {
                    debug!("store RPC to {} failed: {e}", peer.addr);
                }
            }
        }

        info!(
            "announced tessera {hash} to {stored}/{} peers",
            closest.len()
        );
        Ok(stored)
    }

    /// Look up providers for a content hash via DHT FindValue.
    /// First checks the local pointer store, then queries the K closest peers.
    pub async fn find_providers(&self, hash: &ContentHash) -> Result<Vec<PeerInfo>, NodeError> {
        // Check local pointer store first
        {
            let dht = self.dht.lock().unwrap();
            if let Some(providers) = dht.pointer_store.find(hash) {
                return Ok(providers.clone());
            }
        }

        let transport = match self.transport.as_ref() {
            Some(t) => t,
            None => return Ok(Vec::new()),
        };

        let node_id = self.identity.node_id();
        let target_id = NodeId::new(*hash.as_bytes());
        let closest = {
            let dht = self.dht.lock().unwrap();
            dht.routing_table.find_closest(&target_id, K)
        };

        for peer in &closest {
            let msg = DhtMessage::FindValue {
                sender: node_id,
                key: *hash,
            };

            match self.send_rpc(transport, peer.addr, &msg).await {
                Ok(Some(DhtMessage::FindValueResponse {
                    pointers: Some(providers),
                    ..
                })) => {
                    return Ok(providers);
                }
                Ok(Some(DhtMessage::FindValueResponse {
                    closest: Some(new_closest),
                    sender,
                    ..
                })) => {
                    let mut dht = self.dht.lock().unwrap();
                    dht.routing_table.insert(PeerInfo {
                        node_id: sender,
                        addr: peer.addr,
                    });
                    for p in new_closest {
                        dht.routing_table.insert(p);
                    }
                }
                _ => {}
            }
        }

        Ok(Vec::new())
    }

    /// Send a DHT RPC to a peer and wait for a response.
    async fn send_rpc(
        &self,
        transport: &QuicTransport,
        addr: SocketAddr,
        msg: &DhtMessage,
    ) -> Result<Option<DhtMessage>, NodeError> {
        let result = tokio::time::timeout(RPC_TIMEOUT, async {
            let conn = transport
                .connect(addr)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;

            let (mut send, mut recv) = conn
                .open_bi()
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;

            net::send_message(&mut send, msg)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;
            send.finish()
                .map_err(|e| NodeError::Network(e.to_string()))?;

            let response = net::receive_message(&mut recv)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;

            Ok::<_, NodeError>(Some(response))
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => {
                debug!("RPC to {addr} timed out");
                Ok(None)
            }
        }
    }

    /// Add a tessera from local files.
    pub fn add_tessera(
        &self,
        files: &[std::path::PathBuf],
        name: Option<String>,
        visibility: Visibility,
    ) -> Result<Tessera, NodeError> {
        let mut memories = Vec::new();
        for file_path in files {
            let filename = file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed".into());

            let ext = file_path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();

            let size = std::fs::metadata(file_path)
                .map_err(|e| NodeError::Io(e.to_string()))?
                .len();

            let mut reader =
                std::fs::File::open(file_path).map_err(|e| NodeError::Io(e.to_string()))?;
            let blob_hash = self
                .storage
                .store_blob(&mut reader)
                .map_err(|e| NodeError::Storage(e.to_string()))?;

            memories.push(Memory {
                filename,
                media_type: MediaType::from_extension(&ext),
                size,
                blob_hash,
            });
        }

        let content =
            rmp_serde::to_vec(&memories).map_err(|e| NodeError::Serialization(e.to_string()))?;
        let hash = crypto::hash_bytes(&content);
        let signature = self.identity.sign(&content);

        let tessera = Tessera {
            hash,
            author: self.identity.public_key_bytes(),
            signature,
            created_at: chrono::Utc::now(),
            name,
            visibility,
            memories,
        };

        self.storage
            .store_tessera(&tessera)
            .map_err(|e| NodeError::Storage(e.to_string()))?;

        Ok(tessera)
    }

    /// Get a tessera by hash (local lookup).
    pub fn get_tessera(&self, hash: &ContentHash) -> Result<Option<Tessera>, NodeError> {
        self.storage
            .find_tessera(hash)
            .map_err(|e| NodeError::Storage(e.to_string()))
    }

    /// Remove a tessera and its blobs.
    pub fn remove_tessera(&self, hash: &ContentHash) -> Result<(), NodeError> {
        let tessera = self
            .storage
            .find_tessera(hash)
            .map_err(|e| NodeError::Storage(e.to_string()))?
            .ok_or_else(|| NodeError::NotFound(hash.to_string()))?;

        for memory in &tessera.memories {
            self.storage
                .delete_blob(&memory.blob_hash)
                .map_err(|e| NodeError::Storage(e.to_string()))?;
        }

        self.storage
            .delete_tessera(hash)
            .map_err(|e| NodeError::Storage(e.to_string()))?;

        Ok(())
    }

    /// List all tesseras.
    pub fn list_tesseras(&self) -> Result<Vec<Tessera>, NodeError> {
        self.storage
            .list_tesseras()
            .map_err(|e| NodeError::Storage(e.to_string()))
    }

    /// Get the node's ID.
    pub fn node_id(&self) -> NodeId {
        self.identity.node_id()
    }

    /// Get the local address this node is listening on.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.transport.as_ref()?.local_addr().ok()
    }

    /// Number of peers in the routing table.
    pub fn peer_count(&self) -> usize {
        self.dht.lock().unwrap().routing_table.len()
    }

    /// Close the transport and signal background tasks to stop.
    pub fn shutdown(&self) {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        if let Some(transport) = &self.transport {
            transport.close();
        }
    }
}

/// Handle a single incoming QUIC connection: read messages, dispatch to DHT, send responses.
async fn handle_connection(conn: quinn::Connection, dht: Arc<Mutex<Dht>>) -> Result<(), NodeError> {
    let remote_addr = conn.remote_address();

    loop {
        let stream = match conn.accept_bi().await {
            Ok(s) => s,
            Err(quinn::ConnectionError::ApplicationClosed(_)) => break,
            Err(e) => {
                debug!("stream accept error from {remote_addr}: {e}");
                break;
            }
        };

        let (mut send, mut recv) = stream;
        let dht = dht.clone();

        tokio::spawn(async move {
            let msg = match net::receive_message(&mut recv).await {
                Ok(m) => m,
                Err(e) => {
                    debug!("receive error from {remote_addr}: {e}");
                    return;
                }
            };

            let response = {
                let mut dht = dht.lock().unwrap();
                dht.handle_message(msg, remote_addr)
            };

            if let Some(resp) = response {
                if let Err(e) = net::send_message(&mut send, &resp).await {
                    debug!("send error to {remote_addr}: {e}");
                }
                let _ = send.finish();
            }
        });
    }

    Ok(())
}

/// Refresh the routing table by doing FindNode for a random ID in each non-empty bucket.
async fn refresh_routing_table(transport: &QuicTransport, dht: &Arc<Mutex<Dht>>, node_id: NodeId) {
    // Pick a random peer to query
    let peers: Vec<PeerInfo> = {
        let dht = dht.lock().unwrap();
        dht.routing_table.find_closest(&node_id, 3)
    };

    // Generate a random target for the lookup
    let random_target = {
        let mut bytes = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut bytes);
        NodeId::new(bytes)
    };

    for peer in &peers {
        let msg = DhtMessage::FindNode {
            sender: node_id,
            target: random_target,
        };

        let result = tokio::time::timeout(RPC_TIMEOUT, async {
            let conn = transport.connect(peer.addr).await?;
            let (mut send, mut recv) = conn
                .open_bi()
                .await
                .map_err(|e| net::NetError::Connection(e.to_string()))?;
            net::send_message(&mut send, &msg).await?;
            send.finish()
                .map_err(|e| net::NetError::Write(e.to_string()))?;
            net::receive_message(&mut recv).await
        })
        .await;

        match result {
            Ok(Ok(DhtMessage::FindNodeResponse { sender, closest })) => {
                let mut dht = dht.lock().unwrap();
                dht.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: peer.addr,
                });
                for p in closest {
                    dht.routing_table.insert(p);
                }
                debug!("refresh: learned peers from {}", peer.addr);
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                debug!("refresh RPC to {} failed: {e}", peer.addr);
            }
            Err(_) => {
                debug!("refresh RPC to {} timed out", peer.addr);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("not found: {0}")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node() -> (tempfile::TempDir, Node) {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = DataDir::open(tmp.path()).unwrap();
        let identity = Identity::generate();
        let config = NodeConfig::default();
        let node = Node::new(data_dir, identity, config).unwrap();
        (tmp, node)
    }

    #[test]
    fn add_and_get_tessera() {
        let (tmp, node) = test_node();

        let test_file = tmp.path().join("test.txt");
        std::fs::write(&test_file, b"hello world").unwrap();

        let tessera = node
            .add_tessera(&[test_file], Some("Test".into()), Visibility::Public)
            .unwrap();

        assert_eq!(tessera.name, Some("Test".into()));
        assert_eq!(tessera.memories.len(), 1);

        let found = node.get_tessera(&tessera.hash).unwrap().unwrap();
        assert_eq!(found.name, Some("Test".into()));
    }

    #[test]
    fn remove_tessera() {
        let (tmp, node) = test_node();

        let test_file = tmp.path().join("test.txt");
        std::fs::write(&test_file, b"to be removed").unwrap();

        let tessera = node
            .add_tessera(&[test_file], None, Visibility::Private)
            .unwrap();

        node.remove_tessera(&tessera.hash).unwrap();
        assert!(node.get_tessera(&tessera.hash).unwrap().is_none());
    }

    #[test]
    fn list_tesseras() {
        let (tmp, node) = test_node();

        let file1 = tmp.path().join("a.txt");
        let file2 = tmp.path().join("b.txt");
        std::fs::write(&file1, b"file a").unwrap();
        std::fs::write(&file2, b"file b").unwrap();

        node.add_tessera(&[file1], Some("A".into()), Visibility::Public)
            .unwrap();
        node.add_tessera(&[file2], Some("B".into()), Visibility::Public)
            .unwrap();

        let list = node.list_tesseras().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn start_and_accept_ping() {
        let (tmp, mut node_a) = test_node();
        let _ = tmp; // keep alive

        let mut config_a = NodeConfig::default();
        config_a.listen = "127.0.0.1:0".parse().unwrap();
        node_a.config = config_a;

        let addr_a = node_a.start().await.unwrap();

        // Create a client and send a ping
        let client = QuicTransport::client().unwrap();
        let conn = client.connect(addr_a).await.unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();

        let node_b_id = NodeId::new([0x42; 32]);
        let ping = DhtMessage::Ping { sender: node_b_id };
        net::send_message(&mut send, &ping).await.unwrap();
        send.finish().unwrap();

        let response = net::receive_message(&mut recv).await.unwrap();
        if let DhtMessage::Pong { sender } = response {
            assert_eq!(sender, node_a.node_id());
        } else {
            panic!("expected Pong, got {response:?}");
        }

        // Node A should have learned about the ping sender
        assert!(
            node_a
                .dht
                .lock()
                .unwrap()
                .routing_table
                .contains(&node_b_id)
        );

        node_a.shutdown();
    }

    #[tokio::test]
    async fn two_nodes_bootstrap() {
        // Node A
        let (tmp_a, mut node_a) = test_node();
        let _ = tmp_a;
        node_a.config.listen = "127.0.0.1:0".parse().unwrap();
        let addr_a = node_a.start().await.unwrap();

        // Node B with A as bootstrap
        let tmp_b = tempfile::tempdir().unwrap();
        let data_dir_b = DataDir::open(tmp_b.path()).unwrap();
        let identity_b = Identity::generate();
        let mut config_b = NodeConfig::default();
        config_b.listen = "127.0.0.1:0".parse().unwrap();
        config_b.bootstrap = vec![addr_a.to_string()];
        let mut node_b = Node::new(data_dir_b, identity_b, config_b).unwrap();
        let _addr_b = node_b.start().await.unwrap();

        // Bootstrap B from A
        let discovered = node_b.bootstrap().await.unwrap();
        assert!(discovered > 0, "should discover at least 1 peer");

        // B should know about A
        assert!(
            node_b
                .dht
                .lock()
                .unwrap()
                .routing_table
                .contains(&node_a.node_id())
        );

        // A should know about B (from the FindNode request)
        assert!(
            node_a
                .dht
                .lock()
                .unwrap()
                .routing_table
                .contains(&node_b.node_id())
        );

        node_a.shutdown();
        node_b.shutdown();
    }

    #[tokio::test]
    async fn announce_and_find_tessera() {
        // Node A
        let (tmp_a, mut node_a) = test_node();
        node_a.config.listen = "127.0.0.1:0".parse().unwrap();
        let addr_a = node_a.start().await.unwrap();

        // Node B with A as bootstrap
        let tmp_b = tempfile::tempdir().unwrap();
        let data_dir_b = DataDir::open(tmp_b.path()).unwrap();
        let identity_b = Identity::generate();
        let mut config_b = NodeConfig::default();
        config_b.listen = "127.0.0.1:0".parse().unwrap();
        config_b.bootstrap = vec![addr_a.to_string()];
        let mut node_b = Node::new(data_dir_b, identity_b, config_b).unwrap();
        let _addr_b = node_b.start().await.unwrap();

        // Bootstrap B from A
        node_b.bootstrap().await.unwrap();

        // Node A adds a tessera
        let test_file = tmp_a.path().join("memory.txt");
        std::fs::write(&test_file, b"a precious memory").unwrap();
        let tessera = node_a
            .add_tessera(&[test_file], Some("My Memory".into()), Visibility::Public)
            .unwrap();

        // Node A announces it to the DHT (B is the only peer)
        let stored = node_a.announce_tessera(&tessera.hash).await.unwrap();
        assert!(stored > 0, "should store to at least 1 peer");

        // Node B should be able to find the provider via DHT
        let providers = node_b.find_providers(&tessera.hash).await.unwrap();
        assert!(!providers.is_empty(), "should find at least 1 provider");
        assert_eq!(providers[0].node_id, node_a.node_id());

        node_a.shutdown();
        node_b.shutdown();
    }
}
