use std::collections::HashMap;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::types::{ContentHash, NodeId, Tessera};

/// Kademlia replication factor.
const K: usize = 20;

/// Number of buckets (256 for 256-bit NodeId).
const NUM_BUCKETS: usize = 256;

/// A peer in the routing table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub addr: SocketAddr,
}

/// DHT wire messages (MessagePack-serialized).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DhtMessage {
    Ping {
        sender: NodeId,
    },
    Pong {
        sender: NodeId,
    },
    FindNode {
        sender: NodeId,
        target: NodeId,
    },
    FindNodeResponse {
        sender: NodeId,
        closest: Vec<PeerInfo>,
    },
    FindValue {
        sender: NodeId,
        key: ContentHash,
    },
    FindValueResponse {
        sender: NodeId,
        /// If we have pointers for this key, return them.
        pointers: Option<Vec<PeerInfo>>,
        /// Otherwise, return closest nodes.
        closest: Option<Vec<PeerInfo>>,
    },
    Store {
        sender: NodeId,
        key: ContentHash,
        provider: PeerInfo,
    },
    StoreResponse {
        sender: NodeId,
        success: bool,
    },
    Retract {
        sender: NodeId,
        key: ContentHash,
    },
    RetractResponse {
        sender: NodeId,
        success: bool,
    },
    /// Request a blob by its content hash.
    FetchBlob {
        sender: NodeId,
        hash: ContentHash,
    },
    /// Response to FetchBlob: if found is true, blob data follows on the same stream.
    FetchBlobResponse {
        sender: NodeId,
        found: bool,
        size: u64,
    },
    /// Request tessera metadata by content hash.
    FetchTessera {
        sender: NodeId,
        hash: ContentHash,
    },
    /// Response to FetchTessera.
    FetchTesseraResponse {
        sender: NodeId,
        tessera: Option<Tessera>,
    },
    /// Request a relay node to forward a message to a target peer.
    RelayRequest {
        sender: NodeId,
        target: NodeId,
        /// The inner message to relay, serialized as MessagePack bytes.
        payload: Vec<u8>,
    },
    /// A relayed message arriving at the target via a relay node.
    RelayedMessage {
        /// The original sender (not the relay).
        origin: NodeId,
        /// The relay node that forwarded this.
        relay: NodeId,
        /// The inner message, serialized as MessagePack bytes.
        payload: Vec<u8>,
    },
    /// Request to relay a full bidirectional RPC to a peer via persistent connection.
    RelayBiRequest {
        sender: NodeId,
        target: NodeId,
        /// The inner request message, serialized as MessagePack.
        payload: Vec<u8>,
    },
    /// Response from a relayed bidirectional RPC.
    RelayBiResponse {
        sender: NodeId,
        /// The inner response message from the target, serialized as MessagePack.
        /// None if the target was not reachable via the relay's connection pool.
        payload: Option<Vec<u8>>,
    },
    /// Request a relay to coordinate hole punch between two NAT'd peers.
    HolePunchRequest {
        sender: NodeId,
        target: NodeId,
        /// Sender's external address (from STUN).
        sender_addr: SocketAddr,
    },
    /// Relay tells target to start punching toward sender.
    HolePunchNotify {
        /// The peer to punch toward.
        peer_id: NodeId,
        peer_addr: SocketAddr,
    },
    /// Relay sends target's external address back to the initiator.
    HolePunchResponse {
        sender: NodeId,
        target_addr: SocketAddr,
    },
}

impl DhtMessage {
    /// Serialize to MessagePack bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        rmp_serde::to_vec(self).expect("serialization should not fail")
    }

    /// Deserialize from MessagePack bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }

    /// Extract the sender NodeId from any message variant.
    pub fn sender(&self) -> NodeId {
        match self {
            Self::Ping { sender }
            | Self::Pong { sender }
            | Self::FindNode { sender, .. }
            | Self::FindNodeResponse { sender, .. }
            | Self::FindValue { sender, .. }
            | Self::FindValueResponse { sender, .. }
            | Self::Store { sender, .. }
            | Self::StoreResponse { sender, .. }
            | Self::Retract { sender, .. }
            | Self::RetractResponse { sender, .. }
            | Self::FetchBlob { sender, .. }
            | Self::FetchBlobResponse { sender, .. }
            | Self::FetchTessera { sender, .. }
            | Self::FetchTesseraResponse { sender, .. }
            | Self::RelayRequest { sender, .. }
            | Self::RelayBiRequest { sender, .. }
            | Self::RelayBiResponse { sender, .. } => *sender,
            Self::RelayedMessage { origin, .. } => *origin,
            Self::HolePunchRequest { sender, .. }
            | Self::HolePunchResponse { sender, .. } => *sender,
            Self::HolePunchNotify { peer_id, .. } => *peer_id,
        }
    }
}

