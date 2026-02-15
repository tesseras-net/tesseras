use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::OptionalExtension;
use tesseras_core::{ContentHash, CoreError};

/// Content-addressable store keyed by BLAKE3 hash.
/// Files stored at `<root>/<2-char prefix>/<full_hash>.blob`.
pub struct CasStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
    root: PathBuf,
}

impl CasStore {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>, root: PathBuf) -> Self {
        Self { conn, root }
    }

    /// Return filesystem path for a given BLAKE3 hash.
    fn cas_path(&self, hash: &ContentHash) -> PathBuf {
        let hex = hash.to_string();
        let prefix = &hex[..2];
        self.root.join(prefix).join(format!("{hex}.blob"))
    }

    /// Store data in CAS. Returns the BLAKE3 hash and whether it was a dedup hit.
    /// If the hash already exists, increments ref_count. Otherwise writes file and inserts.
    pub fn put(&self, data: &[u8]) -> Result<(ContentHash, bool), CoreError> {
        let hash = ContentHash::new(blake3::hash(data).into());
        let path = self.cas_path(&hash);

        let conn = self.conn.lock().unwrap();

        // Check if already in CAS
        let existing: Option<i64> = conn
            .query_row(
                "SELECT ref_count FROM cas_objects WHERE blake3_hash = ?1",
                rusqlite::params![hash.to_string()],
                |row| row.get(0),
            )
            .ok();

        if existing.is_some() {
            // Dedup hit: increment ref_count, no file write
            conn.execute(
                "UPDATE cas_objects SET ref_count = ref_count + 1 WHERE blake3_hash = ?1",
                rusqlite::params![hash.to_string()],
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
            Ok((hash, true))
        } else {
            // New object: write file first, then insert into SQLite
            drop(conn); // release lock before file I/O
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, data)?;

            let conn = self.conn.lock().unwrap();
            let result = conn.execute(
                "INSERT INTO cas_objects (blake3_hash, size_bytes, ref_count, stored_at)
                 VALUES (?1, ?2, 1, ?3)",
                rusqlite::params![
                    hash.to_string(),
                    data.len() as i64,
                    chrono::Utc::now().to_rfc3339(),
                ],
            );
            match result {
                Ok(_) => Ok((hash, false)),
                Err(e) => {
                    // Best-effort cleanup of the file we just wrote
                    let _ = std::fs::remove_file(&path);
                    Err(CoreError::Database(e.to_string()))
                }
            }
        }
    }

    /// Read data from CAS by hash.
    pub fn get(&self, hash: &ContentHash) -> Result<Vec<u8>, CoreError> {
        let path = self.cas_path(hash);
        std::fs::read(&path).map_err(CoreError::Io)
    }

    /// Decrement ref_count. If it reaches zero, remove from SQLite and delete file.
    /// Returns true if the object was fully removed.
    pub fn release(&self, hash: &ContentHash) -> Result<bool, CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE cas_objects SET ref_count = ref_count - 1 WHERE blake3_hash = ?1",
            rusqlite::params![hash.to_string()],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;

        let ref_count: i64 = conn
            .query_row(
                "SELECT ref_count FROM cas_objects WHERE blake3_hash = ?1",
                rusqlite::params![hash.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        if ref_count <= 0 {
            conn.execute(
                "DELETE FROM cas_objects WHERE blake3_hash = ?1",
                rusqlite::params![hash.to_string()],
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
            drop(conn);
            let path = self.cas_path(hash);
            let _ = std::fs::remove_file(&path); // best-effort
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if hash exists in CAS.
    pub fn contains(&self, hash: &ContentHash) -> Result<bool, CoreError> {
        let conn = self.conn.lock().unwrap();
        let exists = conn
            .prepare("SELECT 1 FROM cas_objects WHERE blake3_hash = ?1")
            .map_err(|e| CoreError::Database(e.to_string()))?
            .exists(rusqlite::params![hash.to_string()])
            .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(exists)
    }

    /// Get current ref_count for a hash. Returns None if not in CAS.
    pub fn ref_count(&self, hash: &ContentHash) -> Result<Option<i64>, CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT ref_count FROM cas_objects WHERE blake3_hash = ?1",
            rusqlite::params![hash.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| CoreError::Database(e.to_string()))
    }

    /// Run a GC sweep: remove orphan files and leaked refcount entries.
    /// Grace period: files younger than 10 minutes are skipped.
    pub fn sweep(&self) -> Result<SweepStats, CoreError> {
        let mut stats = SweepStats::default();
        let grace_period = std::time::Duration::from_secs(600); // 10 minutes
        let now = std::time::SystemTime::now();

        // 1. Remove leaked refcounts (cas_objects with no refs)
        let conn = self.conn.lock().unwrap();
        let leaked = conn
            .execute(
                "DELETE FROM cas_objects WHERE blake3_hash IN (
                    SELECT c.blake3_hash FROM cas_objects c
                    LEFT JOIN blob_refs b ON c.blake3_hash = b.blake3_hash
                    LEFT JOIN fragment_refs f ON c.blake3_hash = f.blake3_hash
                    WHERE b.blake3_hash IS NULL AND f.blake3_hash IS NULL
                )",
                [],
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        stats.leaked_refs_removed = leaked as u64;
        drop(conn);

        // 2. Scan filesystem for orphan files
        if self.root.exists() {
            for prefix_entry in std::fs::read_dir(&self.root)? {
                let prefix_entry = prefix_entry?;
                if !prefix_entry.file_type()?.is_dir() {
                    continue;
                }
                for blob_entry in std::fs::read_dir(prefix_entry.path())? {
                    let blob_entry = blob_entry?;
                    let path = blob_entry.path();
                    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                        continue;
                    };

                    // Check grace period
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        if let Ok(mtime) = metadata.modified() {
                            if let Ok(age) = now.duration_since(mtime) {
                                if age < grace_period {
                                    stats.orphan_files_skipped_young += 1;
                                    continue;
                                }
                            }
                        }
                    }

                    // Check if hash exists in cas_objects
                    let conn = self.conn.lock().unwrap();
                    let exists = conn
                        .prepare("SELECT 1 FROM cas_objects WHERE blake3_hash = ?1")
                        .map_err(|e| CoreError::Database(e.to_string()))?
                        .exists(rusqlite::params![stem])
                        .map_err(|e| CoreError::Database(e.to_string()))?;
                    drop(conn);

                    if !exists {
                        let _ = std::fs::remove_file(&path);
                        stats.orphan_files_removed += 1;
                    }
                }
            }
        }

        Ok(stats)
    }
}

