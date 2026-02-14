use std::str::FromStr;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tesseras_core::ports::{MemoryRecord, MemoryRepository, TesseraRecord, TesseraRepository};
use tesseras_core::{ContentHash, CoreError};

pub struct SqliteTesseraRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTesseraRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl TesseraRepository for SqliteTesseraRepository {
    fn store(&self, tessera: &TesseraRecord) -> Result<(), CoreError> {
        let hash = tessera.hash.to_string();
        let created_at = tessera.created_at.to_rfc3339();
        let sealed_until = tessera.sealed_until.map(|dt| dt.to_rfc3339());
        let size_bytes = tessera.size_bytes as i64;
        let memory_count = tessera.memory_count as i32;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tesseras (hash, creator_pubkey, created_at, size_bytes, memory_count, visibility, sealed_until, is_mine)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                hash,
                tessera.creator_pubkey,
                created_at,
                size_bytes,
                memory_count,
                tessera.visibility,
                sealed_until,
                tessera.is_mine,
            ],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<TesseraRecord>, CoreError> {
        let hash_str = hash.to_string();
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT hash, creator_pubkey, created_at, size_bytes, memory_count, visibility, sealed_until, is_mine
                 FROM tesseras WHERE hash = ?1",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let mut rows = stmt
            .query_map(rusqlite::params![hash_str], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i32>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, bool>(7)?,
                ))
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        match rows.next() {
            Some(Ok((hash, creator_pubkey, created_at, size_bytes, memory_count, visibility, sealed_until, is_mine))) => {
                Ok(Some(TesseraRecord {
                    hash: ContentHash::from_str(&hash)
                        .map_err(|e| CoreError::Database(e.to_string()))?,
                    creator_pubkey,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| CoreError::Database(e.to_string()))?
                        .with_timezone(&chrono::Utc),
                    size_bytes: size_bytes as u64,
                    memory_count: memory_count as u32,
                    visibility,
                    sealed_until: sealed_until
                        .map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                        })
                        .transpose()
                        .map_err(|e| CoreError::Database(e.to_string()))?,
                    is_mine,
                }))
            }
            Some(Err(e)) => Err(CoreError::Database(e.to_string())),
            None => Ok(None),
        }
    }

    fn list(&self) -> Result<Vec<TesseraRecord>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT hash, creator_pubkey, created_at, size_bytes, memory_count, visibility, sealed_until, is_mine
                 FROM tesseras ORDER BY created_at DESC",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i32>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, bool>(7)?,
                ))
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                let (hash, creator_pubkey, created_at, size_bytes, memory_count, visibility, sealed_until, is_mine) =
                    r.map_err(|e| CoreError::Database(e.to_string()))?;
                Ok(TesseraRecord {
                    hash: ContentHash::from_str(&hash)
                        .map_err(|e| CoreError::Database(e.to_string()))?,
                    creator_pubkey,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| CoreError::Database(e.to_string()))?
                        .with_timezone(&chrono::Utc),
                    size_bytes: size_bytes as u64,
                    memory_count: memory_count as u32,
                    visibility,
                    sealed_until: sealed_until
                        .map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                        })
                        .transpose()
                        .map_err(|e| CoreError::Database(e.to_string()))?,
                    is_mine,
                })
            })
            .collect()
    }

    fn delete(&self, hash: &ContentHash) -> Result<(), CoreError> {
        let hash_str = hash.to_string();
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tesseras WHERE hash = ?1", rusqlite::params![hash_str])
            .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn exists(&self, hash: &ContentHash) -> Result<bool, CoreError> {
        let hash_str = hash.to_string();
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT 1 FROM tesseras WHERE hash = ?1")
            .map_err(|e| CoreError::Database(e.to_string()))?;
        let exists = stmt
            .exists(rusqlite::params![hash_str])
            .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(exists)
    }
}

