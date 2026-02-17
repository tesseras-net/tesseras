use std::io::{self, Read, Write};
use std::net::SocketAddr;

use rusqlite::Connection;

use crate::config::DataDir;
use crate::dht::PeerInfo;
use crate::types::{ContentHash, MediaType, Memory, NodeId, Tessera};

/// Current schema version.
const SCHEMA_VERSION: u32 = 2;

/// Storage engine: SQLite metadata + blob CAS on filesystem.
pub struct Storage {
    db: Connection,
    data_dir: DataDir,
}

impl Storage {
    /// Open storage in the given data directory. Creates tables if needed.
    pub fn open(data_dir: DataDir) -> Result<Self, StorageError> {
        let db = Connection::open(data_dir.database_path())?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let storage = Self { db, data_dir };
        storage.run_migrations()?;
        Ok(storage)
    }

    /// Versioned migration system. Each version adds its DDL idempotently.
    fn run_migrations(&self) -> Result<(), StorageError> {
        // Ensure the meta table exists for tracking schema version
        self.db.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;

        let current: u32 = self
            .db
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'version'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        if current < 1 {
            self.migrate_v1()?;
        }
        if current < 2 {
            self.migrate_v2()?;
        }

        // Update stored version
        self.db.execute(
            "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('version', ?1)",
            rusqlite::params![SCHEMA_VERSION.to_string()],
        )?;

        Ok(())
    }

    /// V1: Original schema — tesseras, memories, circles, peers, fragments.
    fn migrate_v1(&self) -> Result<(), StorageError> {
        self.db.execute_batch(
            "CREATE TABLE IF NOT EXISTS tesseras (
                hash        TEXT PRIMARY KEY,
                author      TEXT NOT NULL,
                name        TEXT,
                visibility  TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                signature   BLOB NOT NULL,
                total_size  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS memories (
                id           INTEGER PRIMARY KEY,
                tessera_hash TEXT NOT NULL REFERENCES tesseras(hash),
                filename     TEXT NOT NULL,
                media_type   TEXT NOT NULL,
                size         INTEGER NOT NULL,
                blob_hash    TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS circles (
                name        TEXT NOT NULL,
                member_key  TEXT NOT NULL,
                PRIMARY KEY (name, member_key)
            );

            CREATE TABLE IF NOT EXISTS peers (
                node_id     TEXT PRIMARY KEY,
                addr        TEXT NOT NULL,
                last_seen   TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS fragments (
                id              INTEGER PRIMARY KEY,
                blob_hash       TEXT NOT NULL,
                fragment_index  INTEGER NOT NULL,
                fragment_hash   TEXT NOT NULL,
                shard_size      INTEGER NOT NULL,
                original_size   INTEGER NOT NULL,
                data_shards     INTEGER NOT NULL,
                parity_shards   INTEGER NOT NULL,
                UNIQUE(blob_hash, fragment_index)
            );",
        )?;
        Ok(())
    }

    /// V2: Add reciprocity ledger for bilateral storage accounting.
    fn migrate_v2(&self) -> Result<(), StorageError> {
        self.db.execute_batch(
            "CREATE TABLE IF NOT EXISTS reciprocity_ledger (
                peer_node_id    TEXT PRIMARY KEY,
                bytes_stored    INTEGER NOT NULL DEFAULT 0,
                bytes_served    INTEGER NOT NULL DEFAULT 0,
                last_updated    TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    /// Store a blob by streaming from a reader. Returns the BLAKE3 hash.
    pub fn store_blob(&self, reader: &mut dyn Read) -> Result<ContentHash, StorageError> {
        let blobs_dir = self.data_dir.blobs_dir();
        let tmp_path = blobs_dir.join(".tmp-upload");
        let mut hasher = blake3::Hasher::new();
        let mut file = std::fs::File::create(&tmp_path)?;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n])?;
        }
        file.flush()?;
        drop(file);

        let hash = ContentHash::new(*hasher.finalize().as_bytes());
        let hash_hex = hash.to_string();
        let dest = self.data_dir.blob_path(&hash_hex);

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&tmp_path, &dest)?;

        Ok(hash)
    }

    /// Read a blob into a writer by streaming.
    pub fn read_blob(
        &self,
        hash: &ContentHash,
        writer: &mut dyn Write,
    ) -> Result<(), StorageError> {
        let path = self.data_dir.blob_path(&hash.to_string());
        if !path.exists() {
            return Err(StorageError::BlobNotFound(hash.to_string()));
        }
        let mut file = std::fs::File::open(&path)?;
        io::copy(&mut file, writer)?;
        Ok(())
    }

