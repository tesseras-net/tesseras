use std::io::{self, Read, Write};

use rusqlite::Connection;

use crate::config::DataDir;
use crate::types::{ContentHash, MediaType, Memory, Tessera};

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
        storage.migrate()?;
        Ok(storage)
    }

    fn migrate(&self) -> Result<(), StorageError> {
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
                addresses   TEXT NOT NULL,
                last_seen   TEXT NOT NULL
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
}
