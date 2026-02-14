use std::sync::{Arc, Mutex};

use tesseras_core::ports::ReciprocityLedger;
use tesseras_core::types::NodeId;
use tesseras_core::CoreError;

/// SQLite-backed reciprocity ledger for bilateral storage tracking.
pub struct SqliteReciprocityLedger {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteReciprocityLedger {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }
}

impl ReciprocityLedger for SqliteReciprocityLedger {
    fn record_stored_for_peer(&self, peer: &NodeId, bytes: u64) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO reciprocity (peer_id, bytes_stored_for_them, bytes_they_store_for_us, last_updated)
             VALUES (?1, ?2, 0, ?3)
             ON CONFLICT(peer_id) DO UPDATE SET
                 bytes_stored_for_them = bytes_stored_for_them + ?2,
                 last_updated = ?3",
            rusqlite::params![
                peer.to_string(),
                bytes as i64,
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn record_peer_stores_for_us(&self, peer: &NodeId, bytes: u64) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO reciprocity (peer_id, bytes_stored_for_them, bytes_they_store_for_us, last_updated)
             VALUES (?1, 0, ?2, ?3)
             ON CONFLICT(peer_id) DO UPDATE SET
                 bytes_they_store_for_us = bytes_they_store_for_us + ?2,
                 last_updated = ?3",
            rusqlite::params![
                peer.to_string(),
                bytes as i64,
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn balance(&self, peer: &NodeId) -> Result<i64, CoreError> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT balance FROM reciprocity WHERE peer_id = ?1",
            rusqlite::params![peer.to_string()],
            |row| row.get(0),
        );
        match result {
            Ok(balance) => Ok(balance),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(CoreError::Database(e.to_string())),
        }
    }

    fn best_peers_for_replication(&self, count: usize) -> Result<Vec<NodeId>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT peer_id FROM reciprocity ORDER BY balance DESC LIMIT ?1")
            .map_err(|e| CoreError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![count as i64], |row| {
                let peer_id: String = row.get(0)?;
                Ok(peer_id)
            })
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let mut peers = Vec::new();
        for row in rows {
            let peer_hex = row.map_err(|e| CoreError::Database(e.to_string()))?;
            let node_id: NodeId = peer_hex
                .parse()
                .map_err(|_| CoreError::Database(format!("invalid node_id: {peer_hex}")))?;
            peers.push(node_id);
        }
        Ok(peers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_migrations;
    use std::sync::{Arc, Mutex};

    fn setup() -> SqliteReciprocityLedger {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        SqliteReciprocityLedger::new(Arc::new(Mutex::new(conn)))
    }

    fn node(fill: u8) -> NodeId {
        NodeId::new([fill; 20])
    }

    #[test]
    fn initial_balance_is_zero() {
        let ledger = setup();
        assert_eq!(ledger.balance(&node(0x01)).unwrap(), 0);
    }

    #[test]
    fn record_stored_for_peer_decreases_balance() {
        let ledger = setup();
        ledger.record_stored_for_peer(&node(0x01), 1000).unwrap();
        // balance = they_store - we_store = 0 - 1000 = -1000
        assert_eq!(ledger.balance(&node(0x01)).unwrap(), -1000);
    }

    #[test]
    fn record_peer_stores_for_us_increases_balance() {
        let ledger = setup();
        ledger.record_peer_stores_for_us(&node(0x01), 500).unwrap();
        assert_eq!(ledger.balance(&node(0x01)).unwrap(), 500);
    }

    #[test]
    fn bilateral_balance() {
        let ledger = setup();
        ledger.record_stored_for_peer(&node(0x01), 1000).unwrap();
        ledger
            .record_peer_stores_for_us(&node(0x01), 700)
            .unwrap();
        assert_eq!(ledger.balance(&node(0x01)).unwrap(), -300); // 700 - 1000
    }

    #[test]
    fn best_peers_ordered_by_balance_descending() {
        let ledger = setup();
        // peer_a: balance +500
        ledger
            .record_peer_stores_for_us(&node(0x01), 500)
            .unwrap();
        // peer_b: balance -200
        ledger.record_stored_for_peer(&node(0x02), 200).unwrap();
        // peer_c: balance +100
        ledger
            .record_peer_stores_for_us(&node(0x03), 100)
            .unwrap();

        let best = ledger.best_peers_for_replication(3).unwrap();
        assert_eq!(best.len(), 3);
        assert_eq!(best[0], node(0x01)); // +500 first
        assert_eq!(best[1], node(0x03)); // +100 second
        assert_eq!(best[2], node(0x02)); // -200 last
    }
}