    /// Check if a blob exists on disk.
    pub fn has_blob(&self, hash: &ContentHash) -> bool {
        self.data_dir.blob_path(&hash.to_string()).exists()
    }

    /// Delete a blob from disk.
    pub fn delete_blob(&self, hash: &ContentHash) -> Result<(), StorageError> {
        let path = self.data_dir.blob_path(&hash.to_string());
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Store tessera metadata in SQLite.
    pub fn store_tessera(&self, tessera: &Tessera) -> Result<(), StorageError> {
        self.db.execute(
            "INSERT OR REPLACE INTO tesseras (hash, author, name, visibility, created_at, signature, total_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                tessera.hash.to_string(),
                hex::encode(&tessera.author),
                tessera.name,
                tessera.visibility.to_string(),
                tessera.created_at.to_rfc3339(),
                tessera.signature,
                tessera.memories.iter().map(|m| m.size).sum::<u64>() as i64,
            ],
        )?;

        for memory in &tessera.memories {
            self.db.execute(
                "INSERT INTO memories (tessera_hash, filename, media_type, size, blob_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    tessera.hash.to_string(),
                    memory.filename,
                    format!("{:?}", memory.media_type),
                    memory.size as i64,
                    memory.blob_hash.to_string(),
                ],
            )?;
        }

