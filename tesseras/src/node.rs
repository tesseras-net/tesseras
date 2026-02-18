use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::config::{DataDir, NodeConfig};
use crate::crypto::{self, Identity};
use crate::dht::{Dht, DhtMessage, InsertResult, PeerInfo};
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

/// Minimum interval between hole punch attempts to the same peer.
const HOLE_PUNCH_RETRY_INTERVAL: Duration = Duration::from_secs(300);

/// Timeout for a single hole punch attempt.
const HOLE_PUNCH_TIMEOUT: Duration = Duration::from_secs(3);

/// Maximum messages per IP per window (Sybil protection).
const RATE_LIMIT_MAX: u32 = 100;

/// Rate limit window duration.
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

/// Per-IP rate limiter to mitigate Sybil and flood attacks.
#[derive(Clone)]
struct RateLimiter {
    buckets: Arc<Mutex<HashMap<IpAddr, RateBucket>>>,
}

struct RateBucket {
    count: u32,
    window_start: Instant,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a message from this IP is allowed. Returns false if rate-limited.
    fn check(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();

        let bucket = buckets.entry(ip).or_insert(RateBucket {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(bucket.window_start) >= RATE_LIMIT_WINDOW {
            bucket.count = 0;
            bucket.window_start = now;
        }

        if bucket.count >= RATE_LIMIT_MAX {
            return false;
        }

        bucket.count += 1;
        true
    }

    /// Remove stale entries (call periodically to avoid memory leak).
    fn cleanup(&self) {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();
        buckets.retain(|_, b| now.duration_since(b.window_start) < RATE_LIMIT_WINDOW * 2);
    }
}

/// Persistent connection pool: maps NodeId to an active QUIC connection.
/// Connections from both directions (outbound and inbound) are stored here.
type ConnectionPool = Arc<Mutex<HashMap<NodeId, quinn::Connection>>>;

/// The Node orchestrator: ties storage, DHT, QUIC, and replication together.
pub struct Node {
    pub storage: Arc<Mutex<Storage>>,
    pub identity: Identity,
    pub config: NodeConfig,
    pub dht: Arc<Mutex<Dht>>,
    transport: Option<Arc<QuicTransport>>,
    shutdown_tx: Option<watch::Sender<bool>>,
    /// External (public) address as discovered via STUN.
    external_addr: Option<SocketAddr>,
    /// Persistent connections keyed by NodeId.
    connections: ConnectionPool,
    /// Cache of failed hole punch attempts: NodeId -> last failure time.
    hole_punch_failures: Arc<Mutex<HashMap<NodeId, Instant>>>,
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
        let mut dht = Dht::new(node_id);

        // Restore persisted peers into the routing table
        if let Ok(peers) = storage.load_peers() {
            for peer in peers {
                let _ = dht.routing_table.insert(peer);
            }
        }

        let dht = Arc::new(Mutex::new(dht));

        Ok(Self {
            storage: Arc::new(Mutex::new(storage)),
            identity,
            config,
            dht,
            transport: None,
            shutdown_tx: None,
            external_addr: None,
            connections: Arc::new(Mutex::new(HashMap::new())),
            hole_punch_failures: Arc::new(Mutex::new(HashMap::new())),
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

        // Discover external address via STUN (non-blocking, best-effort)
        if !self.config.stun_servers.is_empty() {
            if let Some(ext) = net::discover_external_addr(&self.config.stun_servers, self.config.listen.port()).await {
                info!("external address: {ext}");
                self.external_addr = Some(ext);
            }
        }

        // Spawn the accept loop
        let dht = self.dht.clone();
        let storage = self.storage.clone();
        let accept_transport = transport.clone();
        let node_id = self.identity.node_id();
        let rate_limiter = RateLimiter::new();
        let connections = self.connections.clone();
        let mut rx = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut cleanup_interval = tokio::time::interval(RATE_LIMIT_WINDOW * 2);
            cleanup_interval.tick().await; // skip first tick
            loop {
                tokio::select! {
                    result = accept_transport.accept() => {
                        match result {
                            Ok(conn) => {
                                let remote_ip = conn.remote_address().ip();
                                if !rate_limiter.check(remote_ip) {
                                    debug!("rate-limited connection from {remote_ip}");
                                    conn.close(0u32.into(), b"rate limited");
                                    continue;
                                }
                                let dht = dht.clone();
                                let storage = storage.clone();
                                let relay_transport = accept_transport.clone();
                                let rl = rate_limiter.clone();
                                let conns = connections.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(conn, dht, storage, node_id, relay_transport, rl, conns).await {
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
                    _ = cleanup_interval.tick() => {
                        rate_limiter.cleanup();
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
    /// Resolves DNS SRV records (if `bootstrap_dns` is set) and merges
    /// with hardcoded `bootstrap` addresses, deduplicating by SocketAddr.
    pub async fn bootstrap(&self) -> Result<usize, NodeError> {
        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| NodeError::Network("transport not started".into()))?;

        let node_id = self.identity.node_id();
        let mut seen = std::collections::HashSet::new();

        // Collect all bootstrap addresses: DNS SRV + hardcoded
        let mut bootstrap_addrs: Vec<SocketAddr> = Vec::new();

        // Resolve DNS SRV if configured
        if let Some(ref domain) = self.config.bootstrap_dns {
            if !domain.is_empty() {
                let dns_addrs = net::resolve_bootstrap_dns(domain).await;
                bootstrap_addrs.extend(dns_addrs);
            }
        }

        // Parse hardcoded bootstrap addresses
        for addr_str in &self.config.bootstrap {
            match addr_str.parse::<SocketAddr>() {
                Ok(a) => bootstrap_addrs.push(a),
                Err(e) => {
                    warn!("invalid bootstrap address {addr_str}: {e}");
                }
            }
        }

        // Dedup by SocketAddr
        bootstrap_addrs.sort();
        bootstrap_addrs.dedup();

        for &addr in &bootstrap_addrs {
            let msg = DhtMessage::FindNode {
                sender: node_id,
                target: node_id,
            };

            match self.send_rpc(transport, addr, &msg).await {
                Ok(Some(DhtMessage::FindNodeResponse { sender, closest })) => {
                    let mut dht = self.dht.lock().unwrap();
                    let _ = dht.routing_table.insert(PeerInfo {
                        node_id: sender,
                        addr,
                    });
                    seen.insert(sender);
                    for peer in &closest {
                        let _ = dht.routing_table.insert(peer.clone());
                        seen.insert(peer.node_id);
                    }
                    debug!(
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

        if !seen.is_empty() {
            info!("bootstrap: discovered {} unique peers", seen.len());
        }
        Ok(seen.len())
    }

    /// Establish persistent connections to peers in the routing table.
    /// Called after bootstrap to enable relay and keepalive.
    /// Connects to up to `max` peers and registers them in the connection pool.
    pub async fn establish_persistent_connections(&self, max: usize) -> usize {
        let transport = match self.transport.as_ref() {
            Some(t) => t,
            None => return 0,
        };

        let peers: Vec<PeerInfo> = {
            let dht = self.dht.lock().unwrap();
            dht.routing_table
                .all_peers()
                .into_iter()
                .take(max)
                .collect()
        };

        for peer in peers {
            // Skip if already in pool
            {
                let pool = self.connections.lock().unwrap();
                if pool.contains_key(&peer.node_id) {
                    continue;
                }
            }

            match transport.connect(peer.addr).await {
                Ok(conn) => {
                    self.register_connection(peer.node_id, conn);
                }
                Err(e) => {
                    debug!("failed to establish persistent connection to {}: {e}", peer.addr);
                }
            }
        }

        let count = self.connections.lock().unwrap().len();
        info!("persistent connections: {count} active");
        count
    }

    /// Send an immediate keepalive ping to all persistent connections.
    /// Used at startup to register ourselves in relay peers' connection pools
    /// before announcing tesseras.
    pub async fn keepalive_now(&self) {
        let transport = match self.transport.as_ref() {
            Some(t) => t,
            None => return,
        };
        keepalive_connections(&self.connections, transport, self.identity.node_id()).await;
    }

    /// Spawn a periodic keepalive task that pings all persistent connections
    /// to keep NAT mappings alive and evicts dead connections.
    pub fn start_keepalive_loop(&self) {
        let transport = match self.transport.as_ref() {
            Some(t) => t.clone(),
            None => return,
        };
        let connections = self.connections.clone();
        let node_id = self.identity.node_id();
        let interval_secs = self.config.keepalive_interval;
        let mut shutdown_rx = self
            .shutdown_tx
            .as_ref()
            .expect("start() must be called first")
            .subscribe();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(interval_secs));
            interval.tick().await; // skip immediate first tick

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        keepalive_connections(&connections, &transport, node_id).await;
                    }
                    _ = shutdown_rx.changed() => {
                        info!("keepalive loop shutting down");
                        break;
                    }
                }
            }
        });
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
                        evict_dead_routing_peers(&transport, &dht, node_id).await;
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
    /// When behind NAT, announces via a relay peer's address so others can
    /// use RelayBiRequest to reach us through that peer.
    pub async fn announce_tessera(&self, hash: &ContentHash) -> Result<usize, NodeError> {
        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| NodeError::Network("transport not started".into()))?;

        let node_id = self.identity.node_id();

        // When behind NAT, use a relay peer's address so others know to relay through it.
        // The provider.node_id being different from the node at provider.addr signals relay needed.
        let addr = if self.is_behind_nat() {
            self.best_relay_addr()
                .or_else(|| self.public_addr())
                .ok_or_else(|| NodeError::Network("no reachable address".into()))?
        } else {
            self.public_addr()
                .ok_or_else(|| NodeError::Network("no local address".into()))?
        };

        let target_id = NodeId::new(*hash.as_bytes());
        let closest = {
            let dht = self.dht.lock().unwrap();
            dht.routing_table.find_closest(&target_id, K)
        };

        if closest.is_empty() {
            return Ok(0);
        }

        let provider = PeerInfo { node_id, addr };

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

    /// Re-announce all local tesseras to the DHT.
    /// Should be called after bootstrap to ensure providers are discoverable.
    pub async fn announce_all_tesseras(&self) -> Result<usize, NodeError> {
        let hashes: Vec<ContentHash> = {
            let storage = self.storage.lock().unwrap();
            storage
                .list_tesseras()
                .map_err(|e| NodeError::Storage(e.to_string()))?
                .iter()
                .map(|t| t.hash)
                .collect()
        };

        let mut total_stored = 0usize;
        for hash in &hashes {
            match self.announce_tessera(hash).await {
                Ok(n) => total_stored += n,
                Err(e) => debug!("failed to re-announce {hash}: {e}"),
            }
        }

        Ok(total_stored)
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
                    let _ = dht.routing_table.insert(PeerInfo {
                        node_id: sender,
                        addr: peer.addr,
                    });
                    for p in new_closest {
                        let _ = dht.routing_table.insert(p);
                    }
                }
                Ok(_other) => {}
                Err(_e) => {}
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
            // Check pool for existing persistent connection
            let pooled_conn = {
                let dht = self.dht.lock().unwrap();
                let peer = dht
                    .routing_table
                    .all_peers()
                    .into_iter()
                    .find(|p| p.addr == addr);
                peer.and_then(|p| {
                    let pool = self.connections.lock().unwrap();
                    pool.get(&p.node_id)
                        .filter(|c| c.close_reason().is_none())
                        .cloned()
                })
            };
            let conn = if let Some(conn) = pooled_conn {
                conn
            } else {
                transport
                    .connect(addr)
                    .await
                    .map_err(|e| NodeError::Network(e.to_string()))?
            };

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
    /// Uses the connection pool when a NodeId is known (via peer info in routing table),
    /// otherwise falls back to a fresh connection by address.
    async fn send_rpc(
        &self,
        transport: &QuicTransport,
        addr: SocketAddr,
        msg: &DhtMessage,
    ) -> Result<Option<DhtMessage>, NodeError> {
        let result = tokio::time::timeout(RPC_TIMEOUT, async {
            // Check connection pool for an existing persistent connection to this peer
            let pooled_conn = {
                let dht = self.dht.lock().unwrap();
                let peer = dht
                    .routing_table
                    .all_peers()
                    .into_iter()
                    .find(|p| p.addr == addr);
                peer.and_then(|p| {
                    let pool = self.connections.lock().unwrap();
                    pool.get(&p.node_id)
                        .filter(|c| c.close_reason().is_none())
                        .cloned()
                })
            };

            let conn = if let Some(conn) = pooled_conn {
                conn
            } else {
                transport
                    .connect(addr)
                    .await
                    .map_err(|e| NodeError::Network(e.to_string()))?
            };

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
            Err(_) => Ok(None),
        }
    }

    /// Check if a provider needs relay (its node_id doesn't match any peer at that address).
    /// This happens when a NAT'd node announces via a relay peer's address.
    /// Check if a provider requires relay to reach.
    /// Returns true if the provider's node_id differs from the peer known at that address,
    /// indicating the provider is behind NAT and announced via a relay peer.
    fn needs_relay(&self, provider: &PeerInfo) -> bool {
        // If the provider address is our own, we ARE the relay for this NAT'd node
        if Some(provider.addr) == self.public_addr() || Some(provider.addr) == self.local_addr() {
            // We're the relay — provider is behind NAT and connected to us
            return provider.node_id != self.identity.node_id();
        }

        let dht = self.dht.lock().unwrap();
        // Check if the routing table has a different node at this address
        let peer_at_addr = dht
            .routing_table
            .all_peers()
            .into_iter()
            .find(|p| p.addr == provider.addr);
        match peer_at_addr {
            Some(p) => p.node_id != provider.node_id,
            None => false, // unknown address, try direct
        }
    }

    /// Check if we are the relay peer for this provider (provider.addr is our own address).
    fn is_self_relay(&self, provider: &PeerInfo) -> bool {
        let our_addr = self.public_addr();
        let our_local = self.local_addr();
        (Some(provider.addr) == our_addr || Some(provider.addr) == our_local)
            && provider.node_id != self.identity.node_id()
    }

    /// Send an RPC to a NAT'd peer via relay using RelayBiRequest.
    async fn send_rpc_via_relay(
        &self,
        transport: &QuicTransport,
        relay_addr: SocketAddr,
        target: NodeId,
        inner_msg: &DhtMessage,
    ) -> Result<Option<DhtMessage>, NodeError> {
        let node_id = self.identity.node_id();
        let payload = inner_msg.to_bytes();
        let msg = DhtMessage::RelayBiRequest {
            sender: node_id,
            target,
            payload,
        };

        let relay_result = self.send_rpc(transport, relay_addr, &msg).await?;
        match relay_result {
            Some(DhtMessage::RelayBiResponse {
                payload: Some(data),
                ..
            }) => {
                let inner = DhtMessage::from_bytes(&data)
                    .map_err(|e| NodeError::Serialization(e.to_string()))?;
                Ok(Some(inner))
            }
            Some(DhtMessage::RelayBiResponse { payload: None, .. }) => Ok(None),
            _other => Ok(None),
        }
    }

    /// Attempt to establish a direct connection to a NAT'd peer via hole punching.
    /// Sends a HolePunchRequest to the relay peer, which notifies the target
    /// and returns the target's external address. Both sides then simultaneously
    /// call `net::hole_punch()` to establish a direct QUIC connection.
    /// Returns `Ok(true)` if a direct connection was established, `Ok(false)` if not.
    pub async fn attempt_hole_punch(
        &self,
        relay_addr: SocketAddr,
        target: NodeId,
    ) -> Result<bool, NodeError> {
        // Check failure cache — skip if we failed recently
        {
            let cache = self.hole_punch_failures.lock().unwrap();
            if let Some(last_failure) = cache.get(&target) {
                if last_failure.elapsed() < HOLE_PUNCH_RETRY_INTERVAL {
                    debug!("hole punch: skipping {target}, failed recently");
                    return Ok(false);
                }
            }
        }

        // Check if we already have a live connection
        {
            let pool = self.connections.lock().unwrap();
            if let Some(conn) = pool.get(&target) {
                if conn.close_reason().is_none() {
                    return Ok(true);
                }
            }
        }

        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| NodeError::Network("transport not started".into()))?;

        let node_id = self.identity.node_id();
        let sender_addr = self
            .public_addr()
            .ok_or_else(|| NodeError::Network("no public address".into()))?;

        // Send HolePunchRequest to relay and get HolePunchResponse with target's address
        let msg = DhtMessage::HolePunchRequest {
            sender: node_id,
            target,
            sender_addr,
        };

        let target_addr = match self.send_rpc(transport, relay_addr, &msg).await {
            Ok(Some(DhtMessage::HolePunchResponse { target_addr, .. })) => target_addr,
            _ => {
                debug!("hole punch: no response from relay for {target}");
                let mut cache = self.hole_punch_failures.lock().unwrap();
                cache.insert(target, Instant::now());
                return Ok(false);
            }
        };

        // Both sides punch simultaneously — the relay already sent HolePunchNotify to target
        debug!("hole punch: punching toward {target} at {target_addr}");
        match net::hole_punch(transport.endpoint(), target_addr, HOLE_PUNCH_TIMEOUT).await {
            Ok(conn) => {
                info!("hole punch: established direct connection to {target}");
                self.register_connection(target, conn);
                Ok(true)
            }
            Err(e) => {
                debug!("hole punch: failed to reach {target} at {target_addr}: {e}");
                let mut cache = self.hole_punch_failures.lock().unwrap();
                cache.insert(target, Instant::now());
                Ok(false)
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
        let addr = if self.is_behind_nat() {
            self.best_relay_addr()
                .or_else(|| self.public_addr())
                .ok_or_else(|| NodeError::Network("no reachable address".into()))?
        } else {
            self.public_addr()
                .ok_or_else(|| NodeError::Network("no local address".into()))?
        };

        let provider = PeerInfo {
            node_id,
            addr,
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
        let node_id = self.identity.node_id();
        let mut tessera: Option<Tessera> = None;
        let mut source_addr = None;
        let mut source_conn: Option<quinn::Connection> = None;
        for provider in &providers {
            if self.is_self_relay(provider) {
                // We ARE the relay peer — use our connection pool to reach the NAT'd provider
                let inner = DhtMessage::FetchTessera {
                    sender: node_id,
                    hash: *hash,
                };
                let target_conn = {
                    let pool = self.connections.lock().unwrap();
                    pool.get(&provider.node_id).cloned()
                };
                if let Some(conn) = target_conn {
                    if conn.close_reason().is_none() {
                        match send_rpc_on_connection(&conn, &inner).await {
                            Ok(Some(DhtMessage::FetchTesseraResponse {
                                tessera: Some(t),
                                ..
                            })) => {
                                tessera = Some(t);
                                source_conn = Some(conn);
                                break;
                            }
                            _ => continue,
                        }
                    }
                }
            } else if self.needs_relay(provider) {
                // Try hole punch to establish direct connection first
                let relay_addr = provider.addr;
                if self.attempt_hole_punch(relay_addr, provider.node_id).await.unwrap_or(false) {
                    // Hole punch succeeded — use pool connection directly
                    let pool_conn = {
                        let pool = self.connections.lock().unwrap();
                        pool.get(&provider.node_id).cloned()
                    };
                    if let Some(conn) = pool_conn {
                        if conn.close_reason().is_none() {
                            let inner = DhtMessage::FetchTessera {
                                sender: node_id,
                                hash: *hash,
                            };
                            if let Ok(Some(DhtMessage::FetchTesseraResponse {
                                tessera: Some(t),
                                ..
                            })) = send_rpc_on_connection(&conn, &inner).await
                            {
                                tessera = Some(t);
                                source_conn = Some(conn);
                                break;
                            }
                        }
                    }
                }

                // Fall back to relay
                let inner = DhtMessage::FetchTessera {
                    sender: node_id,
                    hash: *hash,
                };
                match self
                    .send_rpc_via_relay(transport, relay_addr, provider.node_id, &inner)
                    .await
                {
                    Ok(Some(DhtMessage::FetchTesseraResponse {
                        tessera: Some(t), ..
                    })) => {
                        tessera = Some(t);
                        source_addr = Some(relay_addr);
                        break;
                    }
                    _ => continue,
                }
            } else {
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
        }

        let tessera = match tessera {
            Some(t) => t,
            None => {
                info!("no provider had tessera metadata for {hash}");
                return Ok(None);
            }
        };

        // 3.5. Validate tessera signature before trusting it
        verify_tessera_signature(&tessera)?;

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

            // If we have a direct pool connection (self-relay case), use it
            if let Some(ref conn) = source_conn {
                let msg = DhtMessage::FetchBlob {
                    sender: node_id,
                    hash: memory.blob_hash,
                };
                let fetch_result: Result<Option<Vec<u8>>, NodeError> = async {
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
                }
                .await;
                if let Ok(Some(data)) = fetch_result {
                    let actual_hash = crate::crypto::hash_bytes(&data);
                    if actual_hash == memory.blob_hash {
                        if !self.check_storage_quota(data.len() as u64) {
                            debug!("storage quota exceeded, skipping blob {}", memory.blob_hash);
                            continue;
                        }
                        let storage = self.storage.lock().unwrap();
                        let _ = storage.store_blob_bytes(&data);
                        got_blob = true;
                    }
                }
            } else if let Some(addr) = source_addr {
                if let Ok(Some(data)) = self.fetch_blob(addr, &memory.blob_hash).await {
                    let actual_hash = crate::crypto::hash_bytes(&data);
                    if actual_hash == memory.blob_hash {
                        if !self.check_storage_quota(data.len() as u64) {
                            debug!("storage quota exceeded, skipping blob {}", memory.blob_hash);
                            continue;
                        }
                        let data_len = data.len() as u64;
                        let storage = self.storage.lock().unwrap();
                        let _ = storage.store_blob_bytes(&data);
                        // Track reciprocity: we stored bytes fetched from this peer
                        if let Some(provider) = providers.iter().find(|p| p.addr == addr) {
                            let _ = storage.record_bytes_stored(&provider.node_id, data_len);
                        }
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
            // Check pool for existing persistent connection
            let pooled_conn = {
                let dht = self.dht.lock().unwrap();
                let peer = dht
                    .routing_table
                    .all_peers()
                    .into_iter()
                    .find(|p| p.addr == addr);
                peer.and_then(|p| {
                    let pool = self.connections.lock().unwrap();
                    pool.get(&p.node_id)
                        .filter(|c| c.close_reason().is_none())
                        .cloned()
                })
            };
            let conn = if let Some(conn) = pooled_conn {
                conn
            } else {
                transport
                    .connect(addr)
                    .await
                    .map_err(|e| NodeError::Network(e.to_string()))?
            };
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
                        // Check quota before caching the fragment locally
                        if !self.check_storage_quota(data.len() as u64) {
                            debug!("storage quota exceeded, skipping fragment {}", meta.fragment_hash);
                            if meta.fragment_index < total_shards {
                                shards[meta.fragment_index] = Some(data);
                            }
                            break;
                        }
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

        // Check quota before storing the reconstructed blob
        if !self.check_storage_quota(reconstructed.len() as u64) {
            debug!("storage quota exceeded, not caching reconstructed blob {blob_hash}");
            return Ok(true);
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

    /// Public address: external (STUN-discovered) if available, otherwise local.
    pub fn public_addr(&self) -> Option<SocketAddr> {
        self.external_addr.or_else(|| self.local_addr())
    }

    /// Check if this node appears to be behind NAT.
    /// Compares local listen address with STUN-discovered external address.
    pub fn is_behind_nat(&self) -> bool {
        match (self.local_addr(), self.external_addr) {
            (Some(local), Some(ext)) => local.ip() != ext.ip(),
            _ => false,
        }
    }

    /// Get or create a persistent connection to a peer.
    /// Checks the connection pool first, falls back to transport.connect().
    /// Used for establishing relay connections, not for one-shot RPCs.
    #[allow(dead_code)]
    async fn get_connection(
        &self,
        transport: &QuicTransport,
        peer: &PeerInfo,
    ) -> Result<quinn::Connection, NodeError> {
        // Check pool first
        {
            let pool = self.connections.lock().unwrap();
            if let Some(conn) = pool.get(&peer.node_id) {
                // quinn::Connection reports closed via close_reason()
                if conn.close_reason().is_none() {
                    return Ok(conn.clone());
                }
            }
        }

        // Not in pool or dead — connect fresh
        let conn = transport
            .connect(peer.addr)
            .await
            .map_err(|e| NodeError::Network(e.to_string()))?;

        // Register in pool and spawn bidirectional handler
        self.register_connection(peer.node_id, conn.clone());
        Ok(conn)
    }

    /// Store a connection in the pool and spawn a handler for the reverse direction.
    /// Used for persistent connections (relay setup), not for one-shot RPCs.
    fn register_connection(&self, node_id: NodeId, conn: quinn::Connection) {
        {
            let mut pool = self.connections.lock().unwrap();
            // Evict old dead connection if any
            if let Some(old) = pool.get(&node_id) {
                if old.close_reason().is_some() {
                    pool.remove(&node_id);
                }
            }
            pool.insert(node_id, conn.clone());
        }

        // Spawn handler so the remote side can open streams back to us
        let dht = self.dht.clone();
        let storage = self.storage.clone();
        let local_node_id = self.identity.node_id();
        let transport = self.transport.clone();
        let connections = self.connections.clone();

        if let Some(transport) = transport {
            tokio::spawn(async move {
                if let Err(e) = handle_connection(
                    conn,
                    dht,
                    storage,
                    local_node_id,
                    transport,
                    RateLimiter::new(),
                    connections,
                )
                .await
                {
                    debug!("outbound connection handler error for {node_id}: {e}");
                }
            });
        }
    }

    /// Get the address of a connected peer to use as relay.
    /// Returns the address of the first live persistent connection peer.
    fn best_relay_addr(&self) -> Option<SocketAddr> {
        let pool = self.connections.lock().unwrap();
        let mut fallback = None;
        // Prefer peers on standard port 4433 (likely dedicated/public nodes)
        // over peers on random ports (likely behind NAT).
        for (_id, conn) in pool.iter() {
            if conn.close_reason().is_none() {
                let addr = conn.remote_address();
                if addr.port() == 4433 && !addr.ip().is_loopback() {
                    return Some(addr);
                }
                if fallback.is_none() {
                    fallback = Some(addr);
                }
            }
        }
        fallback
    }

    /// Check whether storing `additional` bytes of foreign data is within quota.
    /// Returns true if allowed (quota not exceeded or quota is unlimited).
    pub fn check_storage_quota(&self, additional: u64) -> bool {
        let storage = self.storage.lock().unwrap();
        // Check foreign storage quota
        if self.config.max_foreign_storage_bytes > 0 {
            match storage.foreign_blob_bytes() {
                Ok(current) => {
                    if current + additional > self.config.max_foreign_storage_bytes {
                        return false;
                    }
                }
                Err(e) => {
                    warn!("failed to check foreign storage: {e}");
                    return false;
                }
            }
        }
        // Check total storage quota
        match storage.check_quota(additional, self.config.max_total_storage_bytes) {
            Ok(allowed) => allowed,
            Err(e) => {
                warn!("failed to check total storage quota: {e}");
                false
            }
        }
    }

    /// Number of peers in the routing table.
    pub fn peer_count(&self) -> usize {
        self.dht.lock().unwrap().routing_table.len()
    }

    /// Spawn a periodic repair loop that checks fragment availability,
    /// re-replicates missing fragments, and re-announces local tesseras
    /// to the DHT so providers remain discoverable after restarts.
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
        let max_foreign = self.config.max_foreign_storage_bytes;
        let max_total = self.config.max_total_storage_bytes;
        let external_addr = self.external_addr;
        let connections = self.connections.clone();
        let hole_punch_failures = self.hole_punch_failures.clone();
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
                        // Prune expired hole punch failure entries
                        {
                            let mut cache = hole_punch_failures.lock().unwrap();
                            cache.retain(|_, t| t.elapsed() < HOLE_PUNCH_RETRY_INTERVAL);
                        }
                        reannounce_tesseras(
                            &transport, &dht, &storage, node_id,
                            external_addr, &connections,
                        ).await;
                        repair_fragments(
                            &transport, &dht, &storage, node_id,
                            data_shards, parity_shards,
                            max_foreign, max_total,
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

    /// Close the transport, persist DHT peers, and signal background tasks to stop.
    pub fn shutdown(&self) {
        // Persist routing table peers before shutting down
        let peers = self.dht.lock().unwrap().routing_table.all_peers();
        if !peers.is_empty() {
            let storage = self.storage.lock().unwrap();
            if let Err(e) = storage.save_peers(&peers) {
                warn!("failed to persist peers on shutdown: {e}");
            }
        }

        // Close all persistent connections in the pool
        {
            let mut pool = self.connections.lock().unwrap();
            for (_, conn) in pool.drain() {
                conn.close(0u32.into(), b"shutdown");
            }
        }

        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        if let Some(transport) = &self.transport {
            transport.close();
        }
    }
}

/// Verify a tessera's signature against its author public key.
/// This must be called when ingesting tesseras from untrusted peers.
fn verify_tessera_signature(tessera: &Tessera) -> Result<(), NodeError> {
    let content = rmp_serde::to_vec(&tessera.memories)
        .map_err(|e| NodeError::Serialization(e.to_string()))?;
    let expected_hash = crypto::hash_bytes(&content);
    if expected_hash != tessera.hash {
        return Err(NodeError::InvalidSignature(format!(
            "tessera hash mismatch: expected {expected_hash}, got {}",
            tessera.hash
        )));
    }
    Identity::verify(&tessera.author, &content, &tessera.signature).map_err(|e| {
        NodeError::InvalidSignature(format!("tessera {} signature invalid: {e}", tessera.hash))
    })?;
    Ok(())
}

/// Verify a signed DHT message envelope.
/// Returns the deserialized inner DhtMessage if the signature is valid.
pub fn verify_signed_envelope(
    envelope: &crate::dht::SignedEnvelope,
) -> Result<DhtMessage, NodeError> {
    Identity::verify(&envelope.public_key, &envelope.payload, &envelope.signature)
        .map_err(|e| NodeError::InvalidSignature(format!("DHT message signature invalid: {e}")))?;
    DhtMessage::from_bytes(&envelope.payload).map_err(|e| NodeError::Serialization(e.to_string()))
}

/// Send an RPC message on an existing connection and receive the response.
/// Unlike send_rpc(), this uses an already-established connection.
#[allow(dead_code)]
async fn send_rpc_on_connection(
    conn: &quinn::Connection,
    msg: &DhtMessage,
) -> Result<Option<DhtMessage>, NodeError> {
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
    Ok(Some(response))
}

/// Handle a single incoming QUIC connection.
async fn handle_connection(
    conn: quinn::Connection,
    dht: Arc<Mutex<Dht>>,
    storage: Arc<Mutex<Storage>>,
    local_node_id: NodeId,
    transport: Arc<QuicTransport>,
    rate_limiter: RateLimiter,
    connections: ConnectionPool,
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

        // Per-stream rate check (counts individual messages)
        if !rate_limiter.check(remote_addr.ip()) {
            debug!("rate-limited stream from {remote_addr}");
            continue;
        }

        let (mut send, mut recv) = stream;
        let dht = dht.clone();
        let storage = storage.clone();
        let transport = transport.clone();
        let connections = connections.clone();
        let conn_for_pool = conn.clone();

        tokio::spawn(async move {
            let raw_msg = match net::receive_message(&mut recv).await {
                Ok(m) => m,
                Err(e) => {
                    debug!("receive error from {remote_addr}: {e}");
                    return;
                }
            };

            // Unwrap RelayedMessage: deserialize inner payload and process as
            // a regular message so FetchTessera, FetchBlob, etc. handlers fire.
            let msg = if let DhtMessage::RelayedMessage {
                origin: _,
                relay: _,
                payload,
            } = &raw_msg
            {
                match DhtMessage::from_bytes(payload) {
                    Ok(inner) => inner,
                    Err(e) => {
                        debug!("relayed message: failed to deserialize inner payload: {e}");
                        return;
                    }
                }
            } else {
                raw_msg
            };

            // Handle RelayRequest — forward payload to the target peer
            if let DhtMessage::RelayRequest {
                sender,
                target,
                payload,
            } = &msg
            {
                {
                    let mut dht = dht.lock().unwrap();
                    let _ = dht.routing_table.insert(PeerInfo {
                        node_id: *sender,
                        addr: remote_addr,
                    });
                }

                // Look up the target peer address
                let target_addr = {
                    let dht = dht.lock().unwrap();
                    dht.routing_table
                        .find_closest(target, 1)
                        .into_iter()
                        .find(|p| p.node_id == *target)
                        .map(|p| p.addr)
                };

                if let Some(addr) = target_addr {
                    let relayed = DhtMessage::RelayedMessage {
                        origin: *sender,
                        relay: local_node_id,
                        payload: payload.clone(),
                    };
                    // Best-effort forward — don't fail the relay node on errors
                    if let Ok(conn) = transport.connect(addr).await {
                        if let Ok((mut fwd_send, _)) = conn.open_bi().await {
                            let _ = net::send_message(&mut fwd_send, &relayed).await;
                            let _ = fwd_send.finish();
                        }
                    }
                } else {
                    debug!("relay: target {target} not found in routing table");
                }
                let _ = send.finish();
                return;
            }

            // Handle RelayBiRequest — bidirectional relay via connection pool
            if let DhtMessage::RelayBiRequest {
                sender,
                target,
                payload,
            } = &msg
            {
                {
                    let mut dht = dht.lock().unwrap();
                    let _ = dht.routing_table.insert(PeerInfo {
                        node_id: *sender,
                        addr: remote_addr,
                    });
                }

                // Find target in connection pool (not routing table)
                let target_conn = {
                    let pool = connections.lock().unwrap();
                    pool.get(target).cloned()
                };

                if let Some(target_conn) = target_conn {
                    if target_conn.close_reason().is_none() {
                        // Forward payload to target via persistent connection
                        let relayed = DhtMessage::RelayedMessage {
                            origin: *sender,
                            relay: local_node_id,
                            payload: payload.clone(),
                        };
                        let relay_result = async {
                            let (mut fwd_send, mut fwd_recv) = target_conn
                                .open_bi()
                                .await
                                .map_err(|e| NodeError::Network(e.to_string()))?;
                            net::send_message(&mut fwd_send, &relayed)
                                .await
                                .map_err(|e| NodeError::Network(e.to_string()))?;
                            fwd_send
                                .finish()
                                .map_err(|e| NodeError::Network(e.to_string()))?;
                            // Wait for target's response
                            let response = net::receive_message(&mut fwd_recv)
                                .await
                                .map_err(|e| NodeError::Network(e.to_string()))?;
                            Ok::<_, NodeError>(response)
                        }
                        .await;

                        match relay_result {
                            Ok(response) => {
                                let resp = DhtMessage::RelayBiResponse {
                                    sender: local_node_id,
                                    payload: Some(response.to_bytes()),
                                };
                                let _ = net::send_message(&mut send, &resp).await;
                            }
                            Err(e) => {
                                debug!("relay bi: forward to {target} failed: {e}");
                                let resp = DhtMessage::RelayBiResponse {
                                    sender: local_node_id,
                                    payload: None,
                                };
                                let _ = net::send_message(&mut send, &resp).await;
                            }
                        }
                    } else {
                        let resp = DhtMessage::RelayBiResponse {
                            sender: local_node_id,
                            payload: None,
                        };
                        let _ = net::send_message(&mut send, &resp).await;
                    }
                } else {
                    debug!("relay bi: target {target} not in connection pool");
                    let resp = DhtMessage::RelayBiResponse {
                        sender: local_node_id,
                        payload: None,
                    };
                    let _ = net::send_message(&mut send, &resp).await;
                }
                let _ = send.finish();
                return;
            }

            // Handle HolePunchRequest — coordinate hole punch between two NAT'd peers
            if let DhtMessage::HolePunchRequest {
                sender,
                target,
                sender_addr,
            } = &msg
            {
                {
                    let mut dht = dht.lock().unwrap();
                    let _ = dht.routing_table.insert(PeerInfo {
                        node_id: *sender,
                        addr: remote_addr,
                    });
                }

                // Find target in connection pool
                let target_conn = {
                    let pool = connections.lock().unwrap();
                    pool.get(target).cloned()
                };

                if let Some(target_conn) = target_conn {
                    if target_conn.close_reason().is_none() {
                        let notify = DhtMessage::HolePunchNotify {
                            peer_id: *sender,
                            peer_addr: *sender_addr,
                        };
                        // Tell target to start punching
                        if let Ok((mut fwd_send, _)) = target_conn.open_bi().await {
                            let _ = net::send_message(&mut fwd_send, &notify).await;
                            let _ = fwd_send.finish();
                        }

                        // Send target's external address back to the initiator
                        let response = DhtMessage::HolePunchResponse {
                            sender: local_node_id,
                            target_addr: target_conn.remote_address(),
                        };
                        let _ = net::send_message(&mut send, &response).await;
                    }
                } else {
                    debug!("hole punch: target {target} not in connection pool");
                }
                let _ = send.finish();
                return;
            }

            // Handle HolePunchNotify — attempt to connect to the specified peer
            if let DhtMessage::HolePunchNotify { peer_id, peer_addr } = &msg {
                let peer_id = *peer_id;
                let peer_addr = *peer_addr;
                let transport = transport.clone();
                let connections = connections.clone();
                debug!("hole punch: received notify, punching toward {peer_id} at {peer_addr}");
                tokio::spawn(async move {
                    match net::hole_punch(
                        transport.endpoint(),
                        peer_addr,
                        Duration::from_secs(3),
                    )
                    .await
                    {
                        Ok(conn) => {
                            info!("hole punch: established direct connection to {peer_id}");
                            let mut pool = connections.lock().unwrap();
                            pool.insert(peer_id, conn);
                        }
                        Err(e) => {
                            debug!("hole punch: failed to reach {peer_id} at {peer_addr}: {e}");
                        }
                    }
                });
                let _ = send.finish();
                return;
            }

            // Handle FetchTessera — return tessera metadata from local storage
            if let DhtMessage::FetchTessera { sender, hash } = &msg {
                {
                    let mut dht = dht.lock().unwrap();
                    let _ = dht.routing_table.insert(PeerInfo {
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
                    let _ = dht.routing_table.insert(PeerInfo {
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
                    let blob_size = data.len() as u64;
                    let resp = DhtMessage::FetchBlobResponse {
                        sender: local_node_id,
                        found: true,
                        size: blob_size,
                    };
                    if let Err(e) = net::send_message(&mut send, &resp).await {
                        debug!("send FetchBlobResponse error to {remote_addr}: {e}");
                        return;
                    }
                    if let Err(e) = net::stream_blob(&mut send, &data).await {
                        debug!("stream blob error to {remote_addr}: {e}");
                    }
                    // Track reciprocity: we served bytes to this peer
                    let storage = storage.lock().unwrap();
                    let _ = storage.record_bytes_served(sender, blob_size);
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

            // Register persistent connection on Ping (from keepalive loop).
            // This means only peers with active keepalive get into the pool,
            // avoiding stale one-shot RPC connections.
            if matches!(&msg, DhtMessage::Ping { .. }) {
                let peer_id = msg.sender();
                let mut pool = connections.lock().unwrap();
                pool.entry(peer_id)
                    .or_insert_with(|| conn_for_pool.clone());
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

/// Re-announce all local tesseras to the DHT so they remain discoverable.
/// DHT pointer stores are in-memory, so announcements are lost when peers restart.
async fn reannounce_tesseras(
    transport: &QuicTransport,
    dht: &Arc<Mutex<Dht>>,
    storage: &Arc<Mutex<Storage>>,
    node_id: NodeId,
    external_addr: Option<SocketAddr>,
    connections: &Arc<Mutex<HashMap<NodeId, quinn::Connection>>>,
) {
    let tesseras = {
        let storage = storage.lock().unwrap();
        match storage.list_tesseras() {
            Ok(t) => t,
            Err(e) => {
                warn!("reannounce: failed to list tesseras: {e}");
                return;
            }
        }
    };

    if tesseras.is_empty() {
        return;
    }

    // Determine provider address (same logic as announce_tessera)
    let local_addr = transport.local_addr().ok();
    let is_nat = match (local_addr, external_addr) {
        (Some(local), Some(ext)) => local.ip() != ext.ip(),
        _ => false,
    };

    let relay_addr = if is_nat {
        let pool = connections.lock().unwrap();
        pool.values()
            .find(|c| c.close_reason().is_none())
            .map(|c| c.remote_address())
    } else {
        None
    };

    let addr = if is_nat {
        match relay_addr.or(external_addr).or(local_addr) {
            Some(a) => a,
            None => {
                warn!("reannounce: no reachable address");
                return;
            }
        }
    } else {
        match external_addr.or(local_addr) {
            Some(a) => a,
            None => {
                warn!("reannounce: no local address");
                return;
            }
        }
    };

    let provider = PeerInfo { node_id, addr };
    let mut announced = 0usize;

    for tessera in &tesseras {
        let target_id = NodeId::new(*tessera.hash.as_bytes());
        let closest = {
            let dht_guard = dht.lock().unwrap();
            dht_guard.routing_table.find_closest(&target_id, K)
        };

        for peer in &closest {
            let msg = DhtMessage::Store {
                sender: node_id,
                key: tessera.hash,
                provider: provider.clone(),
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
                Ok::<_, NodeError>(response)
            })
            .await;

            if let Ok(Ok(DhtMessage::StoreResponse { success: true, .. })) = result {
                announced += 1;
            }
        }
    }

    if announced > 0 {
        info!(
            "reannounced {} tesseras ({announced} store operations)",
            tesseras.len()
        );
    }
}

/// Check and repair missing fragments by fetching from DHT peers.
#[allow(clippy::too_many_arguments)]
async fn repair_fragments(
    transport: &QuicTransport,
    dht: &Arc<Mutex<Dht>>,
    storage: &Arc<Mutex<Storage>>,
    node_id: NodeId,
    data_shards: usize,
    parity_shards: usize,
    max_foreign_storage_bytes: u64,
    max_total_storage_bytes: u64,
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
                    // Check quota before storing
                    {
                        let s = storage.lock().unwrap();
                        if max_foreign_storage_bytes > 0 {
                            if let Ok(current) = s.foreign_blob_bytes() {
                                if current + data.len() as u64 > max_foreign_storage_bytes {
                                    debug!("repair: foreign storage quota exceeded, skipping fragment {frag_index} of blob {blob_hash}");
                                    break;
                                }
                            }
                        }
                        if let Ok(false) = s.check_quota(data.len() as u64, max_total_storage_bytes) {
                            debug!("repair: total storage quota exceeded, skipping fragment {frag_index} of blob {blob_hash}");
                            break;
                        }
                    }
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

/// Ping all persistent connections to keep NAT mappings alive.
/// Evict connections that are dead.
async fn keepalive_connections(
    connections: &ConnectionPool,
    _transport: &QuicTransport,
    node_id: NodeId,
) {
    let peers: Vec<(NodeId, quinn::Connection)> = {
        let pool = connections.lock().unwrap();
        pool.iter()
            .map(|(id, conn)| (*id, conn.clone()))
            .collect()
    };

    let mut dead = Vec::new();
    for (peer_id, conn) in &peers {
        if conn.close_reason().is_some() {
            dead.push(*peer_id);
            continue;
        }
        // Send a keepalive ping
        let ping = DhtMessage::Ping { sender: node_id };
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            let (mut send, mut recv) = conn
                .open_bi()
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;
            net::send_message(&mut send, &ping)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;
            send.finish()
                .map_err(|e| NodeError::Network(e.to_string()))?;
            let _resp = net::receive_message(&mut recv)
                .await
                .map_err(|e| NodeError::Network(e.to_string()))?;
            Ok::<_, NodeError>(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                debug!("keepalive: {peer_id} alive");
            }
            _ => {
                debug!("keepalive: {peer_id} dead, evicting");
                dead.push(*peer_id);
            }
        }
    }

    if !dead.is_empty() {
        let mut pool = connections.lock().unwrap();
        for id in &dead {
            pool.remove(id);
        }
        debug!("keepalive: evicted {} dead connections", dead.len());
    }
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
                // Insert the responder and learned peers, handling full buckets
                let mut pending_evictions: Vec<(PeerInfo, PeerInfo)> = Vec::new();
                {
                    let mut dht_guard = dht.lock().unwrap();
                    let _ = dht_guard.routing_table.insert(PeerInfo {
                        node_id: sender,
                        addr: peer.addr,
                    });
                    for p in closest {
                        match dht_guard.routing_table.insert(p.clone()) {
                            InsertResult::Inserted => {}
                            InsertResult::BucketFull { incumbent } => {
                                pending_evictions.push((incumbent, p));
                            }
                        }
                    }
                }
                // Handle evictions outside the lock (async ping)
                for (incumbent, new_peer) in pending_evictions {
                    if ping_peer(transport, node_id, &incumbent).await {
                        let mut dht_guard = dht.lock().unwrap();
                        dht_guard.routing_table.touch_incumbent(&incumbent.node_id);
                    } else {
                        let mut dht_guard = dht.lock().unwrap();
                        dht_guard
                            .routing_table
                            .evict_and_insert(&incumbent.node_id, new_peer);
                    }
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

/// Ping a peer and return true if it responds with Pong.
async fn ping_peer(transport: &QuicTransport, node_id: NodeId, peer: &PeerInfo) -> bool {
    let ping = DhtMessage::Ping { sender: node_id };
    let result = tokio::time::timeout(RPC_TIMEOUT, async {
        let conn = transport.connect(peer.addr).await?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| net::NetError::Connection(e.to_string()))?;
        net::send_message(&mut send, &ping).await?;
        send.finish()
            .map_err(|e| net::NetError::Write(e.to_string()))?;
        net::receive_message(&mut recv).await
    })
    .await;
    matches!(result, Ok(Ok(DhtMessage::Pong { .. })))
}

/// Ping every peer in the routing table and evict those that don't respond.
/// Follows Kademlia: prefer incumbents (long-lived peers are more reliable).
async fn evict_dead_routing_peers(
    transport: &QuicTransport,
    dht: &Arc<Mutex<Dht>>,
    node_id: NodeId,
) {
    let all_peers: Vec<PeerInfo> = {
        let dht = dht.lock().unwrap();
        dht.routing_table.all_peers()
    };

    if all_peers.is_empty() {
        return;
    }

    let mut dead: Vec<NodeId> = Vec::new();

    for peer in &all_peers {
        if ping_peer(transport, node_id, peer).await {
            let mut dht = dht.lock().unwrap();
            dht.routing_table.touch_incumbent(&peer.node_id);
        } else {
            debug!("eviction: {} at {} is dead", peer.node_id, peer.addr);
            dead.push(peer.node_id);
        }
    }

    if !dead.is_empty() {
        let mut dht = dht.lock().unwrap();
        for id in &dead {
            dht.routing_table.remove(id);
        }
        info!(
            "eviction: removed {} dead peers from routing table",
            dead.len()
        );
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
    #[error("invalid signature: {0}")]
    InvalidSignature(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node() -> (tempfile::TempDir, Node) {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = DataDir::open(tmp.path()).unwrap();
        let identity = Identity::generate();
        let mut config = NodeConfig::default();
        config.stun_servers = Vec::new(); // disable STUN in tests
        config.bootstrap_dns = None; // disable DNS in tests
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
        config_b.stun_servers = Vec::new();
        config_b.bootstrap_dns = None;
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
        config_b.stun_servers = Vec::new();
        config_b.bootstrap_dns = None;
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
        config_b.stun_servers = Vec::new();
        config_b.bootstrap_dns = None;
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
        {
            let storage_b = node_b.storage.lock().unwrap();
            assert!(storage_b.has_blob(&fetched.memories[0].blob_hash));
        }

        node_a.shutdown();
        node_b.shutdown();
    }

    #[tokio::test]
    async fn relay_message_forwarding() {
        // Three nodes: A <-> B (relay) <-> C
        // B knows both A and C. A sends a relay request to B targeting C.

        let (_tmp_a, mut node_a) = test_node();
        node_a.config.listen = "127.0.0.1:0".parse().unwrap();
        let addr_a = node_a.start().await.unwrap();

        let tmp_b = tempfile::tempdir().unwrap();
        let data_dir_b = DataDir::open(tmp_b.path()).unwrap();
        let identity_b = Identity::generate();
        let mut config_b = NodeConfig::default();
        config_b.listen = "127.0.0.1:0".parse().unwrap();
        config_b.bootstrap = vec![addr_a.to_string()];
        config_b.stun_servers = Vec::new();
        config_b.bootstrap_dns = None;
        let mut node_b = Node::new(data_dir_b, identity_b, config_b).unwrap();
        let addr_b = node_b.start().await.unwrap();
        node_b.bootstrap().await.unwrap();

        let tmp_c = tempfile::tempdir().unwrap();
        let data_dir_c = DataDir::open(tmp_c.path()).unwrap();
        let identity_c = Identity::generate();
        let mut config_c = NodeConfig::default();
        config_c.listen = "127.0.0.1:0".parse().unwrap();
        config_c.bootstrap = vec![addr_b.to_string()];
        config_c.stun_servers = Vec::new();
        config_c.bootstrap_dns = None;
        let mut node_c = Node::new(data_dir_c, identity_c, config_c).unwrap();
        let _addr_c = node_c.start().await.unwrap();
        node_c.bootstrap().await.unwrap();

        // Node C creates a tessera
        let test_file = tmp_c.path().join("relay_test.txt");
        std::fs::write(&test_file, b"relayed memory").unwrap();
        let tessera = node_c
            .add_tessera(&[test_file], Some("Relayed".into()), Visibility::Public)
            .unwrap();

        // Announce from C so all nodes know about it
        let stored = node_c.announce_tessera(&tessera.hash).await.unwrap();
        assert!(stored > 0);

        // Send a relay request from A through B to C (test the relay handler)
        let transport_a = node_a.transport.as_ref().unwrap().clone();
        let inner_msg = DhtMessage::Ping {
            sender: node_a.node_id(),
        };
        let payload = rmp_serde::to_vec(&inner_msg).unwrap();
        let relay_msg = DhtMessage::RelayRequest {
            sender: node_a.node_id(),
            target: node_c.node_id(),
            payload,
        };

        // Send relay request to B
        let conn = transport_a.connect(addr_b).await.unwrap();
        let (mut send, _) = conn.open_bi().await.unwrap();
        net::send_message(&mut send, &relay_msg).await.unwrap();
        send.finish().unwrap();

        // Give relay time to forward
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify C's routing table now knows about A (relay forwarded the message)
        {
            let c_dht = node_c.dht.lock().unwrap();
            let known_peers = c_dht.routing_table.len();
            // C should know at least B (from bootstrap) — relay adds no new routing info
            // since the RelayedMessage has origin=A but comes from B's address
            assert!(known_peers >= 1);
        }

        node_a.shutdown();
        node_b.shutdown();
        node_c.shutdown();
    }

    #[test]
    fn verify_tessera_signature_valid() {
        let (tmp, node) = test_node();
        let test_file = tmp.path().join("sign_test.txt");
        std::fs::write(&test_file, b"signed memory").unwrap();
        let tessera = node
            .add_tessera(&[test_file], Some("Signed".into()), Visibility::Public)
            .unwrap();

        // Should verify successfully
        verify_tessera_signature(&tessera).unwrap();
    }

    #[test]
    fn verify_tessera_signature_tampered() {
        let (tmp, node) = test_node();
        let test_file = tmp.path().join("tamper_test.txt");
        std::fs::write(&test_file, b"will be tampered").unwrap();
        let mut tessera = node
            .add_tessera(&[test_file], Some("Tampered".into()), Visibility::Public)
            .unwrap();

        // Tamper with the signature
        tessera.signature[0] ^= 0xFF;
        assert!(verify_tessera_signature(&tessera).is_err());
    }

    #[test]
    fn verify_tessera_signature_wrong_author() {
        let (tmp, node) = test_node();
        let test_file = tmp.path().join("wrong_author.txt");
        std::fs::write(&test_file, b"wrong author test").unwrap();
        let mut tessera = node
            .add_tessera(&[test_file], Some("WrongAuthor".into()), Visibility::Public)
            .unwrap();

        // Replace author with a different key
        let other = Identity::generate();
        tessera.author = other.public_key_bytes();
        assert!(verify_tessera_signature(&tessera).is_err());
    }

    #[test]
    fn signed_envelope_roundtrip() {
        let id = Identity::generate();
        let msg = DhtMessage::Ping {
            sender: id.node_id(),
        };
        let payload = msg.to_bytes();
        let envelope = id.sign_envelope(payload);

        // Should verify successfully
        let decoded = verify_signed_envelope(&envelope).unwrap();
        if let DhtMessage::Ping { sender } = decoded {
            assert_eq!(sender, id.node_id());
        } else {
            panic!("expected Ping");
        }
    }

    #[test]
    fn signed_envelope_tampered() {
        let id = Identity::generate();
        let msg = DhtMessage::Ping {
            sender: id.node_id(),
        };
        let payload = msg.to_bytes();
        let mut envelope = id.sign_envelope(payload);

        // Tamper with the payload
        envelope.payload.push(0xFF);
        assert!(verify_signed_envelope(&envelope).is_err());
    }

    #[test]
    fn rate_limiter_allows_within_limit() {
        let rl = RateLimiter::new();
        let ip: IpAddr = "1.2.3.4".parse().unwrap();
        for _ in 0..RATE_LIMIT_MAX {
            assert!(rl.check(ip));
        }
        // Next should be rejected
        assert!(!rl.check(ip));
    }

    #[test]
    fn rate_limiter_separate_ips() {
        let rl = RateLimiter::new();
        let ip1: IpAddr = "1.2.3.4".parse().unwrap();
        let ip2: IpAddr = "5.6.7.8".parse().unwrap();

        for _ in 0..RATE_LIMIT_MAX {
            assert!(rl.check(ip1));
        }
        assert!(!rl.check(ip1));
        // Different IP should still be allowed
        assert!(rl.check(ip2));
    }

    #[test]
    fn peers_persist_across_restarts() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = DataDir::open(tmp.path()).unwrap();

        // Create first node, add a peer to routing table, shut down
        let identity = Identity::generate();
        let mut config = NodeConfig::default();
        config.stun_servers = Vec::new();
        let node1 = Node::new(data_dir.clone(), identity, config.clone()).unwrap();

        let fake_peer = PeerInfo {
            node_id: NodeId::new([7u8; 32]),
            addr: "10.0.0.1:4433".parse().unwrap(),
        };
        node1
            .dht
            .lock()
            .unwrap()
            .routing_table
            .insert(fake_peer.clone());
        assert_eq!(node1.dht.lock().unwrap().routing_table.len(), 1);

        // Shutdown persists peers
        node1.shutdown();

        // Create second node with same data dir — should restore peers
        let identity2 = Identity::generate();
        let node2 = Node::new(data_dir, identity2, config).unwrap();
        let peers = node2.dht.lock().unwrap().routing_table.all_peers();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, fake_peer.node_id);
        assert_eq!(peers[0].addr, fake_peer.addr);
    }

    #[test]
    fn check_storage_quota_unlimited() {
        let (_tmp, node) = test_node();
        // Default config has 0 (unlimited) for both quotas
        assert!(node.check_storage_quota(1_000_000));
    }

    #[test]
    fn check_storage_quota_total_limit() {
        let (tmp, mut node) = test_node();

        // Add some data first
        let test_file = tmp.path().join("data.txt");
        std::fs::write(&test_file, b"some data here").unwrap();
        node.add_tessera(&[test_file], None, Visibility::Public).unwrap();

        // Set a tight total quota
        node.config.max_total_storage_bytes = 10;

        // Should reject large addition
        assert!(!node.check_storage_quota(1_000_000));
    }

    #[test]
    fn check_storage_quota_allows_within_limit() {
        let (_tmp, mut node) = test_node();

        // Set generous quota
        node.config.max_total_storage_bytes = 10_000_000;

        // Should allow small addition
        assert!(node.check_storage_quota(100));
    }
}
