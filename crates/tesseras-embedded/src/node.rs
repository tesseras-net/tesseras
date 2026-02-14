//! The embedded node: owns a tokio Runtime and all services.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{broadcast, watch};

use tesseras_core::ports::{IdentityStore, KeyAlgorithm};
use tesseras_core::service::TesseraService;
use tesseras_core::NodeIdentity;
use tesseras_crypto::ed25519::{Ed25519KeyGenerator, Ed25519KeyPair};
use tesseras_dht::engine::DhtEngine;
use tesseras_dht::pow;
use tesseras_net::{QuinnTransport, Transport};
use tesseras_replication::ReplicationService;
use tesseras_storage::{
    FsBlobStore, FsFragmentStore, FsIdentityStore, SqliteMemoryRepository,
    SqliteReciprocityLedger, SqliteTesseraRepository,
};

use crate::crypto_service::{Blake3HasherAdapter, Ed25519SignerAdapter, Ed25519VerifierAdapter};
use crate::error::TesserasError;
use crate::types::{CreateMemoryRequest, IdentityInfo, MemoryInfo, NetworkEvent};

/// Profile stored as JSON alongside cryptographic identity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UserProfile {
    name: String,
    avatar_path: Option<String>,
    created_at: String,
}

pub struct EmbeddedNode {
    runtime: tokio::runtime::Runtime,
    data_dir: PathBuf,
    #[allow(dead_code)]
    conn: Arc<Mutex<rusqlite::Connection>>,
    identity: NodeIdentity,
    engine: Arc<DhtEngine>,
    tessera_service: TesseraService,
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

        // Load or generate Ed25519 signing keypair for TesseraService
        let identity_store = FsIdentityStore::new(data_dir.clone());
        let signing_key = Self::load_or_generate_signing_key(&identity_store)?;

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

        // Create storage instances for replication service
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

        // Create TesseraService (separate storage instances since replication took ownership)
        let tessera_repo = SqliteTesseraRepository::new(Arc::clone(&conn));
        let memory_repo = SqliteMemoryRepository::new(Arc::clone(&conn));
        let blob_store_for_service = FsBlobStore::new(data_dir.join("blobs"));

        let tessera_service = TesseraService::new(
            Box::new(tessera_repo),
            Box::new(memory_repo),
            Box::new(blob_store_for_service),
            Box::new(Blake3HasherAdapter),
            Box::new(Ed25519SignerAdapter::new(signing_key)),
            Box::new(Ed25519VerifierAdapter),
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
            tessera_service,
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

    // -- Identity API --

    pub fn create_identity(
        &self,
        name: String,
        avatar_path: Option<String>,
    ) -> Result<IdentityInfo, TesserasError> {
        let profile_path = self.data_dir.join("profile.json");
        if profile_path.exists() {
            return Err(TesserasError::IdentityAlreadyExists);
        }

        let now = chrono::Utc::now().to_rfc3339();
        let profile = UserProfile {
            name: name.clone(),
            avatar_path: avatar_path.clone(),
            created_at: now.clone(),
        };
        let json = serde_json::to_string_pretty(&profile)
            .map_err(|e| TesserasError::Storage(e.to_string()))?;
        std::fs::write(&profile_path, json)?;

        Ok(IdentityInfo {
            name,
            avatar_path,
            public_key_hex: self
                .identity
                .public_key
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            node_id_hex: self.identity.node_id.to_string(),
            created_at: now,
        })
    }

    pub fn get_identity(&self) -> Result<Option<IdentityInfo>, TesserasError> {
        let profile_path = self.data_dir.join("profile.json");
        if !profile_path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&profile_path)?;
        let profile: UserProfile =
            serde_json::from_str(&json).map_err(|e| TesserasError::Storage(e.to_string()))?;

        Ok(Some(IdentityInfo {
            name: profile.name,
            avatar_path: profile.avatar_path,
            public_key_hex: self
                .identity
                .public_key
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            node_id_hex: self.identity.node_id.to_string(),
            created_at: profile.created_at,
        }))
    }

    // -- Memory API --

    pub fn create_memory(
        &self,
        request: CreateMemoryRequest,
    ) -> Result<MemoryInfo, TesserasError> {
        use tesseras_core::metadata::Location;
        use tesseras_core::service::{CreateInput, FileInput};

        let location = match (
            request.location_description,
            request.location_lat,
            request.location_lon,
        ) {
            (Some(desc), lat, lon) => Some(Location {
                description: desc,
                coordinates: lat.zip(lon),
            }),
            _ => None,
        };

        let input = CreateInput {
            files: vec![FileInput {
                path: std::path::PathBuf::from(&request.media_path),
                context: request.context_text.clone(),
                memory_type: request.memory_type,
            }],
            visibility: request.visibility,
            language: "en".to_string(),
            tags: request.tags.clone(),
            location,
        };

        let hash = self
            .runtime
            .block_on(self.tessera_service.create(input))?;

        Ok(MemoryInfo {
            hash: hash.to_string(),
            tessera_hash: hash.to_string(),
            media_path: request.media_path,
            context: request.context_text,
            memory_type: format!("{:?}", request.memory_type).to_lowercase(),
            visibility: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            tags: request.tags,
        })
    }

    pub fn get_timeline(
        &self,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<MemoryInfo>, TesserasError> {
        let mut records = self.runtime.block_on(self.tessera_service.list())?;

        // Sort by created_at descending (newest first)
        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let page = records
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|r| MemoryInfo {
                hash: r.hash.to_string(),
                tessera_hash: r.hash.to_string(),
                media_path: String::new(),
                context: None,
                memory_type: String::new(),
                visibility: r.visibility,
                created_at: r.created_at.to_rfc3339(),
                tags: vec![],
            })
            .collect();

        Ok(page)
    }