/// Statistics from a CAS sweep run.
#[derive(Debug, Default)]
pub struct SweepStats {
    pub orphan_files_removed: u64,
    pub orphan_files_skipped_young: u64,
    pub leaked_refs_removed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (CasStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let conn = crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
        let store = CasStore::new(Arc::new(Mutex::new(conn)), dir.path().join("cas"));
        (store, dir)
    }

    #[test]
    fn put_and_get_roundtrip() {
        let (store, _dir) = setup();
        let data = b"hello CAS world";
        let (hash, is_dedup) = store.put(data).unwrap();
        assert!(!is_dedup);
        let read = store.get(&hash).unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn put_same_data_twice_is_dedup_hit() {
        let (store, _dir) = setup();
        let data = b"duplicate data";
        let (hash1, dedup1) = store.put(data).unwrap();
        let (hash2, dedup2) = store.put(data).unwrap();
        assert_eq!(hash1, hash2);
        assert!(!dedup1);
        assert!(dedup2);
        assert_eq!(store.ref_count(&hash1).unwrap(), Some(2));
    }

    #[test]
    fn release_decrements_refcount() {
        let (store, _dir) = setup();
        let data = b"release test";
        let (hash, _) = store.put(data).unwrap();
        store.put(data).unwrap(); // ref_count = 2

        let removed = store.release(&hash).unwrap();
        assert!(!removed);
        assert_eq!(store.ref_count(&hash).unwrap(), Some(1));
    }

    #[test]
    fn release_to_zero_removes_file() {
        let (store, _dir) = setup();
        let data = b"will be removed";
        let (hash, _) = store.put(data).unwrap();

        let removed = store.release(&hash).unwrap();
        assert!(removed);
        assert_eq!(store.ref_count(&hash).unwrap(), None);
        assert!(store.get(&hash).is_err());
    }

    #[test]
    fn contains_returns_true_for_existing() {
        let (store, _dir) = setup();
        let data = b"exists check";
        let (hash, _) = store.put(data).unwrap();
        assert!(store.contains(&hash).unwrap());
    }

    #[test]
    fn contains_returns_false_after_full_release() {
        let (store, _dir) = setup();
        let data = b"gone soon";
        let (hash, _) = store.put(data).unwrap();
        store.release(&hash).unwrap();
        assert!(!store.contains(&hash).unwrap());
    }

    #[test]
    fn cas_path_uses_two_char_prefix() {
        let (store, dir) = setup();
        let data = b"path test";
        let (hash, _) = store.put(data).unwrap();
        let hex = hash.to_string();
        let expected = dir
            .path()
            .join("cas")
            .join(&hex[..2])
            .join(format!("{hex}.blob"));
        assert!(expected.exists());
    }

    #[test]
    fn sweep_removes_orphan_files() {
        let (store, dir) = setup();
        // Create an orphan file directly in CAS dir (no cas_objects entry)
        let orphan_dir = dir.path().join("cas").join("ab");
        std::fs::create_dir_all(&orphan_dir).unwrap();
        let orphan_path = orphan_dir
            .join("ab00000000000000000000000000000000000000000000000000000000000000.blob");
        std::fs::write(&orphan_path, b"orphan").unwrap();
        // Set mtime to 20 minutes ago
        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(1200);
        filetime::set_file_mtime(&orphan_path, filetime::FileTime::from_system_time(old_time))
            .unwrap();

        let stats = store.sweep().unwrap();
        assert_eq!(stats.orphan_files_removed, 1);
        assert!(!orphan_path.exists());
    }

    #[test]
    fn sweep_skips_young_orphan_files() {
        let (store, dir) = setup();
        // Create an orphan file with recent mtime (just now)
        let orphan_dir = dir.path().join("cas").join("cd");
        std::fs::create_dir_all(&orphan_dir).unwrap();
        let orphan_path = orphan_dir
            .join("cd00000000000000000000000000000000000000000000000000000000000000.blob");
        std::fs::write(&orphan_path, b"young orphan").unwrap();

        let stats = store.sweep().unwrap();
        assert_eq!(stats.orphan_files_removed, 0);
        assert_eq!(stats.orphan_files_skipped_young, 1);
        assert!(orphan_path.exists());
    }

    #[test]
    fn sweep_removes_leaked_refcounts() {
        let (store, _dir) = setup();
        // Manually insert a cas_objects row with no refs
        {
            let conn = store.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO cas_objects (blake3_hash, size_bytes, ref_count, stored_at)
                 VALUES ('deadbeef', 100, 1, '2026-01-01')",
                [],
            )
            .unwrap();
        }
        let stats = store.sweep().unwrap();
        assert_eq!(stats.leaked_refs_removed, 1);
    }

