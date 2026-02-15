use std::num::NonZeroUsize;
use std::sync::Mutex;

use lru::LruCache;
use tesseras_core::ports::FragmentStore;
use tesseras_core::replication::FragmentId;
use tesseras_core::{ContentHash, CoreError};

/// LRU-cached decorator over a FragmentStore implementation.
/// Caches blob data in memory, keyed by (tessera_hash, fragment_index).
/// Thread-safe via internal Mutex (FragmentStore is sync).
pub struct CachedFragmentStore {
    inner: Box<dyn FragmentStore>,
    cache: Mutex<CacheInner>,
}

struct CacheInner {
    lru: LruCache<(ContentHash, u16), Vec<u8>>,
    current_bytes: usize,
    max_bytes: usize,
}

impl CachedFragmentStore {
    pub fn new(inner: Box<dyn FragmentStore>, max_bytes: usize) -> Self {
        Self {
            inner,
            cache: Mutex::new(CacheInner {
                lru: LruCache::new(NonZeroUsize::new(4096).unwrap()),
                current_bytes: 0,
                max_bytes,
            }),
        }
    }

    fn evict_until_fits(cache: &mut CacheInner, needed: usize) {
        while cache.current_bytes + needed > cache.max_bytes {
            if let Some((_, evicted)) = cache.lru.pop_lru() {
                cache.current_bytes -= evicted.len();
            } else {
                break;
            }
        }
    }
}

impl FragmentStore for CachedFragmentStore {
    fn store_fragment(&self, id: &FragmentId, data: &[u8]) -> Result<(), CoreError> {
        self.inner.store_fragment(id, data)?;
        let mut cache = self.cache.lock().unwrap();
        if let Some(old) = cache.lru.pop(&(id.tessera_hash, id.index)) {
            cache.current_bytes -= old.len();
        }
        Ok(())
    }

    fn read_fragment(&self, id: &FragmentId) -> Result<Vec<u8>, CoreError> {
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(data) = cache.lru.get(&(id.tessera_hash, id.index)) {
                return Ok(data.clone());
            }
        }
        let data = self.inner.read_fragment(id)?;
        {
            let mut cache = self.cache.lock().unwrap();
            Self::evict_until_fits(&mut cache, data.len());
            cache.current_bytes += data.len();
            cache.lru.put((id.tessera_hash, id.index), data.clone());
        }
        Ok(data)
    }

    fn delete_fragment(&self, id: &FragmentId) -> Result<(), CoreError> {
        self.inner.delete_fragment(id)?;
        let mut cache = self.cache.lock().unwrap();
        if let Some(old) = cache.lru.pop(&(id.tessera_hash, id.index)) {
            cache.current_bytes -= old.len();
        }
        Ok(())
    }

    fn list_fragments(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<Vec<FragmentId>, CoreError> {
        self.inner.list_fragments(tessera_hash)
    }

    fn verify_fragment(&self, id: &FragmentId) -> Result<bool, CoreError> {
        self.inner.verify_fragment(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeStore {
        data: std::collections::HashMap<(ContentHash, u16), Vec<u8>>,
    }

    impl FakeStore {
        fn new() -> Self {
            Self {
                data: std::collections::HashMap::new(),
            }
        }

        fn insert(&mut self, tessera_hash: ContentHash, index: u16, blob: Vec<u8>) {
            self.data.insert((tessera_hash, index), blob);
        }
    }

    impl FragmentStore for FakeStore {
        fn store_fragment(&self, _id: &FragmentId, _data: &[u8]) -> Result<(), CoreError> {
            Ok(())
        }
        fn read_fragment(&self, id: &FragmentId) -> Result<Vec<u8>, CoreError> {
            self.data
                .get(&(id.tessera_hash, id.index))
                .cloned()
                .ok_or_else(|| CoreError::Database("not found".into()))
        }
        fn delete_fragment(&self, _id: &FragmentId) -> Result<(), CoreError> {
            Ok(())
        }
        fn list_fragments(
            &self,
            _tessera_hash: &ContentHash,
        ) -> Result<Vec<FragmentId>, CoreError> {
            Ok(vec![])
        }
        fn verify_fragment(&self, _id: &FragmentId) -> Result<bool, CoreError> {
            Ok(true)
        }
    }

    fn hash(fill: u8) -> ContentHash {
        ContentHash::new([fill; 32])
    }

    fn frag_id(fill: u8, index: u16) -> FragmentId {
        FragmentId::new(hash(fill), index, 16, hash(0xFF))
    }

    #[test]
    fn cache_hit_avoids_inner_read() {
        let mut store = FakeStore::new();
        store.insert(hash(0x01), 0, vec![1, 2, 3]);
        let cached = CachedFragmentStore::new(Box::new(store), 1024);

        let id = frag_id(0x01, 0);
        // First read: cache miss, reads from inner
        let data1 = cached.read_fragment(&id).unwrap();
        assert_eq!(data1, vec![1, 2, 3]);

        // Second read: cache hit (inner could be gone but cache serves it)
        let data2 = cached.read_fragment(&id).unwrap();
        assert_eq!(data2, vec![1, 2, 3]);
    }

    #[test]
    fn store_invalidates_cache() {
        let mut store = FakeStore::new();
        store.insert(hash(0x02), 0, vec![10, 20]);
        let cached = CachedFragmentStore::new(Box::new(store), 1024);

        let id = frag_id(0x02, 0);
        cached.read_fragment(&id).unwrap(); // populate cache
        cached.store_fragment(&id, &[30, 40]).unwrap(); // invalidate

        // Cache should be empty for this key; next read goes to inner
        // Inner still has old data (FakeStore doesn't actually write)
        let data = cached.read_fragment(&id).unwrap();
        assert_eq!(data, vec![10, 20]); // re-read from inner
    }

    #[test]
    fn evicts_when_over_max_bytes() {
        let mut store = FakeStore::new();
        store.insert(hash(0x03), 0, vec![0; 600]);
        store.insert(hash(0x03), 1, vec![1; 600]);
        // max_bytes = 1000, each entry is 600 bytes
        let cached = CachedFragmentStore::new(Box::new(store), 1000);

        let id0 = frag_id(0x03, 0);
        let id1 = frag_id(0x03, 1);

        cached.read_fragment(&id0).unwrap(); // 600 bytes in cache
        cached.read_fragment(&id1).unwrap(); // 1200 > 1000, should evict id0

        let inner = cached.cache.lock().unwrap();
        assert!(inner.lru.peek(&(hash(0x03), 1)).is_some());
        // id0 should have been evicted
        assert!(inner.lru.peek(&(hash(0x03), 0)).is_none());
        assert!(inner.current_bytes <= 1000);
    }
}
