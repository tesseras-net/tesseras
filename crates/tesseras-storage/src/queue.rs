use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tesseras_core::CoreError;
use tesseras_core::ports::{OperationQueue, QueueEntry, QueuedOperation};

pub struct SqliteOperationQueue {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteOperationQueue {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

fn parse_entry(row: &rusqlite::Row) -> rusqlite::Result<QueueEntry> {
    let payload: Vec<u8> = row.get(2)?;
    let operation: QueuedOperation = rmp_serde::from_slice(&payload).map_err(|e| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            e.to_string(),
        )))
    })?;
    Ok(QueueEntry {
        id: row.get(0)?,
        operation,
        status: row.get(3)?,
        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
            .unwrap()
            .with_timezone(&chrono::Utc),
        completed_at: row.get::<_, Option<String>>(5)?.map(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&chrono::Utc)
        }),
        error: row.get(6)?,
        retries: row.get::<_, u32>(7)?,
    })
}

impl OperationQueue for SqliteOperationQueue {
    fn enqueue(&self, op: &QueuedOperation) -> Result<i64, CoreError> {
        let conn = self.conn.lock().unwrap();
        let payload = rmp_serde::to_vec(op).map_err(|e| CoreError::Database(e.to_string()))?;
        let op_type = match op {
            QueuedOperation::Push { .. } => "push",
            QueuedOperation::Pull { .. } => "pull",
            QueuedOperation::Delete { .. } => "delete",
            QueuedOperation::Retract { .. } => "retract",
        };
        conn.execute(
            "INSERT INTO operation_queue (op_type, payload, status, created_at) VALUES (?1, ?2, 'pending', ?3)",
            rusqlite::params![op_type, payload, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(conn.last_insert_rowid())
    }

    fn dequeue_pending(&self) -> Result<Option<QueueEntry>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, op_type, payload, status, created_at, completed_at, error, retries FROM operation_queue WHERE status = 'pending' ORDER BY id ASC LIMIT 1",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let result = stmt.query_row([], parse_entry);
        match result {
            Ok(entry) => {
                conn.execute(
                    "UPDATE operation_queue SET status = 'in_progress' WHERE id = ?1",
                    rusqlite::params![entry.id],
                )
                .map_err(|e| CoreError::Database(e.to_string()))?;
                Ok(Some(entry))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Database(e.to_string())),
        }
    }

    fn mark_completed(&self, id: i64) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE operation_queue SET status = 'completed', completed_at = ?2 WHERE id = ?1",
            rusqlite::params![id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn mark_failed(&self, id: i64, error: &str) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE operation_queue SET status = 'failed', error = ?2, completed_at = ?3 WHERE id = ?1",
            rusqlite::params![id, error, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn increment_retries(&self, id: i64) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE operation_queue SET retries = retries + 1 WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(())
    }

    fn list_pending(&self) -> Result<Vec<QueueEntry>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, op_type, payload, status, created_at, completed_at, error, retries FROM operation_queue WHERE status = 'pending' ORDER BY id ASC",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], parse_entry)
            .map_err(|e| CoreError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Database(e.to_string()))
    }

    fn list_recent(&self, limit: u32) -> Result<Vec<QueueEntry>, CoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, op_type, payload, status, created_at, completed_at, error, retries FROM operation_queue ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![limit], parse_entry)
            .map_err(|e| CoreError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::Database(e.to_string()))
    }

    fn count_by_status(&self) -> Result<(u32, u32, u32), CoreError> {
        let conn = self.conn.lock().unwrap();
        let pending: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM operation_queue WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        let completed: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM operation_queue WHERE status = 'completed'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        let failed: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM operation_queue WHERE status = 'failed'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok((pending, completed, failed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseras_core::types::ContentHash;

    fn setup() -> Arc<Mutex<Connection>> {
        let conn =
            crate::database::open_in_memory(&crate::database::StorageConfig::default()).unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn enqueue_and_dequeue() {
        let conn = setup();
        let queue = SqliteOperationQueue::new(conn);
        let op = QueuedOperation::Push {
            hash: ContentHash::new([0xab; 32]),
        };
        let id = queue.enqueue(&op).unwrap();
        assert!(id > 0);

        let entry = queue.dequeue_pending().unwrap().unwrap();
        assert_eq!(entry.operation, op);
        assert_eq!(entry.status, "pending");
    }

    #[test]
    fn dequeue_empty_returns_none() {
        let conn = setup();
        let queue = SqliteOperationQueue::new(conn);
        assert!(queue.dequeue_pending().unwrap().is_none());
    }

    #[test]
    fn mark_completed() {
        let conn = setup();
        let queue = SqliteOperationQueue::new(conn);
        let op = QueuedOperation::Pull {
            hash: ContentHash::new([0xcd; 32]),
        };
        let id = queue.enqueue(&op).unwrap();
        queue.mark_completed(id).unwrap();

        let (pending, completed, _) = queue.count_by_status().unwrap();
        assert_eq!(pending, 0);
        assert_eq!(completed, 1);
    }

    #[test]
    fn mark_failed() {
        let conn = setup();
        let queue = SqliteOperationQueue::new(conn);
        let op = QueuedOperation::Delete {
            hash: ContentHash::new([0xef; 32]),
        };
        let id = queue.enqueue(&op).unwrap();
        queue.mark_failed(id, "network timeout").unwrap();

        let (pending, _, failed) = queue.count_by_status().unwrap();
        assert_eq!(pending, 0);
        assert_eq!(failed, 1);
    }

    #[test]
    fn increment_retries() {
        let conn = setup();
        let queue = SqliteOperationQueue::new(conn);
        let op = QueuedOperation::Retract {
            hash: ContentHash::new([0x11; 32]),
        };
        let id = queue.enqueue(&op).unwrap();
        queue.increment_retries(id).unwrap();
        queue.increment_retries(id).unwrap();

        let entries = queue.list_pending().unwrap();
        assert_eq!(entries[0].retries, 2);
    }

    #[test]
    fn list_recent() {
        let conn = setup();
        let queue = SqliteOperationQueue::new(conn);
        for i in 0..5 {
            queue
                .enqueue(&QueuedOperation::Push {
                    hash: ContentHash::new([i; 32]),
                })
                .unwrap();
        }
        let recent = queue.list_recent(3).unwrap();
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn count_by_status() {
        let conn = setup();
        let queue = SqliteOperationQueue::new(conn);
        let id1 = queue
            .enqueue(&QueuedOperation::Push {
                hash: ContentHash::new([0x01; 32]),
            })
            .unwrap();
        let id2 = queue
            .enqueue(&QueuedOperation::Pull {
                hash: ContentHash::new([0x02; 32]),
            })
            .unwrap();
        queue
            .enqueue(&QueuedOperation::Delete {
                hash: ContentHash::new([0x03; 32]),
            })
            .unwrap();

        queue.mark_completed(id1).unwrap();
        queue.mark_failed(id2, "err").unwrap();

        let (pending, completed, failed) = queue.count_by_status().unwrap();
        assert_eq!(pending, 1);
        assert_eq!(completed, 1);
        assert_eq!(failed, 1);
    }
}
