use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::config::{DataDir, NodeConfig};
use crate::crypto::{self, Identity};
use crate::dht::{Dht, DhtMessage, PeerInfo};
use crate::net::{self, QuicTransport};
use crate::replication;
use crate::storage::Storage;
use crate::types::{ContentHash, MediaType, Memory, NodeId, Tessera, Visibility};

/// Kademlia replication factor — how many nodes to query/store to.
const K: usize = 20;

/// Interval between routing table refresh rounds.
const REFRESH_INTERVAL: Duration = Duration::from_secs(60);

/// Interval between repair loop rounds (check fragment availability).
const REPAIR_INTERVAL: Duration = Duration::from_secs(300);

/// Timeout for a single DHT RPC round-trip.
const RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// The Node orchestrator: ties storage, DHT, QUIC, and replication together.
pub struct Node {
    pub storage: Arc<Mutex<Storage>>,
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
            storage: Arc::new(Mutex::new(storage)),
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
        let storage = self.storage.clone();
        let node_id = self.identity.node_id();
        let mut rx = shutdown_rx.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = transport.accept() => {
                        match result {
                            Ok(conn) => {
                                let dht = dht.clone();
                                let storage = storage.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(conn, dht, storage, node_id).await {
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
    pub async fn find_providers(&self, hash: &ContentHash) -> Result<Vec<PeerInfo>, NodeError> {
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

    /// Fetch a blob from a remote peer by hash.
    pub async fn fetch_blob(
        &self,
        addr: SocketAddr,
        hash: &ContentHash,
    ) -> Result<Option<Vec<u8>>, NodeError> {
        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| NodeError::Network("transport not started".into()))?;

        let node_id = self.identity.node_id();
        let msg = DhtMessage::FetchBlob {
            sender: node_id,
            hash: *hash,
        };

        let result = tokio::time::timeout(RPC_TIMEOUT, async {
            let conn = transport
                .connect(addr)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;

            let (mut send, mut recv) = conn
                .open_bi()
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;

            net::send_message(&mut send, &msg)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;
            send.finish()
                .map_err(|e| NodeError::Network(e.to_string()))?;

            let response = net::receive_message(&mut recv)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;

            if let DhtMessage::FetchBlobResponse {
                found: true, size, ..
            } = response
            {
                let data = net::receive_blob(&mut recv)
                    .await
                    .map_err(|e| NodeError::Network(e.to_string()))?;
                if data.len() as u64 != size {
                    return Err(NodeError::Network("blob size mismatch".into()));
                }
                Ok(Some(data))
            } else {
                Ok(None)
            }
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => {
                debug!("fetch_blob from {addr} timed out");
                Ok(None)
            }
        }
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

    /// Add a tessera from local files, erasure-code each blob into fragments.
    pub fn add_tessera(
        &self,
        files: &[std::path::PathBuf],
        name: Option<String>,
        visibility: Visibility,
    ) -> Result<Tessera, NodeError> {
        let storage = self.storage.lock().unwrap();
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
            let blob_hash = storage
                .store_blob(&mut reader)
                .map_err(|e| NodeError::Storage(e.to_string()))?;

            // Erasure-code the blob into fragments
            let blob_data = storage
                .read_blob_bytes(&blob_hash)
                .map_err(|e| NodeError::Storage(e.to_string()))?;
            let fragments = replication::encode_fragments(
                &blob_data,
                self.config.data_shards,
                self.config.parity_shards,
            )
            .map_err(|e| NodeError::Replication(e.to_string()))?;

            for fragment in &fragments {
                let frag_hash = storage
                    .store_blob_bytes(&fragment.data)
                    .map_err(|e| NodeError::Storage(e.to_string()))?;
                storage
                    .store_fragment(
                        &blob_hash,
                        &crate::storage::FragmentMeta {
                            fragment_index: fragment.index,
                            fragment_hash: frag_hash,
                            shard_size: fragment.data.len(),
                            original_size: blob_data.len(),
                            data_shards: self.config.data_shards,
                            parity_shards: self.config.parity_shards,
                        },
                    )
                    .map_err(|e| NodeError::Storage(e.to_string()))?;
            }

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

        storage
            .store_tessera(&tessera)
            .map_err(|e| NodeError::Storage(e.to_string()))?;

        Ok(tessera)
    }

    /// Distribute fragments of a tessera's blobs to DHT peers.
    /// Announces each fragment hash so peers know where to find them.
    pub async fn distribute_fragments(&self, tessera: &Tessera) -> Result<usize, NodeError> {
        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| NodeError::Network("transport not started".into()))?;

        let node_id = self.identity.node_id();
        let local_addr = self
            .local_addr()
            .ok_or_else(|| NodeError::Network("no local address".into()))?;

        let provider = PeerInfo {
            node_id,
            addr: local_addr,
        };

        let mut total_announced = 0usize;

        for memory in &tessera.memories {
            let fragment_metas = {
                let storage = self.storage.lock().unwrap();
                storage
                    .find_fragments(&memory.blob_hash)
                    .map_err(|e| NodeError::Storage(e.to_string()))?
            };

            for meta in &fragment_metas {
                let target_id = NodeId::new(*meta.fragment_hash.as_bytes());
                let closest = {
                    let dht = self.dht.lock().unwrap();
                    dht.routing_table.find_closest(&target_id, K)
                };

                for peer in &closest {
                    let msg = DhtMessage::Store {
                        sender: node_id,
                        key: meta.fragment_hash,
                        provider: provider.clone(),
                    };
                    if let Ok(Some(DhtMessage::StoreResponse { success: true, .. })) =
                        self.send_rpc(transport, peer.addr, &msg).await
                    {
                        total_announced += 1;
                    }
                }
            }
        }

        info!(
            "distributed {} fragment pointers for tessera {}",
            total_announced, tessera.hash
        );
        Ok(total_announced)
    }

    /// Get a tessera by hash (local lookup).
    pub fn get_tessera(&self, hash: &ContentHash) -> Result<Option<Tessera>, NodeError> {
        self.storage
            .lock()
            .unwrap()
            .find_tessera(hash)
            .map_err(|e| NodeError::Storage(e.to_string()))
    }

    /// Fetch a tessera from the network when not available locally.
    /// Queries DHT for providers, fetches tessera metadata, then fetches
    /// all blobs (or reconstructs from fragments), and caches everything locally.
    pub async fn fetch_tessera_from_network(
        &self,
        hash: &ContentHash,
    ) -> Result<Option<Tessera>, NodeError> {
        // 1. Try local first
        if let Some(t) = self.get_tessera(hash)? {
            return Ok(Some(t));
        }

        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| NodeError::Network("transport not started".into()))?;

        // 2. Find providers via DHT
        let providers = self.find_providers(hash).await?;
        if providers.is_empty() {
            info!("no providers found for tessera {hash}");
            return Ok(None);
        }

        // 3. Try to fetch tessera metadata from each provider
        let mut tessera: Option<Tessera> = None;
        let mut source_addr = None;
        for provider in &providers {
            match self
                .fetch_tessera_metadata(transport, provider.addr, hash)
                .await
            {
                Ok(Some(t)) => {
                    tessera = Some(t);
                    source_addr = Some(provider.addr);
                    break;
                }
                Ok(None) => continue,
                Err(e) => {
                    debug!("fetch tessera metadata from {} failed: {e}", provider.addr);
                    continue;
                }
            }
        }

        let tessera = match tessera {
            Some(t) => t,
            None => {
                info!("no provider had tessera metadata for {hash}");
                return Ok(None);
            }
        };

        // 4. Fetch each blob
        for memory in &tessera.memories {
            // Skip if we already have this blob
            {
                let storage = self.storage.lock().unwrap();
                if storage.has_blob(&memory.blob_hash) {
                    continue;
                }
            }

            // Try to fetch full blob from the source provider first
            let mut got_blob = false;
            if let Some(addr) = source_addr {
                if let Ok(Some(data)) = self.fetch_blob(addr, &memory.blob_hash).await {
                    let actual_hash = crate::crypto::hash_bytes(&data);
                    if actual_hash == memory.blob_hash {
                        let storage = self.storage.lock().unwrap();
                        let _ = storage.store_blob_bytes(&data);
                        got_blob = true;
                    }
                }
            }

            // If that failed, try fetching fragments and reconstructing
            if !got_blob {
                self.fetch_and_reconstruct_blob(&memory.blob_hash, &providers)
                    .await?;
            }
        }

        // 5. Cache tessera metadata locally
        {
            let storage = self.storage.lock().unwrap();
            storage
                .store_tessera(&tessera)
                .map_err(|e| NodeError::Storage(e.to_string()))?;
        }

        info!("fetched tessera {hash} from network");
        Ok(Some(tessera))
    }

    /// Fetch tessera metadata from a remote peer.
    async fn fetch_tessera_metadata(
        &self,
        transport: &QuicTransport,
        addr: SocketAddr,
        hash: &ContentHash,
    ) -> Result<Option<Tessera>, NodeError> {
        let node_id = self.identity.node_id();
        let msg = DhtMessage::FetchTessera {
            sender: node_id,
            hash: *hash,
        };

        let result = tokio::time::timeout(RPC_TIMEOUT, async {
            let conn = transport
                .connect(addr)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;
            let (mut send, mut recv) = conn
                .open_bi()
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;
            net::send_message(&mut send, &msg)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;
            send.finish()
                .map_err(|e| NodeError::Network(e.to_string()))?;

            let response = net::receive_message(&mut recv)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;

            if let DhtMessage::FetchTesseraResponse { tessera, .. } = response {
                Ok(tessera)
            } else {
                Ok(None)
            }
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => {
                debug!("fetch_tessera_metadata from {addr} timed out");
                Ok(None)
            }
        }
    }

    /// Fetch fragments for a blob from DHT peers and reconstruct.
    async fn fetch_and_reconstruct_blob(
        &self,
        blob_hash: &ContentHash,
        providers: &[PeerInfo],
    ) -> Result<bool, NodeError> {
        // First check if we have fragment metadata locally
        let fragment_metas = {
            let storage = self.storage.lock().unwrap();
            storage
                .find_fragments(blob_hash)
                .map_err(|e| NodeError::Storage(e.to_string()))?
        };

        if fragment_metas.is_empty() {
            debug!("no fragment metadata for blob {blob_hash}, cannot reconstruct");
            return Ok(false);
        }

        let data_shards = fragment_metas[0].data_shards;
        let parity_shards = fragment_metas[0].parity_shards;
        let original_size = fragment_metas[0].original_size;
        let total_shards = data_shards + parity_shards;

        // Collect available fragments, fetch missing ones from providers
        let mut shards: Vec<Option<Vec<u8>>> = vec![None; total_shards];

        for meta in &fragment_metas {
            // Check local first
            {
                let storage = self.storage.lock().unwrap();
                if let Ok(data) = storage.read_blob_bytes(&meta.fragment_hash) {
                    if meta.fragment_index < total_shards {
                        shards[meta.fragment_index] = Some(data);
                        continue;
                    }
                }
            }

            // Try to fetch from providers
            for provider in providers {
                if let Ok(Some(data)) = self.fetch_blob(provider.addr, &meta.fragment_hash).await {
                    let actual_hash = crate::crypto::hash_bytes(&data);
                    if actual_hash == meta.fragment_hash {
                        // Cache the fragment locally
                        let storage = self.storage.lock().unwrap();
                        let _ = storage.store_blob_bytes(&data);
                        drop(storage);
                        if meta.fragment_index < total_shards {
                            shards[meta.fragment_index] = Some(data);
                        }
                        break;
                    }
                }
            }
        }

        let present = shards.iter().filter(|s| s.is_some()).count();
        if present < data_shards {
            debug!(
                "only {present}/{data_shards} fragments available for blob {blob_hash}, cannot reconstruct"
            );
            return Ok(false);
        }

        // Reconstruct
        let decode_frags: Vec<replication::Fragment> = shards
            .into_iter()
            .enumerate()
            .filter_map(|(i, data)| data.map(|d| replication::Fragment { index: i, data: d }))
            .collect();

        let reconstructed =
            replication::decode_fragments(&decode_frags, data_shards, parity_shards, original_size)
                .map_err(|e| NodeError::Replication(e.to_string()))?;

        // Verify hash
        let actual_hash = crate::crypto::hash_bytes(&reconstructed);
        if actual_hash != *blob_hash {
            return Err(NodeError::Replication(format!(
                "reconstructed blob hash mismatch: expected {blob_hash}, got {actual_hash}"
            )));
        }

        // Store the reconstructed blob
        let storage = self.storage.lock().unwrap();
        storage
            .store_blob_bytes(&reconstructed)
            .map_err(|e| NodeError::Storage(e.to_string()))?;

        info!("reconstructed blob {blob_hash} from fragments");
        Ok(true)
    }

    /// Remove a tessera, its blobs, and fragment data.
    pub fn remove_tessera(&self, hash: &ContentHash) -> Result<(), NodeError> {
        let storage = self.storage.lock().unwrap();
        let tessera = storage
            .find_tessera(hash)
            .map_err(|e| NodeError::Storage(e.to_string()))?
            .ok_or_else(|| NodeError::NotFound(hash.to_string()))?;

        for memory in &tessera.memories {
            // Delete fragments for this blob
            let fragment_metas = storage
                .find_fragments(&memory.blob_hash)
                .map_err(|e| NodeError::Storage(e.to_string()))?;
            for meta in &fragment_metas {
                storage
                    .delete_blob(&meta.fragment_hash)
                    .map_err(|e| NodeError::Storage(e.to_string()))?;
            }
            storage
                .delete_fragments(&memory.blob_hash)
                .map_err(|e| NodeError::Storage(e.to_string()))?;

            // Delete the original blob
            storage
                .delete_blob(&memory.blob_hash)
                .map_err(|e| NodeError::Storage(e.to_string()))?;
        }

        storage
            .delete_tessera(hash)
            .map_err(|e| NodeError::Storage(e.to_string()))?;

        Ok(())
    }

    /// List all tesseras.
    pub fn list_tesseras(&self) -> Result<Vec<Tessera>, NodeError> {
        self.storage
            .lock()
            .unwrap()
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

    /// Spawn a periodic repair loop that checks fragment availability
    /// and re-replicates missing fragments by fetching from DHT peers.
    pub fn start_repair_loop(&self) {
        let transport = match self.transport.as_ref() {
            Some(t) => t.clone(),
            None => return,
        };
        let dht = self.dht.clone();
        let storage = self.storage.clone();
        let node_id = self.identity.node_id();
        let data_shards = self.config.data_shards;
        let parity_shards = self.config.parity_shards;
        let mut shutdown_rx = self
            .shutdown_tx
            .as_ref()
            .expect("start() must be called first")
            .subscribe();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(REPAIR_INTERVAL);
            interval.tick().await; // skip immediate first tick

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        repair_fragments(
                            &transport, &dht, &storage, node_id,
                            data_shards, parity_shards,
                        ).await;
                    }
                    _ = shutdown_rx.changed() => {
                        info!("repair loop shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Check fragment availability for all tesseras. Returns a list of
    /// (blob_hash, fragment_index, fragment_hash) tuples for missing fragments.
    pub fn check_fragments(&self) -> Result<Vec<(ContentHash, usize, ContentHash)>, NodeError> {
        let storage = self.storage.lock().unwrap();
        let tesseras = storage
            .list_tesseras()
            .map_err(|e| NodeError::Storage(e.to_string()))?;

        let mut missing = Vec::new();
        for tessera in &tesseras {
            for memory in &tessera.memories {
                let fragments = storage
                    .find_fragments(&memory.blob_hash)
                    .map_err(|e| NodeError::Storage(e.to_string()))?;
                for meta in &fragments {
                    if !storage.has_blob(&meta.fragment_hash) {
                        missing.push((memory.blob_hash, meta.fragment_index, meta.fragment_hash));
                    }
                }
            }
        }
        Ok(missing)
    }

    /// Get a receiver for the shutdown signal (for passing to RPC server, etc).
    pub fn shutdown_rx(&self) -> Option<watch::Receiver<bool>> {
        self.shutdown_tx.as_ref().map(|tx| tx.subscribe())
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

/// Handle a single incoming QUIC connection.
async fn handle_connection(
    conn: quinn::Connection,
    dht: Arc<Mutex<Dht>>,
    storage: Arc<Mutex<Storage>>,
    local_node_id: NodeId,
) -> Result<(), NodeError> {
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
        let storage = storage.clone();

        tokio::spawn(async move {
            let msg = match net::receive_message(&mut recv).await {
                Ok(m) => m,
                Err(e) => {
                    debug!("receive error from {remote_addr}: {e}");
                    return;
                }
            };

            // Handle FetchTessera — return tessera metadata from local storage
            if let DhtMessage::FetchTessera { sender, hash } = &msg {
                {
                    let mut dht = dht.lock().unwrap();
                    dht.routing_table.insert(PeerInfo {
                        node_id: *sender,
                        addr: remote_addr,
                    });
                }

                let tessera = {
                    let storage = storage.lock().unwrap();
                    storage.find_tessera(hash).ok().flatten()
                };

                let resp = DhtMessage::FetchTesseraResponse {
                    sender: local_node_id,
                    tessera,
                };
                if let Err(e) = net::send_message(&mut send, &resp).await {
                    debug!("send FetchTesseraResponse error to {remote_addr}: {e}");
                }
                let _ = send.finish();
                return;
            }

            // Handle FetchBlob specially — needs storage access and blob streaming
            if let DhtMessage::FetchBlob { sender, hash } = &msg {
                // Update routing table
                {
                    let mut dht = dht.lock().unwrap();
                    dht.routing_table.insert(PeerInfo {
                        node_id: *sender,
                        addr: remote_addr,
                    });
                }

                let blob_data = {
                    let storage = storage.lock().unwrap();
                    if storage.has_blob(hash) {
                        storage.read_blob_bytes(hash).ok()
                    } else {
                        None
                    }
                };

                if let Some(data) = blob_data {
                    let resp = DhtMessage::FetchBlobResponse {
                        sender: local_node_id,
                        found: true,
                        size: data.len() as u64,
                    };
                    if let Err(e) = net::send_message(&mut send, &resp).await {
                        debug!("send FetchBlobResponse error to {remote_addr}: {e}");
                        return;
                    }
                    if let Err(e) = net::stream_blob(&mut send, &data).await {
                        debug!("stream blob error to {remote_addr}: {e}");
                    }
                } else {
                    let resp = DhtMessage::FetchBlobResponse {
                        sender: local_node_id,
                        found: false,
                        size: 0,
                    };
                    if let Err(e) = net::send_message(&mut send, &resp).await {
                        debug!("send FetchBlobResponse error to {remote_addr}: {e}");
                    }
                    let _ = send.finish();
                }
                return;
            }

            // All other messages go through normal DHT handling
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

/// Check and repair missing fragments by fetching from DHT peers.
async fn repair_fragments(
    transport: &QuicTransport,
    dht: &Arc<Mutex<Dht>>,
    storage: &Arc<Mutex<Storage>>,
    node_id: NodeId,
    data_shards: usize,
    parity_shards: usize,
) {
    // Collect all tesseras and their fragment metadata
    let missing: Vec<(ContentHash, usize, ContentHash)> = {
        let storage = storage.lock().unwrap();
        let tesseras = match storage.list_tesseras() {
            Ok(t) => t,
            Err(e) => {
                warn!("repair: failed to list tesseras: {e}");
                return;
            }
        };

        let mut missing = Vec::new();
        for tessera in &tesseras {
            for memory in &tessera.memories {
                let fragments = match storage.find_fragments(&memory.blob_hash) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                for meta in &fragments {
                    if !storage.has_blob(&meta.fragment_hash) {
                        missing.push((memory.blob_hash, meta.fragment_index, meta.fragment_hash));
                    }
                }
            }
        }
        missing
    };

    if missing.is_empty() {
        debug!("repair: all fragments healthy");
        return;
    }

    info!(
        "repair: found {} missing fragments, attempting recovery",
        missing.len()
    );
    let mut recovered = 0usize;

    for (blob_hash, frag_index, frag_hash) in &missing {
        // Look up providers for this fragment via DHT
        let target_id = NodeId::new(*frag_hash.as_bytes());
        let closest = {
            let dht_guard = dht.lock().unwrap();
            dht_guard.routing_table.find_closest(&target_id, K)
        };

        // Ask closest peers if they have the fragment
        for peer in &closest {
            let msg = DhtMessage::FetchBlob {
                sender: node_id,
                hash: *frag_hash,
            };

            let result = tokio::time::timeout(RPC_TIMEOUT, async {
                let conn = transport
                    .connect(peer.addr)
                    .await
                    .map_err(|e| NodeError::Network(e.to_string()))?;
                let (mut send, mut recv) = conn
                    .open_bi()
                    .await
                    .map_err(|e| NodeError::Network(e.to_string()))?;
                net::send_message(&mut send, &msg)
                    .await
                    .map_err(|e| NodeError::Network(e.to_string()))?;
                send.finish()
                    .map_err(|e| NodeError::Network(e.to_string()))?;

                let response = net::receive_message(&mut recv)
                    .await
                    .map_err(|e| NodeError::Network(e.to_string()))?;

                if let DhtMessage::FetchBlobResponse { found: true, .. } = response {
                    let data = net::receive_blob(&mut recv)
                        .await
                        .map_err(|e| NodeError::Network(e.to_string()))?;
                    Ok::<_, NodeError>(Some(data))
                } else {
                    Ok(None)
                }
            })
            .await;

            if let Ok(Ok(Some(data))) = result {
                // Verify the hash matches
                let actual_hash = crate::crypto::hash_bytes(&data);
                if actual_hash == *frag_hash {
                    let storage = storage.lock().unwrap();
                    if storage.store_blob_bytes(&data).is_ok() {
                        recovered += 1;
                        info!(
                            "repair: recovered fragment {frag_index} of blob {blob_hash} from {}",
                            peer.addr
                        );
                        break; // Got this fragment, move on
                    }
                } else {
                    warn!(
                        "repair: hash mismatch for fragment {frag_index} of blob {blob_hash} from {}",
                        peer.addr
                    );
                }
            }
        }
    }

    // If we have enough fragments for any blob, try to reconstruct
    if recovered > 0 {
        let storage = storage.lock().unwrap();
        let tesseras = match storage.list_tesseras() {
            Ok(t) => t,
            Err(_) => return,
        };

        for tessera in &tesseras {
            for memory in &tessera.memories {
                // Skip if original blob already exists
                if storage.has_blob(&memory.blob_hash) {
                    continue;
                }

                let fragments = match storage.find_fragments(&memory.blob_hash) {
                    Ok(f) => f,
                    Err(_) => continue,
                };

                // Collect available fragment data
                let mut available: Vec<Option<Vec<u8>>> = vec![None; data_shards + parity_shards];
                for meta in &fragments {
                    if meta.fragment_index < available.len() {
                        if let Ok(data) = storage.read_blob_bytes(&meta.fragment_hash) {
                            available[meta.fragment_index] = Some(data);
                        }
                    }
                }

                let present = available.iter().filter(|f| f.is_some()).count();
                if present >= data_shards {
                    // Build Fragment structs for decode
                    let decode_frags: Vec<replication::Fragment> = available
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, data)| {
                            data.map(|d| replication::Fragment { index: i, data: d })
                        })
                        .collect();

                    let original_size = fragments.first().map(|f| f.original_size).unwrap_or(0);
                    match replication::decode_fragments(
                        &decode_frags,
                        data_shards,
                        parity_shards,
                        original_size,
                    ) {
                        Ok(reconstructed) => {
                            info!(
                                "repair: reconstructed blob {} ({} bytes)",
                                memory.blob_hash,
                                reconstructed.len()
                            );
                        }
                        Err(e) => {
                            debug!(
                                "repair: reconstruction failed for {}: {e}",
                                memory.blob_hash
                            );
                        }
                    }
                }
            }
        }
    }

    info!(
        "repair: recovered {recovered}/{} missing fragments",
        missing.len()
    );
}

/// Refresh the routing table by doing FindNode for a random ID.
async fn refresh_routing_table(transport: &QuicTransport, dht: &Arc<Mutex<Dht>>, node_id: NodeId) {
    let peers: Vec<PeerInfo> = {
        let dht = dht.lock().unwrap();
        dht.routing_table.find_closest(&node_id, 3)
    };

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
    #[error("replication error: {0}")]
    Replication(String),
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
    fn add_tessera_creates_fragments() {
        let (tmp, node) = test_node();

        let test_file = tmp.path().join("photo.jpg");
        std::fs::write(&test_file, b"fake jpeg data for testing fragments").unwrap();

        let tessera = node
            .add_tessera(&[test_file], Some("Photo".into()), Visibility::Public)
            .unwrap();

        // Check that fragments were created for the blob
        let storage = node.storage.lock().unwrap();
        let fragments = storage
            .find_fragments(&tessera.memories[0].blob_hash)
            .unwrap();
        assert_eq!(
            fragments.len(),
            node.config.data_shards + node.config.parity_shards
        );
        assert_eq!(fragments[0].data_shards, 3);
        assert_eq!(fragments[0].parity_shards, 2);

        // Each fragment blob should exist on disk
        for meta in &fragments {
            assert!(storage.has_blob(&meta.fragment_hash));
        }
    }

    #[test]
    fn remove_tessera_cleans_fragments() {
        let (tmp, node) = test_node();

        let test_file = tmp.path().join("test.txt");
        std::fs::write(&test_file, b"to be removed with fragments").unwrap();

        let tessera = node
            .add_tessera(&[test_file], None, Visibility::Private)
            .unwrap();

        let blob_hash = tessera.memories[0].blob_hash;
        let fragment_hashes: Vec<ContentHash> = {
            let storage = node.storage.lock().unwrap();
            storage
                .find_fragments(&blob_hash)
                .unwrap()
                .iter()
                .map(|m| m.fragment_hash)
                .collect()
        };

        node.remove_tessera(&tessera.hash).unwrap();

        let storage = node.storage.lock().unwrap();
        assert!(storage.find_tessera(&tessera.hash).unwrap().is_none());
        assert!(!storage.has_blob(&blob_hash));
        assert!(storage.find_fragments(&blob_hash).unwrap().is_empty());
        for fh in &fragment_hashes {
            assert!(!storage.has_blob(fh));
        }
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

    #[test]
    fn fragments_can_reconstruct_blob() {
        let (tmp, node) = test_node();

        let test_file = tmp.path().join("data.bin");
        let original_data = b"this data should survive erasure coding";
        std::fs::write(&test_file, original_data).unwrap();

        let tessera = node
            .add_tessera(&[test_file], Some("Data".into()), Visibility::Public)
            .unwrap();

        let storage = node.storage.lock().unwrap();
        let blob_hash = tessera.memories[0].blob_hash;
        let fragment_metas = storage.find_fragments(&blob_hash).unwrap();

        // Read fragment blobs and reconstruct
        let fragments: Vec<replication::Fragment> = fragment_metas
            .iter()
            .map(|m| {
                let data = storage.read_blob_bytes(&m.fragment_hash).unwrap();
                replication::Fragment {
                    index: m.fragment_index,
                    data,
                }
            })
            .collect();

        let reconstructed = replication::decode_fragments(
            &fragments,
            fragment_metas[0].data_shards,
            fragment_metas[0].parity_shards,
            fragment_metas[0].original_size,
        )
        .unwrap();

        assert_eq!(reconstructed, original_data);
    }

    #[tokio::test]
    async fn start_and_accept_ping() {
        let (tmp, mut node_a) = test_node();
        let _ = tmp;

        let mut config_a = NodeConfig::default();
        config_a.listen = "127.0.0.1:0".parse().unwrap();
        node_a.config = config_a;

        let addr_a = node_a.start().await.unwrap();

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
        let (tmp_a, mut node_a) = test_node();
        let _ = tmp_a;
        node_a.config.listen = "127.0.0.1:0".parse().unwrap();
        let addr_a = node_a.start().await.unwrap();

        let tmp_b = tempfile::tempdir().unwrap();
        let data_dir_b = DataDir::open(tmp_b.path()).unwrap();
        let identity_b = Identity::generate();
        let mut config_b = NodeConfig::default();
        config_b.listen = "127.0.0.1:0".parse().unwrap();
        config_b.bootstrap = vec![addr_a.to_string()];
        let mut node_b = Node::new(data_dir_b, identity_b, config_b).unwrap();
        let _addr_b = node_b.start().await.unwrap();

        let discovered = node_b.bootstrap().await.unwrap();
        assert!(discovered > 0);

        assert!(
            node_b
                .dht
                .lock()
                .unwrap()
                .routing_table
                .contains(&node_a.node_id())
        );
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
        let (tmp_a, mut node_a) = test_node();
        node_a.config.listen = "127.0.0.1:0".parse().unwrap();
        let addr_a = node_a.start().await.unwrap();

        let tmp_b = tempfile::tempdir().unwrap();
        let data_dir_b = DataDir::open(tmp_b.path()).unwrap();
        let identity_b = Identity::generate();
        let mut config_b = NodeConfig::default();
        config_b.listen = "127.0.0.1:0".parse().unwrap();
        config_b.bootstrap = vec![addr_a.to_string()];
        let mut node_b = Node::new(data_dir_b, identity_b, config_b).unwrap();
        let _addr_b = node_b.start().await.unwrap();

        node_b.bootstrap().await.unwrap();

        let test_file = tmp_a.path().join("memory.txt");
        std::fs::write(&test_file, b"a precious memory").unwrap();
        let tessera = node_a
            .add_tessera(&[test_file], Some("My Memory".into()), Visibility::Public)
            .unwrap();

        let stored = node_a.announce_tessera(&tessera.hash).await.unwrap();
        assert!(stored > 0);

        let providers = node_b.find_providers(&tessera.hash).await.unwrap();
        assert!(!providers.is_empty());
        assert_eq!(providers[0].node_id, node_a.node_id());

        node_a.shutdown();
        node_b.shutdown();
    }

    #[tokio::test]
    async fn fetch_blob_from_peer() {
        // Node A stores a blob
        let (_tmp_a, mut node_a) = test_node();
        node_a.config.listen = "127.0.0.1:0".parse().unwrap();
        let addr_a = node_a.start().await.unwrap();

        // Store a blob on node A
        let blob_data = b"hello from node A";
        let blob_hash = {
            let storage = node_a.storage.lock().unwrap();
            storage.store_blob_bytes(blob_data).unwrap()
        };

        // Node B fetches the blob from A
        let tmp_b = tempfile::tempdir().unwrap();
        let data_dir_b = DataDir::open(tmp_b.path()).unwrap();
        let identity_b = Identity::generate();
        let mut config_b = NodeConfig::default();
        config_b.listen = "127.0.0.1:0".parse().unwrap();
        let mut node_b = Node::new(data_dir_b, identity_b, config_b).unwrap();
        let _addr_b = node_b.start().await.unwrap();

        let fetched = node_b.fetch_blob(addr_a, &blob_hash).await.unwrap();
        assert_eq!(fetched, Some(blob_data.to_vec()));

        // Try fetching a nonexistent blob
        let fake_hash = crypto::hash_bytes(b"does not exist");
        let not_found = node_b.fetch_blob(addr_a, &fake_hash).await.unwrap();
        assert_eq!(not_found, None);

        node_a.shutdown();
        node_b.shutdown();
    }

    #[test]
    fn check_fragments_reports_missing() {
        let (tmp, node) = test_node();

        let test_file = tmp.path().join("check.txt");
        std::fs::write(&test_file, b"data for fragment check").unwrap();

        let tessera = node
            .add_tessera(&[test_file], Some("Check".into()), Visibility::Public)
            .unwrap();

        // All fragments should be present initially
        let missing = node.check_fragments().unwrap();
        assert!(missing.is_empty(), "expected no missing fragments");

        // Delete one fragment blob to simulate loss
        let blob_hash = tessera.memories[0].blob_hash;
        let storage = node.storage.lock().unwrap();
        let fragments = storage.find_fragments(&blob_hash).unwrap();
        let lost_frag = &fragments[0];
        storage.delete_blob(&lost_frag.fragment_hash).unwrap();
        drop(storage);

        // Now check_fragments should report the missing one
        let missing = node.check_fragments().unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].0, blob_hash);
        assert_eq!(missing[0].1, lost_frag.fragment_index);
        assert_eq!(missing[0].2, lost_frag.fragment_hash);
    }

    #[tokio::test]
    async fn fetch_tessera_from_network() {
        // Set up two nodes with B bootstrapping from A
        let (tmp_a, mut node_a) = test_node();
        node_a.config.listen = "127.0.0.1:0".parse().unwrap();
        let addr_a = node_a.start().await.unwrap();

        let tmp_b = tempfile::tempdir().unwrap();
        let data_dir_b = DataDir::open(tmp_b.path()).unwrap();
        let identity_b = Identity::generate();
        let mut config_b = NodeConfig::default();
        config_b.listen = "127.0.0.1:0".parse().unwrap();
        config_b.bootstrap = vec![addr_a.to_string()];
        let mut node_b = Node::new(data_dir_b, identity_b, config_b).unwrap();
        let _addr_b = node_b.start().await.unwrap();
        node_b.bootstrap().await.unwrap();

        // Node A creates and announces a tessera (after bootstrap so A knows B)
        let test_file = tmp_a.path().join("memory.txt");
        std::fs::write(&test_file, b"a precious memory to fetch").unwrap();
        let tessera = node_a
            .add_tessera(&[test_file], Some("Fetchable".into()), Visibility::Public)
            .unwrap();

        let stored = node_a.announce_tessera(&tessera.hash).await.unwrap();
        assert!(stored > 0);

        // Node B should NOT have the tessera locally
        assert!(node_b.get_tessera(&tessera.hash).unwrap().is_none());

        // Fetch from network
        let fetched = node_b
            .fetch_tessera_from_network(&tessera.hash)
            .await
            .unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.hash, tessera.hash);
        assert_eq!(fetched.name, Some("Fetchable".into()));
        assert_eq!(fetched.memories.len(), 1);

        // Now it should be cached locally
        let local = node_b.get_tessera(&tessera.hash).unwrap();
        assert!(local.is_some());

        // The blob should also be available locally
        let storage_b = node_b.storage.lock().unwrap();
        assert!(storage_b.has_blob(&fetched.memories[0].blob_hash));

        node_a.shutdown();
        node_b.shutdown();
    }
}
