use std::collections::VecDeque;

use tesseras_core::{NodeId, NodeInfo};

use crate::distance;

/// Kademlia routing table: 160 k-buckets, each holding up to K entries.
pub const K: usize = 20;
pub const NUM_BUCKETS: usize = 160;

/// A single k-bucket: ordered deque, front = least-recently-seen.
#[derive(Debug)]
struct KBucket {
    entries: VecDeque<NodeInfo>,
}

impl KBucket {
    fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(K),
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_full(&self) -> bool {
        self.entries.len() >= K
    }

    /// Update a node in this bucket. Returns InsertResult.
    fn update(&mut self, node: NodeInfo) -> InsertResult {
        // If node already exists, move it to the back (most-recently-seen)
        if let Some(pos) = self
            .entries
            .iter()
            .position(|n| n.identity.node_id == node.identity.node_id)
        {
            self.entries.remove(pos);
            self.entries.push_back(node);
            return InsertResult::Updated;
        }

        // If bucket not full, add to the back
        if !self.is_full() {
            self.entries.push_back(node);
            return InsertResult::Inserted;
        }

        // Bucket is full: return the least-recently-seen node for ping check
        InsertResult::BucketFull {
            least_recent: self.entries.front().cloned().unwrap(),
            pending: Box::new(node),
        }
    }

    /// Replace the least-recently-seen node (after failed ping) with the pending node.
    fn evict_and_replace(&mut self, old_id: &NodeId, new_node: NodeInfo) -> bool {
        if let Some(pos) = self
            .entries
            .iter()
            .position(|n| n.identity.node_id == *old_id)
        {
            self.entries.remove(pos);
            self.entries.push_back(new_node);
            true
        } else {
            false
        }
    }

    /// The least-recently-seen responded to ping: keep it, move to back, discard pending.
    fn refresh_least_recent(&mut self, id: &NodeId) {
        if let Some(pos) = self.entries.iter().position(|n| n.identity.node_id == *id) {
            let node = self.entries.remove(pos).unwrap();
            self.entries.push_back(node);
        }
    }

    /// Remove a node by ID.
    fn remove(&mut self, id: &NodeId) -> bool {
        if let Some(pos) = self.entries.iter().position(|n| n.identity.node_id == *id) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    fn closest(&self) -> impl Iterator<Item = &NodeInfo> {
        self.entries.iter()
    }
}

#[derive(Debug)]
pub enum InsertResult {
    Inserted,
    Updated,
    BucketFull {
        least_recent: NodeInfo,
        pending: Box<NodeInfo>,
    },
}

/// Kademlia routing table.
#[derive(Debug)]
pub struct RoutingTable {
    local_id: NodeId,
    buckets: Vec<KBucket>,
}

impl RoutingTable {
    pub fn new(local_id: NodeId) -> Self {
        let buckets = (0..NUM_BUCKETS).map(|_| KBucket::new()).collect();
        Self { local_id, buckets }
    }

    pub fn local_id(&self) -> &NodeId {
        &self.local_id
    }

    /// Update the routing table with a seen node. Returns InsertResult.
    pub fn update(&mut self, node: NodeInfo) -> InsertResult {
        if node.identity.node_id == self.local_id {
            return InsertResult::Updated; // ignore self
        }
        let idx = match distance::bucket_index(&self.local_id, &node.identity.node_id) {
            Some(i) => i,
            None => return InsertResult::Updated,
        };
        self.buckets[idx].update(node)
    }

    /// After a failed ping, evict the old node and insert the pending one.
    pub fn evict_and_replace(&mut self, old_id: &NodeId, new_node: NodeInfo) -> bool {
        let idx = match distance::bucket_index(&self.local_id, old_id) {
            Some(i) => i,
            None => return false,
        };
        self.buckets[idx].evict_and_replace(old_id, new_node)
    }

    /// After a successful ping, refresh the least-recently-seen node.
    pub fn refresh(&mut self, id: &NodeId) {
        let idx = match distance::bucket_index(&self.local_id, id) {
            Some(i) => i,
            None => return,
        };
        self.buckets[idx].refresh_least_recent(id);
    }

    /// Remove a node from the routing table.
    pub fn remove(&mut self, id: &NodeId) -> bool {
        let idx = match distance::bucket_index(&self.local_id, id) {
            Some(i) => i,
            None => return false,
        };
        self.buckets[idx].remove(id)
    }

    /// Find the K closest nodes to a target from the routing table.
    pub fn closest(&self, target: &NodeId, count: usize) -> Vec<NodeInfo> {
        let mut all: Vec<&NodeInfo> = self.buckets.iter().flat_map(|b| b.closest()).collect();
        all.sort_by(|a, b| {
            let da = distance::xor_distance(target, &a.identity.node_id);
            let db = distance::xor_distance(target, &b.identity.node_id);
            da.cmp(&db)
        });
        all.into_iter().take(count).cloned().collect()
    }

