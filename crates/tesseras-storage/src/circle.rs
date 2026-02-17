use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tesseras_core::CoreError;
use tesseras_core::ports::{CircleMemberRecord, CircleRecord, CircleRepository};

pub struct SqliteCircleRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteCircleRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl CircleRepository for SqliteCircleRepository {
    fn create_circle(&self, name: &str, symmetric_key: &[u8]) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO circles (name, symmetric_key, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, symmetric_key, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn delete_circle(&self, name: &str) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM circles WHERE name = ?1",
            rusqlite::params![name],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn list_circles(&self) -> Result<Vec<CircleRecord>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name, symmetric_key, created_at FROM circles ORDER BY name")
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row: &rusqlite::Row| {
                Ok(CircleRecord {
                    name: row.get(0)?,
                    symmetric_key: row.get(1)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Database(e.to_string()))
    }

    fn find_circle(&self, name: &str) -> Result<Option<CircleRecord>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT name, symmetric_key, created_at FROM circles WHERE name = ?1",
            rusqlite::params![name],
            |row: &rusqlite::Row| {
                Ok(CircleRecord {
                    name: row.get(0)?,
                    symmetric_key: row.get(1)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            },
        );

        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Database(e.to_string())),
        }
    }

    fn add_member(
        &self,
        circle: &str,
        alias: &str,
        pubkey: &str,
        wrapped_key: &[u8],
    ) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO circle_members (circle_name, alias, pubkey, wrapped_key, added_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![circle, alias, pubkey, wrapped_key, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn remove_member(&self, circle: &str, alias: &str) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM circle_members WHERE circle_name = ?1 AND alias = ?2",
            rusqlite::params![circle, alias],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn list_members(&self, circle: &str) -> Result<Vec<CircleMemberRecord>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT circle_name, alias, pubkey, wrapped_key, added_at FROM circle_members WHERE circle_name = ?1 ORDER BY alias",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![circle], |row: &rusqlite::Row| {
                Ok(CircleMemberRecord {
                    circle_name: row.get(0)?,
                    alias: row.get(1)?,
                    pubkey: row.get(2)?,
                    wrapped_key: row.get(3)?,
                    added_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Database(e.to_string()))
    }

    fn find_member_wrapped_key(
        &self,
        circle: &str,
        pubkey: &str,
    ) -> Result<Option<Vec<u8>>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT wrapped_key FROM circle_members WHERE circle_name = ?1 AND pubkey = ?2",
            rusqlite::params![circle, pubkey],
            |row| row.get(0),
        );

        match result {
            Ok(key) => Ok(Some(key)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Database(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Arc<Mutex<Connection>> {
        let conn =
            crate::database::open_in_memory(&crate::database::StorageConfig::default()).unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn create_and_find_circle() {
        let conn = setup();
        let repo = SqliteCircleRepository::new(conn);
        repo.create_circle("family", &[0x42; 32]).unwrap();
        let found = repo.find_circle("family").unwrap().unwrap();
        assert_eq!(found.name, "family");
        assert_eq!(found.symmetric_key, vec![0x42; 32]);
    }

    #[test]
    fn list_circles() {
        let conn = setup();
        let repo = SqliteCircleRepository::new(conn);
        repo.create_circle("alpha", &[0x01; 32]).unwrap();
        repo.create_circle("beta", &[0x02; 32]).unwrap();
        let circles = repo.list_circles().unwrap();
        assert_eq!(circles.len(), 2);
        assert_eq!(circles[0].name, "alpha");
        assert_eq!(circles[1].name, "beta");
    }

    #[test]
    fn delete_circle_cascades_members() {
        let conn = setup();
        let repo = SqliteCircleRepository::new(conn);
        repo.create_circle("family", &[0x42; 32]).unwrap();
        repo.add_member("family", "alice", "pubkey_alice", &[0x01; 48])
            .unwrap();
        repo.delete_circle("family").unwrap();
        assert!(repo.find_circle("family").unwrap().is_none());
        let members = repo.list_members("family").unwrap();
        assert!(members.is_empty());
    }

    #[test]
    fn add_and_list_members() {
        let conn = setup();
        let repo = SqliteCircleRepository::new(conn);
        repo.create_circle("friends", &[0x42; 32]).unwrap();
        repo.add_member("friends", "alice", "pk_a", &[0x01; 48])
            .unwrap();
        repo.add_member("friends", "bob", "pk_b", &[0x02; 48])
            .unwrap();
        let members = repo.list_members("friends").unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].alias, "alice");
        assert_eq!(members[1].alias, "bob");
    }

    #[test]
    fn remove_member() {
        let conn = setup();
        let repo = SqliteCircleRepository::new(conn);
        repo.create_circle("friends", &[0x42; 32]).unwrap();
        repo.add_member("friends", "alice", "pk_a", &[0x01; 48])
            .unwrap();
        repo.remove_member("friends", "alice").unwrap();
        let members = repo.list_members("friends").unwrap();
        assert!(members.is_empty());
    }

    #[test]
    fn find_member_wrapped_key() {
        let conn = setup();
        let repo = SqliteCircleRepository::new(conn);
        repo.create_circle("family", &[0x42; 32]).unwrap();
        repo.add_member("family", "alice", "pk_a", &[0x99; 48])
            .unwrap();
        let key = repo
            .find_member_wrapped_key("family", "pk_a")
            .unwrap()
            .unwrap();
        assert_eq!(key, vec![0x99; 48]);
    }

    #[test]
    fn find_missing_returns_none() {
        let conn = setup();
        let repo = SqliteCircleRepository::new(conn);
        assert!(repo.find_circle("nonexistent").unwrap().is_none());
        assert!(
            repo.find_member_wrapped_key("nonexistent", "pk")
                .unwrap()
                .is_none()
        );
    }
}