    #[test]
    fn sweep_idempotent() {
        let (store, _dir) = setup();
        let stats1 = store.sweep().unwrap();
        let stats2 = store.sweep().unwrap();
        assert_eq!(stats1.orphan_files_removed, 0);
        assert_eq!(stats2.orphan_files_removed, 0);
        assert_eq!(stats1.leaked_refs_removed, 0);
        assert_eq!(stats2.leaked_refs_removed, 0);
    }

    #[test]
    fn cross_tessera_dedup_survives_single_release() {
        let (store, _dir) = setup();
        let data = b"shared across tesseras";
        let (hash, _) = store.put(data).unwrap();
        store.put(data).unwrap(); // second reference

        store.release(&hash).unwrap(); // remove one ref
        // Data should still be readable
        let read = store.get(&hash).unwrap();
        assert_eq!(read, data);
    }

    mod proptests {
        use super::*;
        use proptest::collection::vec as prop_vec;
        use proptest::prelude::*;

        /// Represents a put or release action on a specific data blob.
        #[derive(Debug, Clone)]
        enum Action {
            Put(Vec<u8>),
            Release(usize), // index into previously put items
        }

        fn action_strategy() -> impl Strategy<Value = Vec<Action>> {
            prop_vec(
                prop_oneof![
                    prop_vec(any::<u8>(), 1..64).prop_map(Action::Put),
                    (0..10usize).prop_map(Action::Release),
                ],
                1..50,
            )
        }

        proptest! {
            #[test]
            fn refcount_matches_actual_refs(actions in action_strategy()) {
                let dir = TempDir::new().unwrap();
                let conn = crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
                let store = CasStore::new(Arc::new(Mutex::new(conn)), dir.path().join("cas"));

                let mut put_hashes: Vec<ContentHash> = Vec::new();
                // Track expected refcount per hash
                let mut expected_refs: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

                for action in &actions {
                    match action {
                        Action::Put(data) => {
                            let (hash, _) = store.put(data).unwrap();
                            put_hashes.push(hash);
                            *expected_refs.entry(hash.to_string()).or_insert(0) += 1;
                        }
                        Action::Release(idx) => {
                            if !put_hashes.is_empty() {
                                let idx = idx % put_hashes.len();
                                let hash = put_hashes[idx];
                                let hex = hash.to_string();
                                if let Some(count) = expected_refs.get(&hex) {
                                    if *count > 0 {
                                        let _ = store.release(&hash);
                                        *expected_refs.get_mut(&hex).unwrap() -= 1;
                                    }
                                }
                            }
                        }
                    }
                }

                // Verify all refcounts match
                for (hex, expected) in &expected_refs {
                    let actual = store.ref_count(
                        &hex.parse::<ContentHash>().unwrap()
                    ).unwrap();
                    if *expected == 0 {
                        prop_assert!(actual.is_none() || actual == Some(0),
                            "hash {} expected refcount 0, got {:?}", hex, actual);
                    } else {
                        prop_assert_eq!(actual, Some(*expected),
                            "hash {} refcount mismatch", hex);
                    }
                }
            }

            #[test]
            fn cas_path_is_deterministic(data in prop_vec(any::<u8>(), 1..256)) {
                let dir = TempDir::new().unwrap();
                let conn = crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
                let store = CasStore::new(Arc::new(Mutex::new(conn)), dir.path().join("cas"));

                let hash = ContentHash::new(blake3::hash(&data).into());
                let path1 = store.cas_path(&hash);
                let path2 = store.cas_path(&hash);
                prop_assert_eq!(&path1, &path2);

                // Verify prefix matches first 2 chars of hex
                let hex = hash.to_string();
                let prefix = path1.parent().unwrap().file_name().unwrap().to_str().unwrap();
                prop_assert_eq!(prefix, &hex[..2]);
            }
        }
    }
}
