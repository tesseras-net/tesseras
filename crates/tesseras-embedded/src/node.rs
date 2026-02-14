//! The embedded node: owns a tokio Runtime and all services.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{broadcast, watch};

use tesseras_core::NodeIdentity;
use tesseras_dht::engine::DhtEngine;
use tesseras_dht::pow;
use tesseras_net::{QuinnTransport, Transport};
use tesseras_replication::ReplicationService;
use tesseras_storage::{FsBlobStore, FsFragmentStore, SqliteReciprocityLedger};

use crate::error::TesserasError;
use crate::types::NetworkEvent;

pub struct EmbeddedNode {
    runtime: tokio::runtime::Runtime,
    data_dir: PathBuf,
    #[allow(dead_code)]
    conn: Arc<Mutex<rusqlite::Connection>>,
    identity: NodeIdentity,
    engine: Arc<DhtEngine>,
    shutdown_tx: watch::Sender<bool>,
    event_tx: broadcast::Sender<NetworkEvent>,
    running: AtomicBool,
    started_at: std::time::Instant,
}

impl EmbeddedNode {
    /// Start the embedded node. Creates tokio runtime, opens SQLite,
    /// wires storage, inits DHT, spawns background tasks.
    pub fn start(data_dir: String) -> Result<Self, TesserasError> {
        let data_dir = PathBuf::from(&data_dir);
        std::fs::create_dir_all(&data_dir)?;

        // Create our own tokio runtime (Flutter has its own event loop)
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("tesseras-embedded")
            .build()
            .map_err(|e| TesserasError::Storage(format!("failed to create runtime: {e}")))?;

        // Open SQLite and run migrations
        let db_path = data_dir.join("db").join("tesseras.db");
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| TesserasError::Storage(format!("failed to open database: {e}")))?;
        tesseras_storage::run_migrations(&conn)
            .map_err(|e| TesserasError::Storage(format!("migration failed: {e}")))?;
        let conn = Arc::new(Mutex::new(conn));

        // Load or generate node identity
        let identity = Self::load_or_generate_identity(&data_dir)?;

        tracing::info!(node_id = %identity.node_id, "embedded node identity loaded");

        // Create QUIC transport — bind to ephemeral port for mobile
        let engine = runtime.block_on(async {
            let listen_addr: std::net::SocketAddr = "0.0.0.0:0".parse().unwrap();
            let transport = QuinnTransport::bind(listen_addr)
                .await
                .map_err(|e| TesserasError::Network(format!("failed to bind QUIC: {e}")))?;

            tracing::info!(addr = %transport.local_addr(), "QUIC transport bound");

            let dht_config = tesseras_dht::config::DhtConfig::default();
            let engine = DhtEngine::new(identity.clone(), Box::new(transport), dht_config);
            Ok::<Arc<DhtEngine>, TesserasError>(engine)
        })?;

        // Create storage instances
        let fragment_store =
            FsFragmentStore::new(Arc::clone(&conn), data_dir.join("fragments"));
        let reciprocity_ledger = SqliteReciprocityLedger::new(Arc::clone(&conn));
        let blob_store = FsBlobStore::new(data_dir.join("blobs"));

        // Create replication service
        let dht_adapter = crate::dht_adapter::DhtPortAdapter::new(Arc::clone(&engine));
        let replication_config = tesseras_replication::ReplicationConfig::default();
        let replication = ReplicationService::new(
            identity.clone(),
            Box::new(dht_adapter),
            Box::new(fragment_store),
            Box::new(reciprocity_ledger),
            Box::new(blob_store),
            replication_config,
        );

        // Setup shutdown and event channels
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (event_tx, _) = broadcast::channel(256);

        // Spawn DHT engine run loop
        let engine_clone = Arc::clone(&engine);
        runtime.spawn(async move {
            engine_clone.run(shutdown_rx).await;
        });

        // Spawn repair loop
        let repl_shutdown = shutdown_tx.subscribe();
        runtime.spawn(async move {
            replication.run_repair_loop(repl_shutdown).await;
        });

        let event_tx_clone = event_tx.clone();
        runtime.spawn(async move {
            // Emit BootstrapComplete once DHT is seeded (placeholder)
            let _ = event_tx_clone.send(NetworkEvent::BootstrapComplete);
        });

        Ok(Self {
            runtime,
            data_dir,
            conn,
            identity,
            engine,
            shutdown_tx,
            event_tx,
            running: AtomicBool::new(true),
            started_at: std::time::Instant::now(),
        })
    }

    /// Stop the node. Signals shutdown, awaits drain, drops runtime.
    pub fn stop(self) -> Result<(), TesserasError> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        tracing::info!("embedded node shutting down");
        self.shutdown_tx.send(true).ok();

        // Give background tasks a moment to drain
        self.runtime.block_on(async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        });

        // Runtime drops here, cancelling remaining tasks
        tracing::info!("embedded node stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    pub fn event_sender(&self) -> &broadcast::Sender<NetworkEvent> {
        &self.event_tx
    }

    fn load_or_generate_identity(data_dir: &PathBuf) -> Result<NodeIdentity, TesserasError> {
        let identity_path = data_dir.join("identity.key");
        if identity_path.exists() {
            let bytes = std::fs::read(&identity_path)?;
            if bytes.len() < 40 {
                return Err(TesserasError::Storage("invalid identity file".into()));
            }
            let pubkey: [u8; 32] = bytes[..32]
                .try_into()
                .map_err(|_| TesserasError::Storage("invalid identity file".into()))?;
            let nonce = u64::from_le_bytes(
                bytes[32..40]
                    .try_into()
                    .map_err(|_| TesserasError::Storage("invalid identity file".into()))?,
            );
            let node_id = pow::compute_node_id(&pubkey, nonce);
            Ok(NodeIdentity {
                node_id,
                public_key: pubkey,
                nonce,
            })
        } else {
            let mut rng = rand::thread_rng();
            let mut pubkey = [0u8; 32];
            rand::Rng::fill(&mut rng, &mut pubkey);
            tracing::info!("generating new node identity (PoW)...");
            let identity = pow::generate_node_identity(&pubkey);
            let mut bytes = Vec::with_capacity(40);
            bytes.extend_from_slice(&identity.public_key);
            bytes.extend_from_slice(&identity.nonce.to_le_bytes());
            std::fs::write(&identity_path, &bytes)?;
            Ok(identity)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn node_start_stop_lifecycle() {
        let dir = TempDir::new().unwrap();
        let node = EmbeddedNode::start(dir.path().to_str().unwrap().to_string())
            .expect("node should start");
        assert!(node.is_running());
        node.stop().expect("node should stop cleanly");
    }

    #[test]
    fn node_restart_cycle() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().to_str().unwrap().to_string();

        // First start
        let node = EmbeddedNode::start(data_dir.clone()).expect("first start");
        node.stop().expect("first stop");

        // Second start — must not corrupt SQLite or leave locks
        let node = EmbeddedNode::start(data_dir).expect("second start");
        node.stop().expect("second stop");
    }
}
