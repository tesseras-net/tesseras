use std::sync::{Arc, Mutex};

use tesseras_core::ports::BlobStore;
use tesseras_core::{ContentHash, CoreError};

use crate::cas::CasStore;

/// CAS-backed blob store with deduplication.
/// Maps (tessera_hash, memory_hash, filename) -> BLAKE3 hash via blob_refs table.
pub struct FsBlobStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
    cas: Arc<CasStore>,
}

impl FsBlobStore {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>, cas: Arc<CasStore>) -> Self {
        Self { conn, cas }
    }
}

impl BlobStore for FsBlobStore {
    fn write(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
        data: &[u8],
    ) -> Result<(), CoreError> {
        let (cas_hash, _dedup) = self.cas.put(data)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO blob_refs (tessera_hash, memory_hash, filename, blake3_hash)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                tessera_hash.to_string(),
                memory_hash.to_string(),
                name,
                cas_hash.to_string(),
            ],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn read(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<Vec<u8>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let cas_hash_hex: String = conn
            .query_row(
                "SELECT blake3_hash FROM blob_refs
                 WHERE tessera_hash = ?1 AND memory_hash = ?2 AND filename = ?3",
                rusqlite::params![
                    tessera_hash.to_string(),
                    memory_hash.to_string(),
                    name,
                ],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        drop(conn);

        let cas_hash: ContentHash = cas_hash_hex
            .parse()
            .map_err(|_| CoreError::Database(format!("invalid hash: {cas_hash_hex}")))?;
        self.cas.get(&cas_hash)
    }

    fn exists(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<bool, CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.prepare(
            "SELECT 1 FROM blob_refs
             WHERE tessera_hash = ?1 AND memory_hash = ?2 AND filename = ?3",
        )
        .map_err(|e| CoreError::Database(e.to_string()))?
        .exists(rusqlite::params![
            tessera_hash.to_string(),
            memory_hash.to_string(),
            name,
        ])
        .map_err(|e| CoreError::Database(e.to_string()))
    }

    fn delete_tessera(&self, tessera_hash: &ContentHash) -> Result<(), CoreError> {
        let hashes: Vec<String> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT blake3_hash FROM blob_refs WHERE tessera_hash = ?1")
                .map_err(|e| CoreError::Database(e.to_string()))?;
            let hashes = stmt
                .query_map(rusqlite::params![tessera_hash.to_string()], |row| {
                    row.get(0)
                })
                .map_err(|e| CoreError::Database(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| CoreError::Database(e.to_string()))?;

            // Delete all blob_refs for this tessera
            conn.execute(
                "DELETE FROM blob_refs WHERE tessera_hash = ?1",
                rusqlite::params![tessera_hash.to_string()],
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
            hashes
        };

        // Release each CAS object (may delete file if refcount hits zero)
        for hash_hex in hashes {
            if let Ok(hash) = hash_hex.parse::<ContentHash>() {
                let _ = self.cas.release(&hash);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (FsBlobStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let conn = crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let cas = Arc::new(CasStore::new(Arc::clone(&conn), dir.path().join("cas")));
        let store = FsBlobStore::new(Arc::clone(&conn), cas);
        (store, dir)
    }

    fn hash(fill: u8) -> ContentHash {
        ContentHash::new([fill; 32])
    }

    #[test]
    fn write_read_roundtrip() {
        let (store, _dir) = setup();
        let t_hash = hash(0x01);
        let m_hash = hash(0x02);
        let data = b"JPEG image data here";
        store.write(&t_hash, &m_hash, "media.jpg", data).unwrap();
        let read = store.read(&t_hash, &m_hash, "media.jpg").unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn read_nonexistent_returns_error() {
        let (store, _dir) = setup();
        let result = store.read(&hash(0x01), &hash(0x02), "nope.jpg");
        assert!(result.is_err());
    }

    #[test]
    fn exists_check() {
        let (store, _dir) = setup();
        let t_hash = hash(0x01);
        let m_hash = hash(0x02);
        assert!(!store.exists(&t_hash, &m_hash, "media.jpg").unwrap());
        store.write(&t_hash, &m_hash, "media.jpg", b"data").unwrap();
        assert!(store.exists(&t_hash, &m_hash, "media.jpg").unwrap());
    }

    #[test]
    fn delete_tessera_removes_all() {
        let (store, _dir) = setup();
        let t_hash = hash(0x01);
        let m_hash = hash(0x02);
        store.write(&t_hash, &m_hash, "media.jpg", b"data").unwrap();
        store.delete_tessera(&t_hash).unwrap();
        assert!(!store.exists(&t_hash, &m_hash, "media.jpg").unwrap());
    }

    #[test]
    fn dedup_across_tesseras() {
        let (store, _dir) = setup();
        let data = b"same photo in two tesseras";
        store
            .write(&hash(0x01), &hash(0xA0), "photo.jpg", data)
            .unwrap();
        store
            .write(&hash(0x02), &hash(0xB0), "photo.jpg", data)
            .unwrap();

        // Delete first tessera, second should still read fine
        store.delete_tessera(&hash(0x01)).unwrap();
        let read = store.read(&hash(0x02), &hash(0xB0), "photo.jpg").unwrap();
        assert_eq!(read, data);
    }
}
