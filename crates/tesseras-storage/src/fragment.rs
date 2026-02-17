use std::sync::{Arc, Mutex};

use rusqlite::OptionalExtension;
use tesseras_core::ports::FragmentStore;
use tesseras_core::replication::FragmentId;
use tesseras_core::{ContentHash, CoreError};

use crate::cas::CasStore;

/// CAS-backed fragment store with deduplication.
/// Maps (tessera_hash, fragment_index) -> BLAKE3 hash via fragment_refs table.
pub struct FsFragmentStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
    cas: Arc<CasStore>,
}

impl FsFragmentStore {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>, cas: Arc<CasStore>) -> Self {
        Self { conn, cas }
    }
}

impl FragmentStore for FsFragmentStore {
    fn store_fragment(&self, id: &FragmentId, data: &[u8]) -> Result<(), CoreError> {
        let (cas_hash, _dedup) = self.cas.put(data)?;

        let conn = self.conn.lock().unwrap();

        // Check if we're replacing an existing ref (release old CAS object)
        let old_hash: Option<String> = conn
            .query_row(
                "SELECT blake3_hash FROM fragment_refs
                 WHERE tessera_hash = ?1 AND fragment_index = ?2",
                rusqlite::params![id.tessera_hash.to_string(), id.index],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| CoreError::Database(e.to_string()))?;

        conn.execute(
            "INSERT OR REPLACE INTO fragment_refs (tessera_hash, fragment_index, blake3_hash)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![id.tessera_hash.to_string(), id.index, cas_hash.to_string(),],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        drop(conn);

        // Release old CAS object if we replaced a ref
        if let Some(old_hex) = old_hash {
            if old_hex != cas_hash.to_string() {
                if let Ok(old) = old_hex.parse::<ContentHash>() {
                    let _ = self.cas.release(&old);
                }
            }
        }

        Ok(())
    }

    fn read_fragment(&self, id: &FragmentId) -> Result<Vec<u8>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let cas_hash_hex: String = conn
            .query_row(
                "SELECT blake3_hash FROM fragment_refs
                 WHERE tessera_hash = ?1 AND fragment_index = ?2",
                rusqlite::params![id.tessera_hash.to_string(), id.index],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        drop(conn);

        let cas_hash: ContentHash = cas_hash_hex
            .parse()
            .map_err(|_| CoreError::Database(format!("invalid hash: {cas_hash_hex}")))?;
        self.cas.get(&cas_hash)
    }

    fn delete_fragment(&self, id: &FragmentId) -> Result<(), CoreError> {
        let cas_hash_hex: Option<String> = {
            let conn = self.conn.lock().unwrap();
            let hex = conn
                .query_row(
                    "SELECT blake3_hash FROM fragment_refs
                     WHERE tessera_hash = ?1 AND fragment_index = ?2",
                    rusqlite::params![id.tessera_hash.to_string(), id.index],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| CoreError::Database(e.to_string()))?;

            conn.execute(
                "DELETE FROM fragment_refs WHERE tessera_hash = ?1 AND fragment_index = ?2",
                rusqlite::params![id.tessera_hash.to_string(), id.index],
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
            hex
        };

        if let Some(hex) = cas_hash_hex {
            if let Ok(hash) = hex.parse::<ContentHash>() {
                let _ = self.cas.release(&hash);
            }
        }
        Ok(())
    }

    fn list_fragments(&self, tessera_hash: &ContentHash) -> Result<Vec<FragmentId>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT fr.fragment_index, fr.blake3_hash
                 FROM fragment_refs fr
                 WHERE fr.tessera_hash = ?1
                 ORDER BY fr.fragment_index",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![tessera_hash.to_string()], |row| {
                let index: u16 = row.get(0)?;
                let checksum_hex: String = row.get(1)?;
                Ok((index, checksum_hex))
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let mut fragments = Vec::new();
        for row in rows {
            let (index, checksum_hex) = row.map_err(|e| CoreError::Database(e.to_string()))?;
            let checksum: ContentHash = checksum_hex
                .parse()
                .map_err(|_| CoreError::Database(format!("invalid checksum: {checksum_hex}")))?;
            // is_parity is derived from the fragment plan, not stored.
            // We use the CAS hash as the checksum for the FragmentId.
            fragments.push(FragmentId {
                tessera_hash: *tessera_hash,
                index,
                is_parity: false, // caller should derive from FragmentPlan
                checksum,
            });
        }
        Ok(fragments)
    }

    fn verify_fragment(&self, id: &FragmentId) -> Result<bool, CoreError> {
        let data = self.read_fragment(id)?;
        let computed = ContentHash::new(blake3::hash(&data).into());
        // Verify against what's stored in fragment_refs (the CAS hash)
        let conn = self.conn.lock().unwrap();
        let stored_hex: String = conn
            .query_row(
                "SELECT blake3_hash FROM fragment_refs
                 WHERE tessera_hash = ?1 AND fragment_index = ?2",
                rusqlite::params![id.tessera_hash.to_string(), id.index],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        let stored: ContentHash = stored_hex
            .parse()
            .map_err(|_| CoreError::Database(format!("invalid hash: {stored_hex}")))?;
        Ok(computed == stored)
    }

    fn list_tessera_hashes(&self) -> Result<Vec<ContentHash>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT DISTINCT tessera_hash FROM fragment_refs ORDER BY tessera_hash")
            .map_err(|e| CoreError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let hash_hex: String = row.get(0)?;
                Ok(hash_hex)
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let mut hashes = Vec::new();
        for row in rows {
            let hex = row.map_err(|e| CoreError::Database(e.to_string()))?;
            let hash: ContentHash = hex
                .parse()
                .map_err(|_| CoreError::Database(format!("invalid hash: {hex}")))?;
            hashes.push(hash);
        }
        Ok(hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (FsFragmentStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let conn = crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let cas = Arc::new(CasStore::new(Arc::clone(&conn), dir.path().join("cas")));
        let store = FsFragmentStore::new(Arc::clone(&conn), cas);
        (store, dir)
    }

    fn hash(fill: u8) -> ContentHash {
        ContentHash::new([fill; 32])
    }

    fn make_frag_id(tessera: u8, index: u16, data: &[u8]) -> FragmentId {
        let checksum = ContentHash::new(blake3::hash(data).into());
        FragmentId::new(hash(tessera), index, 16, checksum)
    }

    #[test]
    fn store_and_read_fragment() {
        let (store, _dir) = setup();
        let data = b"hello fragment data";
        let id = make_frag_id(0x01, 0, data);
        store.store_fragment(&id, data).unwrap();
        let read = store.read_fragment(&id).unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn verify_valid_fragment() {
        let (store, _dir) = setup();
        let data = b"fragment data for verification";
        let id = make_frag_id(0x02, 0, data);
        store.store_fragment(&id, data).unwrap();
        assert!(store.verify_fragment(&id).unwrap());
    }

    #[test]
    fn delete_fragment() {
        let (store, _dir) = setup();
        let data = b"to be deleted";
        let id = make_frag_id(0x03, 0, data);
        store.store_fragment(&id, data).unwrap();
        store.delete_fragment(&id).unwrap();
        assert!(store.read_fragment(&id).is_err());
    }

    #[test]
    fn list_fragments_for_tessera() {
        let (store, _dir) = setup();
        let data = b"list test";
        for i in 0..3 {
            let id = make_frag_id(0x04, i, data);
            store.store_fragment(&id, data).unwrap();
        }
        let frags = store.list_fragments(&hash(0x04)).unwrap();
        assert_eq!(frags.len(), 3);
    }

    #[test]
    fn dedup_across_tesseras() {
        let (store, _dir) = setup();
        let data = b"same fragment in two tesseras";
        let id1 = make_frag_id(0x05, 0, data);
        let id2 = make_frag_id(0x06, 0, data);
        store.store_fragment(&id1, data).unwrap();
        store.store_fragment(&id2, data).unwrap();

        // Delete first, second should still work
        store.delete_fragment(&id1).unwrap();
        let read = store.read_fragment(&id2).unwrap();
        assert_eq!(read, data);
    }
}
