use std::str::FromStr;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tesseras_core::ports::TombstoneRepository;
use tesseras_core::tombstone::Tombstone;
use tesseras_core::types::ContentHash;
use tesseras_core::CoreError;

pub struct SqliteTombstoneRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTombstoneRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl TombstoneRepository for SqliteTombstoneRepository {
    fn store(&self, tombstone: &Tombstone) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tombstones (hash, retracted_at, creator_pubkey, ed25519_signature, mldsa_signature) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                tombstone.hash.to_string(),
                tombstone.retracted_at.to_rfc3339(),
                tombstone.creator_pubkey,
                tombstone.ed25519_signature,
                tombstone.mldsa_signature,
            ],
        ).map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn find(&self, hash: &ContentHash) -> Result<Option<Tombstone>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT hash, retracted_at, creator_pubkey, ed25519_signature, mldsa_signature FROM tombstones WHERE hash = ?1"
        ).map_err(|e| CoreError::Database(e.to_string()))?;

        let result = stmt.query_row(
            rusqlite::params![hash.to_string()],
            |row: &rusqlite::Row| {
                Ok(Tombstone {
                    hash: ContentHash::from_str(&row.get::<_, String>(0)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                    retracted_at: chrono::DateTime::parse_from_rfc3339(
                        &row.get::<_, String>(1)?,
                    )
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                    creator_pubkey: row.get(2)?,
                    ed25519_signature: row.get(3)?,
                    mldsa_signature: row.get(4)?,
                })
            },
        );

        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Database(e.to_string())),
        }
    }

    fn exists(&self, hash: &ContentHash) -> Result<bool, CoreError> {
        let conn = self.conn.lock().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tombstones WHERE hash = ?1)",
                rusqlite::params![hash.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(exists)
    }

    fn list(&self) -> Result<Vec<Tombstone>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT hash, retracted_at, creator_pubkey, ed25519_signature, mldsa_signature FROM tombstones ORDER BY retracted_at DESC"
        ).map_err(|e| CoreError::Database(e.to_string()))?;

        let tombstones = stmt
            .query_map([], |row: &rusqlite::Row| {
                Ok(Tombstone {
                    hash: ContentHash::from_str(&row.get::<_, String>(0)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                    retracted_at: chrono::DateTime::parse_from_rfc3339(
                        &row.get::<_, String>(1)?,
                    )
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                    creator_pubkey: row.get(2)?,
                    ed25519_signature: row.get(3)?,
                    mldsa_signature: row.get(4)?,
                })
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        tombstones
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        crate::run_migrations(&conn).unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn store_and_find_tombstone() {
        let conn = setup();
        let repo = SqliteTombstoneRepository::new(conn);
        let t = Tombstone {
            hash: ContentHash::new([0xab; 32]),
            retracted_at: chrono::Utc::now(),
            creator_pubkey: "deadbeef".repeat(4),
            ed25519_signature: vec![0x01; 64],
            mldsa_signature: vec![0x02; 128],
        };
        repo.store(&t).unwrap();
        let found = repo.find(&t.hash).unwrap().unwrap();
        assert_eq!(found.hash, t.hash);
        assert_eq!(found.creator_pubkey, t.creator_pubkey);
    }

    #[test]
    fn exists_returns_false_for_missing() {
        let conn = setup();
        let repo = SqliteTombstoneRepository::new(conn);
        assert!(!repo.exists(&ContentHash::new([0xff; 32])).unwrap());
    }

    #[test]
    fn exists_returns_true_after_store() {
        let conn = setup();
        let repo = SqliteTombstoneRepository::new(conn);
        let t = Tombstone {
            hash: ContentHash::new([0xab; 32]),
            retracted_at: chrono::Utc::now(),
            creator_pubkey: "test".to_string(),
            ed25519_signature: vec![],
            mldsa_signature: vec![],
        };
        repo.store(&t).unwrap();
        assert!(repo.exists(&t.hash).unwrap());
    }
}
