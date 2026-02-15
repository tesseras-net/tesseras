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
}