/// Authenticated envelope wrapping a DhtMessage with sender's Ed25519 signature.
/// The signature covers the serialized inner message bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedEnvelope {
    /// Serialized DhtMessage (MessagePack bytes).
    pub payload: Vec<u8>,
    /// Sender's Ed25519 public key (32 bytes).
    pub public_key: Vec<u8>,
    /// Ed25519 signature over `payload` (64 bytes).
    pub signature: Vec<u8>,
}

/// Result of attempting to insert a peer into a k-bucket.
#[derive(Debug)]
pub enum InsertResult {
    /// Peer was inserted (new) or updated (existing, moved to tail).
    Inserted,
    /// Bucket is full. Caller should ping the incumbent (head of bucket)
    /// and call `evict_and_insert()` if it's dead, or ignore the new peer.
    BucketFull { incumbent: PeerInfo },
}

/// A single k-bucket in the routing table.
#[derive(Debug, Clone, Default)]
struct KBucket {
    peers: Vec<PeerInfo>,
}

impl KBucket {
    fn insert(&mut self, peer: PeerInfo) -> InsertResult {
        // If already present, move to tail (most recently seen)
        if let Some(pos) = self.peers.iter().position(|p| p.node_id == peer.node_id) {
            self.peers.remove(pos);
            self.peers.push(peer);
            InsertResult::Inserted
        } else if self.peers.len() < K {
            self.peers.push(peer);
            InsertResult::Inserted
        } else {
            InsertResult::BucketFull {
                incumbent: self.peers[0].clone(), // head = least recently seen
            }
        }
    }

    /// Evict the least-recently-seen peer and insert a new one.
    /// Called after pinging the incumbent and finding it dead.
    fn evict_and_insert(&mut self, dead_id: &NodeId, new_peer: PeerInfo) {
        self.peers.retain(|p| &p.node_id != dead_id);
        if self.peers.len() < K {
            self.peers.push(new_peer);
        }
    }

    /// Move a peer to tail (it responded to ping, it's alive).
    fn touch_incumbent(&mut self, node_id: &NodeId) {
        if let Some(pos) = self.peers.iter().position(|p| &p.node_id == node_id) {
            let peer = self.peers.remove(pos);
            self.peers.push(peer);
        }
    }

    fn remove(&mut self, node_id: &NodeId) {
        self.peers.retain(|p| &p.node_id != node_id);
    }

    fn contains(&self, node_id: &NodeId) -> bool {
        self.peers.iter().any(|p| &p.node_id == node_id)
    }
}

/// Kademlia routing table with 256 k-buckets.
#[derive(Debug)]
pub struct RoutingTable {
    local_id: NodeId,
    buckets: Vec<KBucket>,
}

impl RoutingTable {
    pub fn new(local_id: NodeId) -> Self {
        Self {
            local_id,
            buckets: (0..NUM_BUCKETS).map(|_| KBucket::default()).collect(),
        }
    }

    /// Determine which bucket a node belongs in based on XOR distance.
    fn bucket_index(&self, node_id: &NodeId) -> usize {
        let distance = self.local_id.distance(node_id);
        // Find the index of the highest bit set in the distance
        for (i, &byte) in distance.iter().enumerate() {
            if byte != 0 {
                let leading = byte.leading_zeros() as usize;
                return 255 - (i * 8 + leading);
            }
        }
        0 // distance is zero (same node), put in bucket 0
    }

    /// Insert or update a peer in the routing table.
    pub fn insert(&mut self, peer: PeerInfo) -> InsertResult {
        if peer.node_id == self.local_id {
            return InsertResult::Inserted; // no-op
        }
        let idx = self.bucket_index(&peer.node_id);
        self.buckets[idx].insert(peer)
    }

