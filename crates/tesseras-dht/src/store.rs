use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use tesseras_core::{ContentHash, NodeId, TesseraPointer};

use crate::distance;

/// Configuration for the pointer store.
#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub max_entries: usize,
    pub ttl: Duration,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            ttl: Duration::from_secs(86400), // 24 hours
        }
    }
}

struct StoreEntry {
    pointer: TesseraPointer,
    last_refreshed: Instant,
}

/// Bounded pointer store with TTL and distance-based eviction.
pub struct PointerStore {
    local_id: NodeId,
    entries: HashMap<ContentHash, StoreEntry>,
    tombstones: HashSet<ContentHash>,
    config: StoreConfig,
}

impl PointerStore {
    pub fn new(local_id: NodeId, config: StoreConfig) -> Self {
        Self {
            local_id,
            entries: HashMap::new(),
            tombstones: HashSet::new(),
            config,
        }
    }

    /// Record a tombstone: removes the pointer and blocks future stores.
    pub fn add_tombstone(&mut self, hash: ContentHash) {
        self.tombstones.insert(hash);
        self.entries.remove(&hash);
    }

    /// Check whether a hash has been tombstoned.
    pub fn is_tombstoned(&self, hash: &ContentHash) -> bool {
        self.tombstones.contains(hash)
    }

    /// All tombstoned hashes (for republishing retract messages).
    pub fn tombstoned_hashes(&self) -> Vec<ContentHash> {
        self.tombstones.iter().cloned().collect()
    }

    /// Store or refresh a pointer. Returns true if accepted.
    /// Rejects pointers for tombstoned hashes.
    pub fn store(&mut self, pointer: TesseraPointer) -> bool {
        let key = pointer.tessera_hash;

        // Reject stores for tombstoned hashes
        if self.tombstones.contains(&key) {
            return false;
        }

        let now = Instant::now();

        // If key exists, refresh it
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.pointer = pointer;
            entry.last_refreshed = now;
            return true;
        }

        // Evict expired entries first
        self.evict_expired(now);

        // If still full, evict entry furthest from our ID
        if self.entries.len() >= self.config.max_entries {
            self.evict_furthest();
        }

        // Should have space now
        if self.entries.len() >= self.config.max_entries {
            return false;
        }

        self.entries.insert(
            key,
            StoreEntry {
                pointer,
                last_refreshed: now,
            },
        );
        true
    }

    /// Look up a pointer by content hash.
    pub fn get(&self, key: &ContentHash) -> Option<&TesseraPointer> {
        let entry = self.entries.get(key)?;
        // Check TTL
        if entry.last_refreshed.elapsed() > self.config.ttl {
            return None;
        }
        Some(&entry.pointer)
    }

    /// Remove expired entries.
    pub fn evict_expired(&mut self, now: Instant) {
        self.entries
            .retain(|_, entry| now.duration_since(entry.last_refreshed) <= self.config.ttl);
    }

    /// Evict the entry whose key is furthest from our node ID.
    fn evict_furthest(&mut self) {
        // ContentHash is 32 bytes, NodeId is 20 bytes. We compare the first 20 bytes.
        let furthest = self
            .entries
            .keys()
            .max_by_key(|key| {
                let key_bytes = key.as_bytes();
                let mut trunc = [0u8; 20];
                trunc.copy_from_slice(&key_bytes[..20]);
                let key_as_node = NodeId::new(trunc);
                distance::xor_distance(&self.local_id, &key_as_node)
            })
            .cloned();
        if let Some(key) = furthest {
            self.entries.remove(&key);
        }
    }

    /// Number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All stored keys (for republishing).
    pub fn keys(&self) -> Vec<ContentHash> {
        self.entries.keys().cloned().collect()
    }

    /// All stored pointers (for republishing).
    pub fn pointers(&self) -> Vec<TesseraPointer> {
        self.entries.values().map(|e| e.pointer.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseras_core::{HolderInfo, Visibility};

    fn make_pointer(hash_byte: u8) -> TesseraPointer {
        TesseraPointer {
            tessera_hash: ContentHash::new([hash_byte; 32]),
            size_bytes: 1000,
            holders: vec![HolderInfo {
                node_id: NodeId::new([0x01; 20]),
                addr: "127.0.0.1:4433".parse().unwrap(),
                alt_addrs: vec![],
                last_seen: chrono::Utc::now(),
                fragments: vec![],
            }],
            visibility: Visibility::Public,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn store_and_get() {
        let mut store = PointerStore::new(NodeId::new([0x00; 20]), StoreConfig::default());
        let ptr = make_pointer(0xaa);
        assert!(store.store(ptr));
        assert!(store.get(&ContentHash::new([0xaa; 32])).is_some());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let store = PointerStore::new(NodeId::new([0x00; 20]), StoreConfig::default());
        assert!(store.get(&ContentHash::new([0xff; 32])).is_none());
    }

    #[test]
    fn refresh_updates_existing() {
        let mut store = PointerStore::new(NodeId::new([0x00; 20]), StoreConfig::default());
        let ptr1 = make_pointer(0xaa);
        store.store(ptr1);
        assert_eq!(store.len(), 1);

        // Store again (refresh)
        let ptr2 = make_pointer(0xaa);
        store.store(ptr2);
        assert_eq!(store.len(), 1); // No duplicate
    }

    #[test]
    fn max_entries_enforced() {
        let mut store = PointerStore::new(
            NodeId::new([0x00; 20]),
            StoreConfig {
                max_entries: 3,
                ttl: Duration::from_secs(3600),
            },
        );
        for i in 0..5u8 {
            store.store(make_pointer(i));
        }
        assert!(store.len() <= 3);
    }

    #[test]
    fn evict_expired_removes_old_entries() {
        let mut store = PointerStore::new(
            NodeId::new([0x00; 20]),
            StoreConfig {
                max_entries: 100,
                ttl: Duration::from_millis(1),
            },
        );
        store.store(make_pointer(0xaa));
        assert_eq!(store.len(), 1);

        // Wait for TTL
        std::thread::sleep(Duration::from_millis(10));
        store.evict_expired(Instant::now());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn get_returns_none_for_expired() {
        let mut store = PointerStore::new(
            NodeId::new([0x00; 20]),
            StoreConfig {
                max_entries: 100,
                ttl: Duration::from_millis(1),
            },
        );
        store.store(make_pointer(0xaa));
        std::thread::sleep(Duration::from_millis(10));
        assert!(store.get(&ContentHash::new([0xaa; 32])).is_none());
    }

    #[test]
    fn store_rejected_after_tombstone() {
        let mut store = PointerStore::new(NodeId::new([0x00; 20]), StoreConfig::default());
        store.add_tombstone(ContentHash::new([0xaa; 32]));
        assert!(!store.store(make_pointer(0xaa)));
        assert!(store.get(&ContentHash::new([0xaa; 32])).is_none());
    }

    #[test]
    fn retract_removes_existing_pointer() {
        let mut store = PointerStore::new(NodeId::new([0x00; 20]), StoreConfig::default());
        assert!(store.store(make_pointer(0xaa)));
        assert!(store.get(&ContentHash::new([0xaa; 32])).is_some());

        store.add_tombstone(ContentHash::new([0xaa; 32]));
        assert!(store.get(&ContentHash::new([0xaa; 32])).is_none());
        assert!(store.is_tombstoned(&ContentHash::new([0xaa; 32])));
    }
}