        Ok(())
    }

    /// Find a tessera by hash.
    pub fn find_tessera(&self, hash: &ContentHash) -> Result<Option<Tessera>, StorageError> {
        let hash_str = hash.to_string();
        let mut stmt = self.db.prepare(
            "SELECT hash, author, name, visibility, created_at, signature FROM tesseras WHERE hash = ?1",
        )?;

        let tessera = stmt.query_row(rusqlite::params![hash_str], |row| {
            let hash_s: String = row.get(0)?;
            let author_hex: String = row.get(1)?;
            let name: Option<String> = row.get(2)?;
            let vis_s: String = row.get(3)?;
            let created_s: String = row.get(4)?;
            let signature: Vec<u8> = row.get(5)?;

            Ok((hash_s, author_hex, name, vis_s, created_s, signature))
        });

        let (hash_s, author_hex, name, vis_s, created_s, signature) = match tessera {
            Ok(t) => t,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let memories = self.find_memories(&hash_s)?;

        Ok(Some(Tessera {
            hash: hash_s
                .parse()
                .map_err(|e| StorageError::Data(format!("{e}")))?,
            author: hex::decode(&author_hex).map_err(|e| StorageError::Data(format!("{e}")))?,
            signature,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_s)
                .map_err(|e| StorageError::Data(format!("{e}")))?
                .with_timezone(&chrono::Utc),
            name,
            visibility: vis_s.parse().map_err(StorageError::Data)?,
            memories,
        }))
    }

    fn find_memories(&self, tessera_hash: &str) -> Result<Vec<Memory>, StorageError> {
        let mut stmt = self.db.prepare(
            "SELECT filename, media_type, size, blob_hash FROM memories WHERE tessera_hash = ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![tessera_hash], |row| {
            let filename: String = row.get(0)?;
            let media_type_s: String = row.get(1)?;
            let size: i64 = row.get(2)?;
            let blob_hash_s: String = row.get(3)?;
            Ok((filename, media_type_s, size, blob_hash_s))
        })?;

        let mut memories = Vec::new();
        for row in rows {
            let (filename, media_type_s, size, blob_hash_s) = row?;
            let media_type = match media_type_s.as_str() {
                "Image" => MediaType::Image,
                "Audio" => MediaType::Audio,
                "Video" => MediaType::Video,
                _ => MediaType::Text,
            };
            memories.push(Memory {
                filename,
                media_type,
                size: size as u64,
                blob_hash: blob_hash_s
                    .parse()
                    .map_err(|e| StorageError::Data(format!("{e}")))?,
            });
        }
        Ok(memories)
    }

    /// List all tesseras, most recent first.
    pub fn list_tesseras(&self) -> Result<Vec<Tessera>, StorageError> {
        let mut stmt = self
            .db
            .prepare("SELECT hash FROM tesseras ORDER BY created_at DESC")?;
        let hashes: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        let mut tesseras = Vec::new();
        for hash_s in hashes {
            let hash: ContentHash = hash_s
                .parse()
                .map_err(|e| StorageError::Data(format!("{e}")))?;
            if let Some(t) = self.find_tessera(&hash)? {
                tesseras.push(t);
            }
        }
        Ok(tesseras)
    }

    /// Store a blob directly from bytes. Returns the BLAKE3 hash.
    pub fn store_blob_bytes(&self, data: &[u8]) -> Result<ContentHash, StorageError> {
        self.store_blob(&mut &data[..])
    }

    /// Read a blob into a Vec<u8>.
    pub fn read_blob_bytes(&self, hash: &ContentHash) -> Result<Vec<u8>, StorageError> {
        let mut buf = Vec::new();
        self.read_blob(hash, &mut buf)?;
        Ok(buf)
    }

    /// Store fragment metadata in SQLite. The fragment data itself is stored as a blob.
    pub fn store_fragment(
        &self,
        blob_hash: &ContentHash,
        meta: &FragmentMeta,
    ) -> Result<(), StorageError> {
        self.db.execute(
            "INSERT OR REPLACE INTO fragments
             (blob_hash, fragment_index, fragment_hash, shard_size, original_size, data_shards, parity_shards)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                blob_hash.to_string(),
                meta.fragment_index as i64,
                meta.fragment_hash.to_string(),
                meta.shard_size as i64,
                meta.original_size as i64,
                meta.data_shards as i64,
                meta.parity_shards as i64,
            ],
        )?;
        Ok(())
    }

    /// Find all fragment metadata for a blob hash.
    pub fn find_fragments(
        &self,
        blob_hash: &ContentHash,
    ) -> Result<Vec<FragmentMeta>, StorageError> {
        let mut stmt = self.db.prepare(
            "SELECT fragment_index, fragment_hash, shard_size, original_size, data_shards, parity_shards
             FROM fragments WHERE blob_hash = ?1 ORDER BY fragment_index",
        )?;
        let rows = stmt.query_map(rusqlite::params![blob_hash.to_string()], |row| {
            let index: i64 = row.get(0)?;
            let hash_s: String = row.get(1)?;
            let shard_size: i64 = row.get(2)?;
            let original_size: i64 = row.get(3)?;
            let data_shards: i64 = row.get(4)?;
            let parity_shards: i64 = row.get(5)?;
            Ok((
                index,
                hash_s,
                shard_size,
                original_size,
                data_shards,
                parity_shards,
            ))
        })?;

        let mut fragments = Vec::new();
        for row in rows {
            let (index, hash_s, shard_size, original_size, data_shards, parity_shards) = row?;
            fragments.push(FragmentMeta {
                fragment_index: index as usize,
                fragment_hash: hash_s
                    .parse()
                    .map_err(|e| StorageError::Data(format!("{e}")))?,
                shard_size: shard_size as usize,
                original_size: original_size as usize,
                data_shards: data_shards as usize,
                parity_shards: parity_shards as usize,
            });
        }
        Ok(fragments)
    }

    /// Delete fragment metadata for a blob hash.
    pub fn delete_fragments(&self, blob_hash: &ContentHash) -> Result<(), StorageError> {
        self.db.execute(
            "DELETE FROM fragments WHERE blob_hash = ?1",
            rusqlite::params![blob_hash.to_string()],
        )?;
        Ok(())
    }

    /// Delete a tessera and its memories from SQLite (blobs deleted separately).
    pub fn delete_tessera(&self, hash: &ContentHash) -> Result<(), StorageError> {
        let hash_str = hash.to_string();
        self.db.execute(
            "DELETE FROM memories WHERE tessera_hash = ?1",
            rusqlite::params![hash_str],
        )?;
        self.db.execute(
            "DELETE FROM tesseras WHERE hash = ?1",
            rusqlite::params![hash_str],
        )?;
        Ok(())
    }

    // --- Peer persistence ---

    /// Save DHT routing table peers to the database.
    pub fn save_peers(&self, peers: &[PeerInfo]) -> Result<(), StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        let tx = self.db.unchecked_transaction()?;
        tx.execute("DELETE FROM peers", [])?;
        for peer in peers {
            tx.execute(
                "INSERT INTO peers (node_id, addr, last_seen) VALUES (?1, ?2, ?3)",
                rusqlite::params![peer.node_id.to_string(), peer.addr.to_string(), now],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Load persisted peers from the database.
    pub fn load_peers(&self) -> Result<Vec<PeerInfo>, StorageError> {
        let mut stmt = self.db.prepare("SELECT node_id, addr FROM peers")?;
        let peers = stmt
            .query_map([], |row| {
                let node_id_str: String = row.get(0)?;
                let addr_str: String = row.get(1)?;
                Ok((node_id_str, addr_str))
            })?
            .filter_map(|r| {
                let (nid_s, addr_s) = r.ok()?;
                let node_id: NodeId = nid_s.parse().ok()?;
                let addr: SocketAddr = addr_s.parse().ok()?;
                Some(PeerInfo { node_id, addr })
            })
            .collect();
        Ok(peers)
    }

    // --- Reciprocity ledger ---

    /// Record bytes stored for a peer (we store their fragments).
    pub fn record_bytes_stored(
        &self,
        peer_node_id: &NodeId,
        bytes: u64,
    ) -> Result<(), StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.db.execute(
            "INSERT INTO reciprocity_ledger (peer_node_id, bytes_stored, bytes_served, last_updated)
             VALUES (?1, ?2, 0, ?3)
             ON CONFLICT(peer_node_id) DO UPDATE SET
                bytes_stored = bytes_stored + excluded.bytes_stored,
                last_updated = excluded.last_updated",
            rusqlite::params![peer_node_id.to_string(), bytes as i64, now],
        )?;
        Ok(())
    }

    /// Record bytes served to a peer (they fetched our fragments).
    pub fn record_bytes_served(
        &self,
        peer_node_id: &NodeId,
        bytes: u64,
    ) -> Result<(), StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.db.execute(
            "INSERT INTO reciprocity_ledger (peer_node_id, bytes_stored, bytes_served, last_updated)
             VALUES (?1, 0, ?2, ?3)
             ON CONFLICT(peer_node_id) DO UPDATE SET
                bytes_served = bytes_served + excluded.bytes_served,
                last_updated = excluded.last_updated",
            rusqlite::params![peer_node_id.to_string(), bytes as i64, now],
        )?;
        Ok(())
    }

    /// Get the reciprocity balance for a peer.
    /// Positive means they owe us (we stored more for them than they served us).
    pub fn get_reciprocity_balance(&self, peer_node_id: &NodeId) -> Result<i64, StorageError> {
        let result = self.db.query_row(
            "SELECT bytes_stored - bytes_served FROM reciprocity_ledger WHERE peer_node_id = ?1",
            rusqlite::params![peer_node_id.to_string()],
            |row| row.get(0),
        );
        match result {
            Ok(balance) => Ok(balance),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(e.into()),
        }
    }

    /// List all reciprocity ledger entries.
    pub fn list_reciprocity(&self) -> Result<Vec<ReciprocityEntry>, StorageError> {
        let mut stmt = self.db.prepare(
            "SELECT peer_node_id, bytes_stored, bytes_served, last_updated FROM reciprocity_ledger",
        )?;
        let entries = stmt
            .query_map([], |row| {
                Ok(ReciprocityEntry {
                    peer_node_id: row.get(0)?,
                    bytes_stored: row.get::<_, i64>(1)? as u64,
                    bytes_served: row.get::<_, i64>(2)? as u64,
                    last_updated: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }
}

/// Fragment metadata stored in SQLite.
#[derive(Debug, Clone)]
pub struct FragmentMeta {
    pub fragment_index: usize,
    pub fragment_hash: ContentHash,
    pub shard_size: usize,
    pub original_size: usize,
    pub data_shards: usize,
    pub parity_shards: usize,
}

/// A reciprocity ledger entry showing bilateral storage balance with a peer.
#[derive(Debug, Clone)]
pub struct ReciprocityEntry {
    pub peer_node_id: String,
    pub bytes_stored: u64,
    pub bytes_served: u64,
    pub last_updated: String,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("blob not found: {0}")]
    BlobNotFound(String),
    #[error("data error: {0}")]
    Data(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto;
    use crate::types::Visibility;

    fn test_storage() -> (tempfile::TempDir, Storage) {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = DataDir::open(tmp.path()).unwrap();
        let storage = Storage::open(data_dir).unwrap();
        (tmp, storage)
    }

    #[test]
    fn store_and_read_blob() {
        let (_tmp, storage) = test_storage();
        let data = b"hello tesseras blob";
        let hash = storage.store_blob(&mut &data[..]).unwrap();

        let mut output = Vec::new();
        storage.read_blob(&hash, &mut output).unwrap();
        assert_eq!(output, data);
    }

    #[test]
    fn blob_hash_matches_blake3() {
        let (_tmp, storage) = test_storage();
        let data = b"test content";
        let hash = storage.store_blob(&mut &data[..]).unwrap();
        assert_eq!(hash, crypto::hash_bytes(data));
    }

    #[test]
    fn has_blob_check() {
        let (_tmp, storage) = test_storage();
        let hash = storage.store_blob(&mut &b"data"[..]).unwrap();
        assert!(storage.has_blob(&hash));

        let fake = crypto::hash_bytes(b"nonexistent");
        assert!(!storage.has_blob(&fake));
    }

    #[test]
    fn delete_blob() {
        let (_tmp, storage) = test_storage();
        let hash = storage.store_blob(&mut &b"delete me"[..]).unwrap();
        assert!(storage.has_blob(&hash));

        storage.delete_blob(&hash).unwrap();
        assert!(!storage.has_blob(&hash));
    }

    #[test]
    fn store_and_find_tessera() {
        let (_tmp, storage) = test_storage();
        let blob_hash = storage.store_blob(&mut &b"photo data"[..]).unwrap();

        let tessera = Tessera {
            hash: crypto::hash_bytes(b"tessera id"),
            author: vec![0u8; 32],
            signature: vec![0u8; 64],
            created_at: chrono::Utc::now(),
            name: Some("Test".into()),
            visibility: Visibility::Public,
            memories: vec![Memory {
                filename: "photo.jpg".into(),
                media_type: MediaType::Image,
                size: 10,
                blob_hash,
            }],
        };

        storage.store_tessera(&tessera).unwrap();
        let found = storage.find_tessera(&tessera.hash).unwrap().unwrap();
        assert_eq!(found.name, Some("Test".into()));
        assert_eq!(found.memories.len(), 1);
        assert_eq!(found.memories[0].filename, "photo.jpg");
    }

    #[test]
    fn list_tesseras_most_recent_first() {
        let (_tmp, storage) = test_storage();

        for i in 0..3 {
            let tessera = Tessera {
                hash: crypto::hash_bytes(format!("tessera-{i}").as_bytes()),
                author: vec![0u8; 32],
                signature: vec![0u8; 64],
                created_at: chrono::Utc::now() + chrono::Duration::seconds(i as i64),
                name: Some(format!("Tessera {i}")),
                visibility: Visibility::Public,
                memories: vec![],
            };
            storage.store_tessera(&tessera).unwrap();
        }

        let list = storage.list_tesseras().unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].name, Some("Tessera 2".into()));
    }

    #[test]
    fn delete_tessera() {
        let (_tmp, storage) = test_storage();
        let tessera = Tessera {
            hash: crypto::hash_bytes(b"to delete"),
            author: vec![0u8; 32],
            signature: vec![0u8; 64],
            created_at: chrono::Utc::now(),
            name: None,
            visibility: Visibility::Private,
            memories: vec![],
        };
        storage.store_tessera(&tessera).unwrap();
        storage.delete_tessera(&tessera.hash).unwrap();
        assert!(storage.find_tessera(&tessera.hash).unwrap().is_none());
    }

    #[test]
    fn read_nonexistent_blob_fails() {
        let (_tmp, storage) = test_storage();
        let fake = crypto::hash_bytes(b"nope");
        let mut buf = Vec::new();
        assert!(storage.read_blob(&fake, &mut buf).is_err());
    }

    #[test]
    fn store_and_find_fragments() {
        let (_tmp, storage) = test_storage();
        let blob_hash = crypto::hash_bytes(b"original blob");

        // Store 5 fragment metadata entries
        for i in 0..5 {
            let frag_hash = crypto::hash_bytes(format!("fragment-{i}").as_bytes());
            let meta = FragmentMeta {
                fragment_index: i,
                fragment_hash: frag_hash,
                shard_size: 128,
                original_size: 300,
                data_shards: 3,
                parity_shards: 2,
            };
            storage.store_fragment(&blob_hash, &meta).unwrap();
        }

        let fragments = storage.find_fragments(&blob_hash).unwrap();
        assert_eq!(fragments.len(), 5);
        assert_eq!(fragments[0].fragment_index, 0);
        assert_eq!(fragments[4].fragment_index, 4);
        assert_eq!(fragments[0].shard_size, 128);
        assert_eq!(fragments[0].original_size, 300);
        assert_eq!(fragments[0].data_shards, 3);
        assert_eq!(fragments[0].parity_shards, 2);
    }

    #[test]
    fn delete_fragments() {
        let (_tmp, storage) = test_storage();
        let blob_hash = crypto::hash_bytes(b"blob to delete frags");

        for i in 0..3 {
            let frag_hash = crypto::hash_bytes(format!("frag-{i}").as_bytes());
            let meta = FragmentMeta {
                fragment_index: i,
                fragment_hash: frag_hash,
                shard_size: 64,
                original_size: 100,
                data_shards: 3,
                parity_shards: 2,
            };
            storage.store_fragment(&blob_hash, &meta).unwrap();
        }

        storage.delete_fragments(&blob_hash).unwrap();
        let fragments = storage.find_fragments(&blob_hash).unwrap();
        assert!(fragments.is_empty());
    }

    #[test]
    fn store_and_read_blob_bytes() {
        let (_tmp, storage) = test_storage();
        let data = b"direct bytes storage";
        let hash = storage.store_blob_bytes(data).unwrap();
        let read_back = storage.read_blob_bytes(&hash).unwrap();
        assert_eq!(read_back, data);
    }

    #[test]
    fn versioned_migration_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = DataDir::open(tmp.path()).unwrap();

        // Open twice — second time should detect version and skip migrations
        let _s1 = Storage::open(data_dir.clone()).unwrap();
        drop(_s1);
        let s2 = Storage::open(data_dir).unwrap();

        // Verify schema is usable
        s2.store_blob_bytes(b"works").unwrap();
    }

    #[test]
    fn save_and_load_peers() {
        let (_tmp, storage) = test_storage();
        let peers = vec![
            PeerInfo {
                node_id: NodeId::new([1u8; 32]),
                addr: "127.0.0.1:4433".parse().unwrap(),
            },
            PeerInfo {
                node_id: NodeId::new([2u8; 32]),
                addr: "192.168.1.1:5000".parse().unwrap(),
            },
        ];

        storage.save_peers(&peers).unwrap();
        let loaded = storage.load_peers().unwrap();
        assert_eq!(loaded.len(), 2);

        // Check that node IDs match (order may differ)
        let ids: Vec<String> = loaded.iter().map(|p| p.node_id.to_string()).collect();
        assert!(ids.contains(&peers[0].node_id.to_string()));
        assert!(ids.contains(&peers[1].node_id.to_string()));
    }

    #[test]
    fn save_peers_replaces_old() {
        let (_tmp, storage) = test_storage();
        let peers1 = vec![PeerInfo {
            node_id: NodeId::new([1u8; 32]),
            addr: "127.0.0.1:4433".parse().unwrap(),
        }];
        storage.save_peers(&peers1).unwrap();

        let peers2 = vec![PeerInfo {
            node_id: NodeId::new([2u8; 32]),
            addr: "10.0.0.1:5000".parse().unwrap(),
        }];
        storage.save_peers(&peers2).unwrap();

        let loaded = storage.load_peers().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].node_id.to_string(), peers2[0].node_id.to_string());
    }

    #[test]
    fn reciprocity_ledger_basic() {
        let (_tmp, storage) = test_storage();
        let peer = NodeId::new([42u8; 32]);

        // Initially zero balance
        assert_eq!(storage.get_reciprocity_balance(&peer).unwrap(), 0);

        // Record bytes stored (we store for them)
        storage.record_bytes_stored(&peer, 1000).unwrap();
        assert_eq!(storage.get_reciprocity_balance(&peer).unwrap(), 1000);

        // Record bytes served (they fetch from us)
        storage.record_bytes_served(&peer, 400).unwrap();
        assert_eq!(storage.get_reciprocity_balance(&peer).unwrap(), 600);

        // Accumulates
        storage.record_bytes_stored(&peer, 200).unwrap();
        assert_eq!(storage.get_reciprocity_balance(&peer).unwrap(), 800);
    }

    #[test]
    fn list_reciprocity_entries() {
        let (_tmp, storage) = test_storage();
        let peer1 = NodeId::new([1u8; 32]);
        let peer2 = NodeId::new([2u8; 32]);

        storage.record_bytes_stored(&peer1, 500).unwrap();
        storage.record_bytes_served(&peer2, 300).unwrap();

        let entries = storage.list_reciprocity().unwrap();
        assert_eq!(entries.len(), 2);

        let e1 = entries.iter().find(|e| e.bytes_stored == 500).unwrap();
        assert_eq!(e1.bytes_served, 0);
        let e2 = entries.iter().find(|e| e.bytes_served == 300).unwrap();
        assert_eq!(e2.bytes_stored, 0);
    }
}