    pub fn get_memory(&self, hash: String) -> Result<MemoryInfo, TesserasError> {
        use std::str::FromStr;
        let content_hash = tesseras_core::ContentHash::from_str(&hash)
            .map_err(|e| TesserasError::InvalidInput(e.to_string()))?;

        let tessera = self
            .runtime
            .block_on(self.tessera_service.list())?
            .into_iter()
            .find(|r| r.hash == content_hash)
            .ok_or_else(|| TesserasError::InvalidInput(format!("tessera not found: {hash}")))?;

        Ok(MemoryInfo {
            hash: tessera.hash.to_string(),
            tessera_hash: tessera.hash.to_string(),
            media_path: String::new(),
            context: None,
            memory_type: String::new(),
            visibility: tessera.visibility,
            created_at: tessera.created_at.to_rfc3339(),
            tags: vec![],
        })
    }

    // -- Private helpers --

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

    fn load_or_generate_signing_key(
        identity_store: &FsIdentityStore,
    ) -> Result<ed25519_dalek::SigningKey, TesserasError> {
        if identity_store
            .keypair_exists(KeyAlgorithm::Ed25519)
            .map_err(|e| TesserasError::Storage(e.to_string()))?
        {
            let material = identity_store
                .load_keypair(KeyAlgorithm::Ed25519)
                .map_err(|e| TesserasError::Storage(e.to_string()))?;
            let keypair = Ed25519KeyPair::try_from(&material)
                .map_err(|e| TesserasError::Storage(e.to_string()))?;
            Ok(keypair.signing_key)
        } else {
            let keypair = Ed25519KeyGenerator::generate();
            let material = tesseras_core::ports::KeyMaterial::from(&keypair);
            identity_store
                .save_keypair(&material)
                .map_err(|e| TesserasError::Storage(e.to_string()))?;
            Ok(keypair.signing_key)
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
    fn create_and_get_identity() {
        let dir = TempDir::new().unwrap();
        let node = EmbeddedNode::start(dir.path().to_str().unwrap().to_string()).unwrap();

        // No identity initially
        let id = node.get_identity().unwrap();
        assert!(id.is_none());

        // Create identity
        let id = node.create_identity("Alice".to_string(), None).unwrap();
        assert_eq!(id.name, "Alice");
        assert!(!id.public_key_hex.is_empty());
        assert!(!id.node_id_hex.is_empty());

        // Get identity returns same data
        let id2 = node.get_identity().unwrap().expect("should exist now");
        assert_eq!(id2.name, id.name);
        assert_eq!(id2.public_key_hex, id.public_key_hex);

        node.stop().unwrap();
    }

    #[test]
    fn identity_persists_across_restart() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().to_str().unwrap().to_string();

        let node = EmbeddedNode::start(data_dir.clone()).unwrap();
        let id = node.create_identity("Bob".to_string(), None).unwrap();
        let pubkey = id.public_key_hex.clone();
        node.stop().unwrap();

        // Restart — identity should persist
        let node = EmbeddedNode::start(data_dir).unwrap();
        let id2 = node.get_identity().unwrap().expect("should persist");
        assert_eq!(id2.name, "Bob");
        assert_eq!(id2.public_key_hex, pubkey);
        node.stop().unwrap();
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

    #[test]
    fn create_memory_and_get_timeline() {
        let dir = TempDir::new().unwrap();
        let node = EmbeddedNode::start(dir.path().to_str().unwrap().to_string()).unwrap();
        node.create_identity("Test".to_string(), None).unwrap();

        // Create a test file
        let media_path = dir.path().join("photo.jpg");
        std::fs::write(&media_path, b"fake jpeg data").unwrap();

        let request = CreateMemoryRequest {
            media_path: media_path.to_str().unwrap().to_string(),
            context_text: Some("A beautiful sunset".to_string()),
            memory_type: tesseras_core::MemoryType::Moment,
            visibility: tesseras_core::Visibility::Private,
            location_description: None,
            location_lat: None,
            location_lon: None,
            tags: vec!["nature".to_string()],
            people: vec![],
        };

        let memory = node.create_memory(request).unwrap();
        assert!(!memory.hash.is_empty());
        assert_eq!(memory.memory_type, "moment");

        // Timeline should contain the memory
        let timeline = node.get_timeline(0, 10).unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].hash, memory.hash);

        node.stop().unwrap();
    }

    #[test]
    fn get_memory_by_hash() {
        let dir = TempDir::new().unwrap();
        let node = EmbeddedNode::start(dir.path().to_str().unwrap().to_string()).unwrap();
        node.create_identity("Test".to_string(), None).unwrap();

        let media_path = dir.path().join("note.txt");
        std::fs::write(&media_path, b"Some thoughts").unwrap();

        let request = CreateMemoryRequest {
            media_path: media_path.to_str().unwrap().to_string(),
            context_text: Some("Daily reflection".to_string()),
            memory_type: tesseras_core::MemoryType::Reflection,
            visibility: tesseras_core::Visibility::Public,
            location_description: None,
            location_lat: None,
            location_lon: None,
            tags: vec![],
            people: vec![],
        };

        let created = node.create_memory(request).unwrap();
        let fetched = node.get_memory(created.hash.clone()).unwrap();
        assert_eq!(fetched.hash, created.hash);

        node.stop().unwrap();
    }
}
