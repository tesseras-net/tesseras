use std::str::FromStr;

use async_trait::async_trait;
use sqlx::SqlitePool;
use tesseras_core::ports::{MemoryRecord, MemoryRepository, TesseraRecord, TesseraRepository};
use tesseras_core::{ContentHash, CoreError};

pub struct SqliteTesseraRepository {
    pool: SqlitePool,
}

impl SqliteTesseraRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TesseraRepository for SqliteTesseraRepository {
    async fn store(&self, tessera: &TesseraRecord) -> Result<(), CoreError> {
        let hash = tessera.hash.to_string();
        let created_at = tessera.created_at.to_rfc3339();
        let sealed_until = tessera.sealed_until.map(|dt| dt.to_rfc3339());
        let size_bytes = tessera.size_bytes as i64;
        let memory_count = tessera.memory_count as i32;
        sqlx::query(
            "INSERT OR REPLACE INTO tesseras (hash, creator_pubkey, created_at, size_bytes, memory_count, visibility, sealed_until, is_mine)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&hash)
        .bind(&tessera.creator_pubkey)
        .bind(&created_at)
        .bind(size_bytes)
        .bind(memory_count)
        .bind(&tessera.visibility)
        .bind(&sealed_until)
        .bind(tessera.is_mine)
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;
        Ok(())
    }

    async fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<TesseraRecord>, CoreError> {
        let hash_str = hash.to_string();
        let row: Option<(String, String, String, i64, i32, String, Option<String>, bool)> =
            sqlx::query_as(
                "SELECT hash, creator_pubkey, created_at, size_bytes, memory_count, visibility, sealed_until, is_mine
                 FROM tesseras WHERE hash = ?"
            )
            .bind(&hash_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;

        match row {
            Some((
                hash,
                creator_pubkey,
                created_at,
                size_bytes,
                memory_count,
                visibility,
                sealed_until,
                is_mine,
            )) => Ok(Some(TesseraRecord {
                hash: ContentHash::from_str(&hash)
                    .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?,
                creator_pubkey,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?
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
                    .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?,
                is_mine,
            })),
            None => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<TesseraRecord>, CoreError> {
        let rows: Vec<(String, String, String, i64, i32, String, Option<String>, bool)> =
            sqlx::query_as(
                "SELECT hash, creator_pubkey, created_at, size_bytes, memory_count, visibility, sealed_until, is_mine
                 FROM tesseras ORDER BY created_at DESC"
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;

        rows.into_iter()
            .map(
                |(
                    hash,
                    creator_pubkey,
                    created_at,
                    size_bytes,
                    memory_count,
                    visibility,
                    sealed_until,
                    is_mine,
                )| {
                    Ok(TesseraRecord {
                        hash: ContentHash::from_str(&hash)
                            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?,
                        creator_pubkey,
                        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?
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
                            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?,
                        is_mine,
                    })
                },
            )
            .collect()
    }

    async fn delete(&self, hash: &ContentHash) -> Result<(), CoreError> {
        let hash_str = hash.to_string();
        sqlx::query("DELETE FROM tesseras WHERE hash = ?")
            .bind(&hash_str)
            .execute(&self.pool)
            .await
            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;
        Ok(())
    }

    async fn exists(&self, hash: &ContentHash) -> Result<bool, CoreError> {
        let hash_str = hash.to_string();
        let row: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM tesseras WHERE hash = ?")
            .bind(&hash_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;
        Ok(row.is_some())
    }
}

pub struct SqliteMemoryRepository {
    pool: SqlitePool,
}

impl SqliteMemoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MemoryRepository for SqliteMemoryRepository {
    async fn store(&self, memory: &MemoryRecord) -> Result<(), CoreError> {
        let hash = memory.hash.to_string();
        let tessera_hash = memory.tessera_hash.to_string();
        let created_at = memory.created_at.to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO memories (hash, tessera_hash, memory_type, media_path, context_path, meta_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&hash)
        .bind(&tessera_hash)
        .bind(&memory.memory_type)
        .bind(&memory.media_path)
        .bind(&memory.context_path)
        .bind(&memory.meta_json)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;
        Ok(())
    }

    async fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<MemoryRecord>, CoreError> {
        let hash_str = hash.to_string();
        let row: Option<(String, String, String, String, Option<String>, Option<String>, String)> =
            sqlx::query_as(
                "SELECT hash, tessera_hash, memory_type, media_path, context_path, meta_json, created_at
                 FROM memories WHERE hash = ?"
            )
            .bind(&hash_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;

        match row {
            Some((
                hash,
                tessera_hash,
                memory_type,
                media_path,
                context_path,
                meta_json,
                created_at,
            )) => Ok(Some(MemoryRecord {
                hash: ContentHash::from_str(&hash)
                    .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?,
                tessera_hash: ContentHash::from_str(&tessera_hash)
                    .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?,
                memory_type,
                media_path,
                context_path,
                meta_json,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?
                    .with_timezone(&chrono::Utc),
            })),
            None => Ok(None),
        }
    }

    async fn list_by_tessera(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<Vec<MemoryRecord>, CoreError> {
        let hash_str = tessera_hash.to_string();
        let rows: Vec<(String, String, String, String, Option<String>, Option<String>, String)> =
            sqlx::query_as(
                "SELECT hash, tessera_hash, memory_type, media_path, context_path, meta_json, created_at
                 FROM memories WHERE tessera_hash = ?"
            )
            .bind(&hash_str)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;

        rows.into_iter()
            .map(
                |(
                    hash,
                    tessera_hash,
                    memory_type,
                    media_path,
                    context_path,
                    meta_json,
                    created_at,
                )| {
                    Ok(MemoryRecord {
                        hash: ContentHash::from_str(&hash)
                            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?,
                        tessera_hash: ContentHash::from_str(&tessera_hash)
                            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?,
                        memory_type,
                        media_path,
                        context_path,
                        meta_json,
                        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?
                            .with_timezone(&chrono::Utc),
                    })
                },
            )
            .collect()
    }

    async fn delete(&self, hash: &ContentHash) -> Result<(), CoreError> {
        let hash_str = hash.to_string();
        sqlx::query("DELETE FROM memories WHERE hash = ?")
            .bind(&hash_str)
            .execute(&self.pool)
            .await
            .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseras_core::ContentHash;

    async fn setup_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
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

    #[tokio::test]
    async fn tessera_store_and_find() {
        let pool = setup_pool().await;
        let repo = SqliteTesseraRepository::new(pool);
        let record = sample_tessera_record();
        repo.store(&record).await.unwrap();
        let found = repo.find_by_hash(&record.hash).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().hash, record.hash);
    }

    #[tokio::test]
    async fn tessera_find_nonexistent() {
        let pool = setup_pool().await;
        let repo = SqliteTesseraRepository::new(pool);
        let hash = ContentHash::new([0xff; 32]);
        let found = repo.find_by_hash(&hash).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn tessera_list() {
        let pool = setup_pool().await;
        let repo = SqliteTesseraRepository::new(pool);
        let record = sample_tessera_record();
        repo.store(&record).await.unwrap();
        let all = repo.list().await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn tessera_delete() {
        let pool = setup_pool().await;
        let repo = SqliteTesseraRepository::new(pool.clone());
        let record = sample_tessera_record();
        repo.store(&record).await.unwrap();
        repo.delete(&record.hash).await.unwrap();
        assert!(!repo.exists(&record.hash).await.unwrap());
    }

    #[tokio::test]
    async fn memory_cascade_delete() {
        let pool = setup_pool().await;
        // Enable foreign keys for cascade to work
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        let t_repo = SqliteTesseraRepository::new(pool.clone());
        let m_repo = SqliteMemoryRepository::new(pool);
        let tessera = sample_tessera_record();
        let memory = sample_memory_record(tessera.hash);
        t_repo.store(&tessera).await.unwrap();
        m_repo.store(&memory).await.unwrap();
        // Delete tessera — memory should cascade
        t_repo.delete(&tessera.hash).await.unwrap();
        let found = m_repo.find_by_hash(&memory.hash).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn memory_list_by_tessera() {
        let pool = setup_pool().await;
        let t_repo = SqliteTesseraRepository::new(pool.clone());
        let m_repo = SqliteMemoryRepository::new(pool);
        let tessera = sample_tessera_record();
        t_repo.store(&tessera).await.unwrap();
        let mem = sample_memory_record(tessera.hash);
        m_repo.store(&mem).await.unwrap();
        let list = m_repo.list_by_tessera(&tessera.hash).await.unwrap();
        assert_eq!(list.len(), 1);
    }
}