    /// Total number of entries across all buckets.
    pub fn len(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tesseras_core::{Capabilities, NodeIdentity};

    fn make_node(id_byte: u8) -> NodeInfo {
        NodeInfo {
            identity: NodeIdentity {
                node_id: NodeId::new([id_byte; 20]),
                public_key: [id_byte; 32],
                nonce: 0,
            },
            addr: SocketAddr::from(([127, 0, 0, 1], 4433)),
            alt_addrs: vec![],
            capabilities: Capabilities::phase1_default(),
        }
    }

    fn make_node_id(id_byte: u8) -> NodeId {
        NodeId::new([id_byte; 20])
    }

    #[test]
    fn insert_new_node() {
        let mut rt = RoutingTable::new(make_node_id(0x00));
        let result = rt.update(make_node(0x01));
        assert!(matches!(result, InsertResult::Inserted));
        assert_eq!(rt.len(), 1);
    }

    #[test]
    fn update_existing_node() {
        let mut rt = RoutingTable::new(make_node_id(0x00));
        rt.update(make_node(0x01));
        let result = rt.update(make_node(0x01));
        assert!(matches!(result, InsertResult::Updated));
        assert_eq!(rt.len(), 1);
    }

    #[test]
    fn ignore_self() {
        let mut rt = RoutingTable::new(make_node_id(0x00));
        let result = rt.update(make_node(0x00));
        assert!(matches!(result, InsertResult::Updated));
        assert_eq!(rt.len(), 0);
    }

    #[test]
    fn bucket_full_returns_least_recent() {
        let local = make_node_id(0x00);
        let mut rt = RoutingTable::new(local);

        // Fill a bucket with K nodes that all land in the same bucket.
        // For local=[0x00;20], nodes with last byte 0x80..=0xFF all go to
        // bucket 152 (highest differing bit is bit 7 of byte 19).
        let mut nodes = Vec::new();
        for i in 0..K {
            let mut id_bytes = [0x00u8; 20];
            id_bytes[19] = 0x80 + i as u8;
            nodes.push(NodeInfo {
                identity: NodeIdentity {
                    node_id: NodeId::new(id_bytes),
                    public_key: [i as u8; 32],
                    nonce: 0,
                },
                addr: SocketAddr::from(([127, 0, 0, 1], 4433)),
                alt_addrs: vec![],
                capabilities: Capabilities::phase1_default(),
            });
        }

        for node in &nodes {
            rt.update(node.clone());
        }
        assert_eq!(rt.len(), K);

        // Now insert one more into the same bucket
        let mut extra_bytes = [0x00u8; 20];
        extra_bytes[19] = 0x80 + K as u8;
        let extra = NodeInfo {
            identity: NodeIdentity {
                node_id: NodeId::new(extra_bytes),
                public_key: [0xff; 32],
                nonce: 0,
            },
            addr: SocketAddr::from(([127, 0, 0, 1], 4433)),
            alt_addrs: vec![],
            capabilities: Capabilities::phase1_default(),
        };

        let result = rt.update(extra);
        match result {
            InsertResult::BucketFull { least_recent, .. } => {
                // Least recent should be the first node inserted
                assert_eq!(least_recent.identity.node_id, nodes[0].identity.node_id);
            }
            _ => panic!("expected BucketFull"),
        }
    }

    #[test]
    fn evict_and_replace_works() {
        let local = make_node_id(0x00);
        let mut rt = RoutingTable::new(local);

        let mut id_bytes = [0x00u8; 20];
        id_bytes[19] = 0x01;
        let old_node = NodeInfo {
            identity: NodeIdentity {
                node_id: NodeId::new(id_bytes),
                public_key: [0x01; 32],
                nonce: 0,
            },
            addr: SocketAddr::from(([127, 0, 0, 1], 4433)),
            alt_addrs: vec![],
            capabilities: Capabilities::phase1_default(),
        };
        rt.update(old_node.clone());

        let mut new_bytes = [0x00u8; 20];
        new_bytes[19] = 0x02;
        let new_node = NodeInfo {
            identity: NodeIdentity {
                node_id: NodeId::new(new_bytes),
                public_key: [0x02; 32],
                nonce: 0,
            },
            addr: SocketAddr::from(([127, 0, 0, 1], 4433)),
            alt_addrs: vec![],
            capabilities: Capabilities::phase1_default(),
        };

        let replaced = rt.evict_and_replace(&old_node.identity.node_id, new_node.clone());
        assert!(replaced);
        assert_eq!(rt.len(), 1);

        let closest = rt.closest(&NodeId::new(new_bytes), 1);
        assert_eq!(closest[0].identity.node_id, new_node.identity.node_id);
    }

    #[test]
    fn closest_returns_sorted_by_distance() {
        let local = make_node_id(0x00);
        let mut rt = RoutingTable::new(local);

        // Insert nodes at varying distances
        for i in 1..=5u8 {
            let mut id = [0x00u8; 20];
            id[19] = i;
            rt.update(NodeInfo {
                identity: NodeIdentity {
                    node_id: NodeId::new(id),
                    public_key: [i; 32],
                    nonce: 0,
                },
                addr: SocketAddr::from(([127, 0, 0, 1], 4433)),
                alt_addrs: vec![],
                capabilities: Capabilities::phase1_default(),
            });
        }

        let target = NodeId::new([0x00; 20]);
        let closest = rt.closest(&target, 3);
        assert_eq!(closest.len(), 3);
        // Verify sorted by distance
        for w in closest.windows(2) {
            let da = distance::xor_distance(&target, &w[0].identity.node_id);
            let db = distance::xor_distance(&target, &w[1].identity.node_id);
            assert!(da <= db);
        }
    }

    #[test]
    fn remove_works() {
        let mut rt = RoutingTable::new(make_node_id(0x00));
        let node = make_node(0x01);
        rt.update(node.clone());
        assert_eq!(rt.len(), 1);
        assert!(rt.remove(&node.identity.node_id));
        assert_eq!(rt.len(), 0);
    }
}