    /// Evict a dead peer and insert a new one in its bucket.
    pub fn evict_and_insert(&mut self, dead_id: &NodeId, new_peer: PeerInfo) {
        let idx = self.bucket_index(dead_id);
        self.buckets[idx].evict_and_insert(dead_id, new_peer);
    }

    /// Move a peer to tail of its bucket (mark as most recently seen).
    pub fn touch_incumbent(&mut self, node_id: &NodeId) {
        let idx = self.bucket_index(node_id);
        self.buckets[idx].touch_incumbent(node_id);
    }

    /// Remove a peer from the routing table.
    pub fn remove(&mut self, node_id: &NodeId) {
        if node_id == &self.local_id {
            return;
        }
        let idx = self.bucket_index(node_id);
        self.buckets[idx].remove(node_id);
    }

    /// Check if a peer is in the routing table.
    pub fn contains(&self, node_id: &NodeId) -> bool {
        if node_id == &self.local_id {
            return false;
        }
        let idx = self.bucket_index(node_id);
        self.buckets[idx].contains(node_id)
    }

    /// Find the k closest nodes to a target.
    pub fn find_closest(&self, target: &NodeId, count: usize) -> Vec<PeerInfo> {
        let mut all_peers: Vec<&PeerInfo> =
            self.buckets.iter().flat_map(|b| b.peers.iter()).collect();

        all_peers.sort_by(|a, b| {
            let dist_a = target.distance(&a.node_id);
            let dist_b = target.distance(&b.node_id);
            dist_a.cmp(&dist_b)
        });

        all_peers.into_iter().take(count).cloned().collect()
    }

    /// Get total number of peers in the routing table.
    pub fn len(&self) -> usize {
        self.buckets.iter().map(|b| b.peers.len()).sum()
    }

    /// Check if the routing table is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all peers in the routing table.
    pub fn all_peers(&self) -> Vec<PeerInfo> {
        self.buckets
            .iter()
            .flat_map(|b| b.peers.iter().cloned())
            .collect()
    }
}

/// DHT pointer store: maps content hashes to provider peers.
#[derive(Debug, Default)]
pub struct PointerStore {
    store: HashMap<ContentHash, Vec<PeerInfo>>,
}

impl PointerStore {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    /// Store a provider pointer for a content hash.
    pub fn store(&mut self, key: ContentHash, provider: PeerInfo) {
        let providers = self.store.entry(key).or_default();
        // Update if already present
        if let Some(pos) = providers.iter().position(|p| p.node_id == provider.node_id) {
            providers[pos] = provider;
        } else {
            providers.push(provider);
        }
    }

    /// Find providers for a content hash.
    pub fn find(&self, key: &ContentHash) -> Option<&Vec<PeerInfo>> {
        self.store.get(key)
    }

    /// Remove all pointers for a content hash.
    pub fn retract(&mut self, key: &ContentHash) {
        self.store.remove(key);
    }

    /// Number of stored keys.
    pub fn key_count(&self) -> usize {
        self.store.len()
    }

    /// Remove a specific provider for a content hash.
    pub fn retract_provider(&mut self, key: &ContentHash, node_id: &NodeId) {
        if let Some(providers) = self.store.get_mut(key) {
            providers.retain(|p| &p.node_id != node_id);
            if providers.is_empty() {
                self.store.remove(key);
            }
        }
    }
}

/// The DHT node: routing table + pointer store + message handling.
pub struct Dht {
    pub local_id: NodeId,
    pub routing_table: RoutingTable,
    pub pointer_store: PointerStore,
}

impl Dht {
    pub fn new(local_id: NodeId) -> Self {
        Self {
            local_id,
            routing_table: RoutingTable::new(local_id),
            pointer_store: PointerStore::new(),
        }
    }

