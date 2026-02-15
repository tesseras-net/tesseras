use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tesseras_core::ports::FragmentStore;
use tesseras_core::replication::FragmentId;
use tesseras_core::{ContentHash, CoreError};

/// Filesystem-backed fragment store with SQLite metadata index.
pub struct FsFragmentStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
    root: PathBuf,
}

impl FsFragmentStore {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>, root: PathBuf) -> Self {
        Self { conn, root }
    }

    fn blob_path(&self, tessera_hash: &ContentHash, index: u16) -> PathBuf {
        self.root
            .join(tessera_hash.to_string())
            .join(format!("{index:03}.shard"))
    }
}

impl FragmentStore for FsFragmentStore {
    fn store_fragment(&self, id: &FragmentId, data: &[u8]) -> Result<(), CoreError> {
        let path = self.blob_path(&id.tessera_hash, id.index);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, data)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO fragments
             (tessera_hash, fragment_index, is_parity, checksum, size_bytes, blob_path, stored_at, last_verified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            rusqlite::params![
                id.tessera_hash.to_string(),
                id.index,
                id.is_parity as i32,
                id.checksum.to_string(),
                data.len() as i64,
                path.to_string_lossy().to_string(),
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn read_fragment(&self, id: &FragmentId) -> Result<Vec<u8>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let path: String = conn
            .query_row(
                "SELECT blob_path FROM fragments WHERE tessera_hash = ?1 AND fragment_index = ?2",
                rusqlite::params![id.tessera_hash.to_string(), id.index],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        drop(conn);
        let data = std::fs::read(path)?;
        Ok(data)
    }

    fn delete_fragment(&self, id: &FragmentId) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        let path: String = conn
            .query_row(
                "SELECT blob_path FROM fragments WHERE tessera_hash = ?1 AND fragment_index = ?2",
                rusqlite::params![id.tessera_hash.to_string(), id.index],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        conn.execute(
            "DELETE FROM fragments WHERE tessera_hash = ?1 AND fragment_index = ?2",
            rusqlite::params![id.tessera_hash.to_string(), id.index],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        drop(conn);
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    fn list_fragments(&self, tessera_hash: &ContentHash) -> Result<Vec<FragmentId>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT fragment_index, is_parity, checksum FROM fragments WHERE tessera_hash = ?1 ORDER BY fragment_index",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![tessera_hash.to_string()], |row| {
                let index: u16 = row.get(0)?;
                let is_parity: bool = row.get(1)?;
                let checksum_hex: String = row.get(2)?;
                Ok((index, is_parity, checksum_hex))
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let mut fragments = Vec::new();
        for row in rows {
            let (index, is_parity, checksum_hex) =
                row.map_err(|e| CoreError::Database(e.to_string()))?;
            let checksum: ContentHash = checksum_hex
                .parse()
                .map_err(|_| CoreError::Database(format!("invalid checksum: {checksum_hex}")))?;
            fragments.push(FragmentId {
                tessera_hash: *tessera_hash,
                index,
                is_parity,
                checksum,
            });
        }
        Ok(fragments)
    }

    fn verify_fragment(&self, id: &FragmentId) -> Result<bool, CoreError> {
        let data = self.read_fragment(id)?;
        let computed = ContentHash::new(blake3::hash(&data).into());
        Ok(computed == id.checksum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    fn setup() -> (FsFragmentStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let conn = crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
        let store = FsFragmentStore::new(Arc::new(Mutex::new(conn)), dir.path().join("fragments"));
        (store, dir)
    }

    fn hash(fill: u8) -> ContentHash {
        ContentHash::new([fill; 32])
    }

    #[test]
    fn store_and_read_fragment() {
        let (store, _dir) = setup();
        let data = b"hello fragment data";
        let checksum = ContentHash::new(blake3::hash(data).into());
        let id = FragmentId::new(hash(0x01), 0, 16, checksum);

        store.store_fragment(&id, data).unwrap();
        let read = store.read_fragment(&id).unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn verify_valid_fragment() {
        let (store, _dir) = setup();
        let data = b"fragment data for verification";
        let checksum = ContentHash::new(blake3::hash(data).into());
        let id = FragmentId::new(hash(0x02), 0, 16, checksum);

        store.store_fragment(&id, data).unwrap();
        assert!(store.verify_fragment(&id).unwrap());
    }

    #[test]
    fn delete_fragment() {
        let (store, _dir) = setup();
        let data = b"to be deleted";
        let checksum = ContentHash::new(blake3::hash(data).into());
        let id = FragmentId::new(hash(0x03), 0, 16, checksum);

        store.store_fragment(&id, data).unwrap();
        store.delete_fragment(&id).unwrap();
        assert!(store.read_fragment(&id).is_err());
    }

    #[test]
    fn list_fragments_for_tessera() {
        let (store, _dir) = setup();
        let data = b"list test";
        let checksum = ContentHash::new(blake3::hash(data).into());

        for i in 0..3 {
            let id = FragmentId::new(hash(0x04), i, 16, checksum);
            store.store_fragment(&id, data).unwrap();
        }
        let frags = store.list_fragments(&hash(0x04)).unwrap();
        assert_eq!(frags.len(), 3);
    }
}