pub struct SqliteMemoryRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMemoryRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl MemoryRepository for SqliteMemoryRepository {
    fn store(&self, memory: &MemoryRecord) -> Result<(), CoreError> {
        let hash = memory.hash.to_string();
        let tessera_hash = memory.tessera_hash.to_string();
        let created_at = memory.created_at.to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO memories (hash, tessera_hash, memory_type, media_path, context_path, meta_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                hash,
                tessera_hash,
                memory.memory_type,
                memory.media_path,
                memory.context_path,
                memory.meta_json,
                created_at,
            ],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<MemoryRecord>, CoreError> {
        let hash_str = hash.to_string();
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT hash, tessera_hash, memory_type, media_path, context_path, meta_json, created_at
                 FROM memories WHERE hash = ?1",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let mut rows = stmt
            .query_map(rusqlite::params![hash_str], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        match rows.next() {
            Some(Ok((hash, tessera_hash, memory_type, media_path, context_path, meta_json, created_at))) => {
                Ok(Some(MemoryRecord {
                    hash: ContentHash::from_str(&hash)
                        .map_err(|e| CoreError::Database(e.to_string()))?,
                    tessera_hash: ContentHash::from_str(&tessera_hash)
                        .map_err(|e| CoreError::Database(e.to_string()))?,
                    memory_type,
                    media_path,
                    context_path,
                    meta_json,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| CoreError::Database(e.to_string()))?
                        .with_timezone(&chrono::Utc),
                }))
            }
            Some(Err(e)) => Err(CoreError::Database(e.to_string())),
            None => Ok(None),
        }
    }

    fn list_by_tessera(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<Vec<MemoryRecord>, CoreError> {
        let hash_str = tessera_hash.to_string();
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT hash, tessera_hash, memory_type, media_path, context_path, meta_json, created_at
                 FROM memories WHERE tessera_hash = ?1",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![hash_str], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                let (hash, tessera_hash, memory_type, media_path, context_path, meta_json, created_at) =
                    r.map_err(|e| CoreError::Database(e.to_string()))?;
                Ok(MemoryRecord {
                    hash: ContentHash::from_str(&hash)
                        .map_err(|e| CoreError::Database(e.to_string()))?,
                    tessera_hash: ContentHash::from_str(&tessera_hash)
                        .map_err(|e| CoreError::Database(e.to_string()))?,
                    memory_type,
                    media_path,
                    context_path,
                    meta_json,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| CoreError::Database(e.to_string()))?
                        .with_timezone(&chrono::Utc),
                })
            })
            .collect()
    }

    fn delete(&self, hash: &ContentHash) -> Result<(), CoreError> {
        let hash_str = hash.to_string();
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM memories WHERE hash = ?1", rusqlite::params![hash_str])
            .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_migrations;
    use tesseras_core::ContentHash;

    fn setup_conn() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        Arc::new(Mutex::new(conn))
    }

    fn sample_tessera_record() -> TesseraRecord {
        TesseraRecord {
            hash: ContentHash::new([0x01; 32]),
            creator_pubkey: "aa".repeat(32),
            created_at: chrono::Utc::now(),
            size_bytes: 1024,
            memory_count: 1,
            visibility: "public".to_string(),
            sealed_until: None,
            is_mine: true,
        }
    }

    fn sample_memory_record(tessera_hash: ContentHash) -> MemoryRecord {
        MemoryRecord {
            hash: ContentHash::new([0x02; 32]),
            tessera_hash,
            memory_type: "moment".to_string(),
            media_path: "media.jpg".to_string(),
            context_path: Some("context.txt".to_string()),
            meta_json: Some("{}".to_string()),
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn tessera_store_and_find() {
        let conn = setup_conn();
        let repo = SqliteTesseraRepository::new(conn);
        let record = sample_tessera_record();
        repo.store(&record).unwrap();
        let found = repo.find_by_hash(&record.hash).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().hash, record.hash);
    }

    #[test]
    fn tessera_find_nonexistent() {
        let conn = setup_conn();
        let repo = SqliteTesseraRepository::new(conn);
        let hash = ContentHash::new([0xff; 32]);
        let found = repo.find_by_hash(&hash).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn tessera_list() {
        let conn = setup_conn();
        let repo = SqliteTesseraRepository::new(conn);
        let record = sample_tessera_record();
        repo.store(&record).unwrap();
        let all = repo.list().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn tessera_delete() {
        let conn = setup_conn();
        let repo = SqliteTesseraRepository::new(conn);
        let record = sample_tessera_record();
        repo.store(&record).unwrap();
        repo.delete(&record.hash).unwrap();
        assert!(!repo.exists(&record.hash).unwrap());
    }

    #[test]
    fn memory_cascade_delete() {
        let conn = setup_conn();
        // Enable foreign keys for cascade to work
        conn.lock()
            .unwrap()
            .execute_batch("PRAGMA foreign_keys = ON")
            .unwrap();
        let t_repo = SqliteTesseraRepository::new(conn.clone());
        let m_repo = SqliteMemoryRepository::new(conn);
        let tessera = sample_tessera_record();
        let memory = sample_memory_record(tessera.hash);
        t_repo.store(&tessera).unwrap();
        m_repo.store(&memory).unwrap();
        // Delete tessera — memory should cascade
        t_repo.delete(&tessera.hash).unwrap();
        let found = m_repo.find_by_hash(&memory.hash).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn memory_list_by_tessera() {
        let conn = setup_conn();
        let t_repo = SqliteTesseraRepository::new(conn.clone());
        let m_repo = SqliteMemoryRepository::new(conn);
        let tessera = sample_tessera_record();
        t_repo.store(&tessera).unwrap();
        let mem = sample_memory_record(tessera.hash);
        m_repo.store(&mem).unwrap();
        let list = m_repo.list_by_tessera(&tessera.hash).unwrap();
        assert_eq!(list.len(), 1);
    }
}