    /// Handle an incoming DHT message and produce a response.
    pub fn handle_message(&mut self, msg: DhtMessage, from_addr: SocketAddr) -> Option<DhtMessage> {
        match msg {
            DhtMessage::Ping { sender } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                Some(DhtMessage::Pong {
                    sender: self.local_id,
                })
            }
            DhtMessage::Pong { sender } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                None
            }
            DhtMessage::FindNode { sender, target } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                let closest = self.routing_table.find_closest(&target, K);
                Some(DhtMessage::FindNodeResponse {
                    sender: self.local_id,
                    closest,
                })
            }
            DhtMessage::FindNodeResponse { sender, closest } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                for peer in closest {
                    let _ = self.routing_table.insert(peer);
                }
                None
            }
            DhtMessage::FindValue { sender, key } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                if let Some(providers) = self.pointer_store.find(&key) {
                    Some(DhtMessage::FindValueResponse {
                        sender: self.local_id,
                        pointers: Some(providers.clone()),
                        closest: None,
                    })
                } else {
                    let node_id = NodeId::new(*key.as_bytes());
                    let closest = self.routing_table.find_closest(&node_id, K);
                    Some(DhtMessage::FindValueResponse {
                        sender: self.local_id,
                        pointers: None,
                        closest: Some(closest),
                    })
                }
            }
            DhtMessage::FindValueResponse {
                sender, closest, ..
            } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                if let Some(peers) = closest {
                    for peer in peers {
                        let _ = self.routing_table.insert(peer);
                    }
                }
                None
            }
            DhtMessage::Store {
                sender,
                key,
                provider,
            } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                self.pointer_store.store(key, provider);
                Some(DhtMessage::StoreResponse {
                    sender: self.local_id,
                    success: true,
                })
            }
            DhtMessage::StoreResponse { sender, .. } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                None
            }
            DhtMessage::Retract { sender, key } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                self.pointer_store.retract_provider(&key, &sender);
                Some(DhtMessage::RetractResponse {
                    sender: self.local_id,
                    success: true,
                })
            }
            DhtMessage::RetractResponse { sender, .. } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                None
            }
            // These are handled at the connection level, not by the DHT.
            DhtMessage::FetchBlob { sender, .. }
            | DhtMessage::FetchBlobResponse { sender, .. }
            | DhtMessage::FetchTessera { sender, .. }
            | DhtMessage::FetchTesseraResponse { sender, .. }
            | DhtMessage::RelayRequest { sender, .. }
            | DhtMessage::RelayBiRequest { sender, .. }
            | DhtMessage::RelayBiResponse { sender, .. }
            | DhtMessage::RelayedMessage { origin: sender, .. } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                None
            }
            // Hole punch messages are handled at the connection level.
            DhtMessage::HolePunchRequest { sender, .. }
            | DhtMessage::HolePunchResponse { sender, .. }
            | DhtMessage::HolePunchNotify { peer_id: sender, .. } => {
                let _ = self.routing_table.insert(PeerInfo {
                    node_id: sender,
                    addr: from_addr,
                });
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node_id(byte: u8) -> NodeId {
        NodeId::new([byte; 32])
    }

    fn make_peer(byte: u8, port: u16) -> PeerInfo {
        PeerInfo {
            node_id: make_node_id(byte),
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
        }
    }

    fn make_addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    #[test]
    fn routing_table_insert_and_find() {
        let local = make_node_id(0x00);
        let mut rt = RoutingTable::new(local);

        rt.insert(make_peer(0x01, 1001));
        rt.insert(make_peer(0x02, 1002));
        rt.insert(make_peer(0x03, 1003));

        assert_eq!(rt.len(), 3);
        assert!(rt.contains(&make_node_id(0x01)));
        assert!(!rt.contains(&make_node_id(0xFF)));
    }

    #[test]
    fn routing_table_does_not_insert_self() {
        let local = make_node_id(0x42);
        let mut rt = RoutingTable::new(local);

        rt.insert(PeerInfo {
            node_id: local,
            addr: make_addr(1000),
        });
        assert_eq!(rt.len(), 0);
    }

    #[test]
    fn routing_table_find_closest() {
        let local = make_node_id(0x00);
        let mut rt = RoutingTable::new(local);

        for i in 1..=10u8 {
            rt.insert(make_peer(i, 1000 + i as u16));
        }

        let target = make_node_id(0x01);
        let closest = rt.find_closest(&target, 3);
        assert_eq!(closest.len(), 3);
        // The closest to 0x01 should be 0x01 itself (distance 0)
        assert_eq!(closest[0].node_id, make_node_id(0x01));
    }

    #[test]
    fn routing_table_remove() {
        let local = make_node_id(0x00);
        let mut rt = RoutingTable::new(local);

        rt.insert(make_peer(0x01, 1001));
        assert!(rt.contains(&make_node_id(0x01)));

        rt.remove(&make_node_id(0x01));
        assert!(!rt.contains(&make_node_id(0x01)));
        assert_eq!(rt.len(), 0);
    }

    #[test]
    fn routing_table_bucket_boundaries() {
        let local = make_node_id(0x00);
        let mut rt = RoutingTable::new(local);

        // Node with distance that differs only in the last bit
        let mut bytes = [0u8; 32];
        bytes[31] = 0x01;
        rt.insert(PeerInfo {
            node_id: NodeId::new(bytes),
            addr: make_addr(2000),
        });

        // Node with distance that differs in the first byte
        bytes = [0u8; 32];
        bytes[0] = 0x80;
        rt.insert(PeerInfo {
            node_id: NodeId::new(bytes),
            addr: make_addr(2001),
        });

        assert_eq!(rt.len(), 2);
    }

    #[test]
    fn kbucket_full_returns_incumbent() {
        let local = make_node_id(0x00);
        let mut rt = RoutingTable::new(local);

        // Fill a bucket with K peers (all in the same bucket — high bit differs)
        for i in 1..=K as u8 {
            let mut bytes = [0u8; 32];
            bytes[0] = 0x80; // same bucket (highest bit set)
            bytes[31] = i; // unique peers
            let result = rt.insert(PeerInfo {
                node_id: NodeId::new(bytes),
                addr: make_addr(3000 + i as u16),
            });
            assert!(matches!(result, InsertResult::Inserted));
        }

        // Next insert in the same bucket should return BucketFull
        let mut bytes = [0u8; 32];
        bytes[0] = 0x80;
        bytes[31] = 0xFF;
        let result = rt.insert(PeerInfo {
            node_id: NodeId::new(bytes),
            addr: make_addr(4000),
        });
        assert!(matches!(result, InsertResult::BucketFull { .. }));

        // The incumbent should be the first peer inserted (head)
        if let InsertResult::BucketFull { incumbent } = result {
            let mut expected = [0u8; 32];
            expected[0] = 0x80;
            expected[31] = 1;
            assert_eq!(incumbent.node_id, NodeId::new(expected));
        }
    }

    #[test]
    fn evict_and_insert_replaces_dead_peer() {
        let local = make_node_id(0x00);
        let mut rt = RoutingTable::new(local);

        // Fill a bucket
        for i in 1..=K as u8 {
            let mut bytes = [0u8; 32];
            bytes[0] = 0x80;
            bytes[31] = i;
            rt.insert(PeerInfo {
                node_id: NodeId::new(bytes),
                addr: make_addr(3000 + i as u16),
            });
        }

        // Evict first peer and insert new one
        let mut dead_bytes = [0u8; 32];
        dead_bytes[0] = 0x80;
        dead_bytes[31] = 1;
        let dead_id = NodeId::new(dead_bytes);

        let mut new_bytes = [0u8; 32];
        new_bytes[0] = 0x80;
        new_bytes[31] = 0xFF;
        let new_peer = PeerInfo {
            node_id: NodeId::new(new_bytes),
            addr: make_addr(4000),
        };

        rt.evict_and_insert(&dead_id, new_peer);

        assert!(!rt.contains(&dead_id));
        assert!(rt.contains(&NodeId::new(new_bytes)));
        assert_eq!(rt.len(), K);
    }

    #[test]
    fn xor_distance_correctness() {
        let a = NodeId::new([0xFF; 32]);
        let b = NodeId::new([0x00; 32]);
        let dist = a.distance(&b);
        assert_eq!(dist, [0xFF; 32]);

        let dist_self = a.distance(&a);
        assert_eq!(dist_self, [0x00; 32]);
    }

    #[test]
    fn xor_distance_ordering() {
        let origin = make_node_id(0x00);
        let close = make_node_id(0x01);
        let far = make_node_id(0xFF);

        let dist_close = origin.distance(&close);
        let dist_far = origin.distance(&far);
        assert!(dist_close < dist_far);
    }

    #[test]
    fn message_serialization_roundtrip() {
        let messages = vec![
            DhtMessage::Ping {
                sender: make_node_id(0x01),
            },
            DhtMessage::Pong {
                sender: make_node_id(0x02),
            },
            DhtMessage::FindNode {
                sender: make_node_id(0x01),
                target: make_node_id(0x42),
            },
            DhtMessage::Store {
                sender: make_node_id(0x01),
                key: ContentHash::new([0xAB; 32]),
                provider: make_peer(0x01, 1001),
            },
            DhtMessage::Retract {
                sender: make_node_id(0x01),
                key: ContentHash::new([0xAB; 32]),
            },
        ];

        for msg in messages {
            let bytes = msg.to_bytes();
            let decoded = DhtMessage::from_bytes(&bytes).unwrap();
            // Re-serialize and compare bytes
            assert_eq!(bytes, decoded.to_bytes());
        }
    }

    #[test]
    fn pointer_store_and_find() {
        let mut store = PointerStore::new();
        let key = ContentHash::new([0xAB; 32]);
        let provider = make_peer(0x01, 1001);

        store.store(key, provider.clone());
        let found = store.find(&key).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].node_id, make_node_id(0x01));
    }

    #[test]
    fn pointer_store_multiple_providers() {
        let mut store = PointerStore::new();
        let key = ContentHash::new([0xAB; 32]);

        store.store(key, make_peer(0x01, 1001));
        store.store(key, make_peer(0x02, 1002));

        let found = store.find(&key).unwrap();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn pointer_store_retract() {
        let mut store = PointerStore::new();
        let key = ContentHash::new([0xAB; 32]);
        store.store(key, make_peer(0x01, 1001));
        store.retract(&key);
        assert!(store.find(&key).is_none());
    }

    #[test]
    fn pointer_store_retract_provider() {
        let mut store = PointerStore::new();
        let key = ContentHash::new([0xAB; 32]);
        store.store(key, make_peer(0x01, 1001));
        store.store(key, make_peer(0x02, 1002));

        store.retract_provider(&key, &make_node_id(0x01));
        let found = store.find(&key).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].node_id, make_node_id(0x02));
    }

    #[test]
    fn dht_ping_pong() {
        let mut dht = Dht::new(make_node_id(0x00));
        let addr = make_addr(5000);

        let response = dht.handle_message(
            DhtMessage::Ping {
                sender: make_node_id(0x01),
            },
            addr,
        );

        assert!(response.is_some());
        if let Some(DhtMessage::Pong { sender }) = response {
            assert_eq!(sender, make_node_id(0x00));
        } else {
            panic!("expected Pong");
        }

        // Sender should be in routing table
        assert!(dht.routing_table.contains(&make_node_id(0x01)));
    }

    #[test]
    fn dht_store_and_find_value() {
        let mut dht = Dht::new(make_node_id(0x00));
        let addr = make_addr(5000);
        let key = ContentHash::new([0xAB; 32]);

        // Store
        let response = dht.handle_message(
            DhtMessage::Store {
                sender: make_node_id(0x01),
                key,
                provider: make_peer(0x01, 1001),
            },
            addr,
        );
        assert!(matches!(
            response,
            Some(DhtMessage::StoreResponse { success: true, .. })
        ));

        // Find value
        let response = dht.handle_message(
            DhtMessage::FindValue {
                sender: make_node_id(0x02),
                key,
            },
            make_addr(5001),
        );

        if let Some(DhtMessage::FindValueResponse {
            pointers: Some(providers),
            ..
        }) = response
        {
            assert_eq!(providers.len(), 1);
            assert_eq!(providers[0].node_id, make_node_id(0x01));
        } else {
            panic!("expected FindValueResponse with pointers");
        }
    }

    #[test]
    fn dht_retract() {
        let mut dht = Dht::new(make_node_id(0x00));
        let addr = make_addr(5000);
        let key = ContentHash::new([0xAB; 32]);

        // Store
        dht.handle_message(
            DhtMessage::Store {
                sender: make_node_id(0x01),
                key,
                provider: make_peer(0x01, 1001),
            },
            addr,
        );

        // Retract
        let response = dht.handle_message(
            DhtMessage::Retract {
                sender: make_node_id(0x01),
                key,
            },
            addr,
        );
        assert!(matches!(
            response,
            Some(DhtMessage::RetractResponse { success: true, .. })
        ));

        // Find should return closest nodes, not pointers
        let response = dht.handle_message(
            DhtMessage::FindValue {
                sender: make_node_id(0x02),
                key,
            },
            make_addr(5001),
        );

        if let Some(DhtMessage::FindValueResponse {
            pointers, closest, ..
        }) = response
        {
            assert!(pointers.is_none());
            assert!(closest.is_some());
        } else {
            panic!("expected FindValueResponse without pointers");
        }
    }
}
