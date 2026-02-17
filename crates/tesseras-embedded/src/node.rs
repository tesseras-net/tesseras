use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tesseras::config::DataDir;
use tesseras::crypto::Identity;
use tesseras::node::Node;
use tesseras::types::Visibility;

use crate::types::{IdentityInfo, MemoryInfo, NetworkStats, ReplicationStatus};

/// Embedded node for mobile/desktop FFI via flutter_rust_bridge.
///
/// Owns a tokio runtime and a `tesseras::node::Node`, exposing a
/// synchronous API suitable for FFI.
pub struct EmbeddedNode {
    /// Kept alive to sustain background tasks (accept loop, repair loop, etc.).
    _runtime: tokio::runtime::Runtime,
    node: Arc<Mutex<Option<Node>>>,
    data_dir: DataDir,
    running: AtomicBool,
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddedError {
    #[error("node not started")]
    NotStarted,
    #[error("node already running")]
    AlreadyRunning,
    #[error("{0}")]
    Node(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    InvalidInput(String),
}

impl EmbeddedNode {
    /// Start an embedded node at the given data directory path.
    pub fn start(data_dir_path: String) -> Result<Self, EmbeddedError> {
        let data_dir = DataDir::open(PathBuf::from(&data_dir_path))
            .map_err(|e| EmbeddedError::Io(e.to_string()))?;

        let config = data_dir
            .load_config()
            .map_err(|e| EmbeddedError::Node(e.to_string()))?;

        // Load or generate identity
        let key_path = data_dir.identity_key_path();
        let identity = if key_path.exists() {
            Identity::load(&key_path).map_err(|e| EmbeddedError::Io(e.to_string()))?
        } else {
            let id = Identity::generate();
            id.save(&key_path)
                .map_err(|e| EmbeddedError::Io(e.to_string()))?;
            id
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("tesseras-embedded")
            .build()
            .map_err(|e| EmbeddedError::Io(e.to_string()))?;

        let mut node = Node::new(data_dir.clone(), identity, config)
            .map_err(|e| EmbeddedError::Node(e.to_string()))?;

        // All tokio operations must run inside the runtime context.
        runtime.block_on(async {
            node.start()
                .await
                .map_err(|e| EmbeddedError::Node(e.to_string()))?;
            node.start_refresh_loop();
            node.start_repair_loop();
            let _ = node.bootstrap().await;
            Ok::<(), EmbeddedError>(())
        })?;

        let embedded = Self {
            _runtime: runtime,
            node: Arc::new(Mutex::new(Some(node))),
            data_dir,
            running: AtomicBool::new(true),
        };

        Ok(embedded)
    }

    /// Stop the embedded node.
    pub fn stop(self) -> Result<(), EmbeddedError> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(node) = self.node.lock().unwrap().take() {
            node.shutdown();
        }
        // Runtime drops here, shutting down background tasks
        Ok(())
    }

    /// Whether the node is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn with_node<F, R>(&self, f: F) -> Result<R, EmbeddedError>
    where
        F: FnOnce(&Node) -> Result<R, EmbeddedError>,
    {
        let guard = self.node.lock().unwrap();
        let node = guard.as_ref().ok_or(EmbeddedError::NotStarted)?;
        f(node)
    }

    /// Create or update the user profile (identity info stored as JSON).
    pub fn create_identity(
        &self,
        name: String,
        avatar_path: Option<String>,
    ) -> Result<IdentityInfo, EmbeddedError> {
        let profile_path = self.data_dir.root().join("profile.json");

        let info = self.with_node(|node| {
            let node_id = node.node_id();
            Ok(IdentityInfo {
                name: name.clone(),
                avatar_path: avatar_path.clone(),
                public_key_hex: node_id.to_string(),
                node_id_hex: node_id.to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            })
        })?;

        let json =
            serde_json::to_string_pretty(&info).map_err(|e| EmbeddedError::Io(e.to_string()))?;
        std::fs::write(&profile_path, json).map_err(|e| EmbeddedError::Io(e.to_string()))?;

        Ok(info)
    }

    /// Get the current user identity if profile.json exists.
    pub fn get_identity(&self) -> Result<Option<IdentityInfo>, EmbeddedError> {
        let profile_path = self.data_dir.root().join("profile.json");
        if !profile_path.exists() {
            return Ok(None);
        }
        let json =
            std::fs::read_to_string(&profile_path).map_err(|e| EmbeddedError::Io(e.to_string()))?;
        let info: IdentityInfo =
            serde_json::from_str(&json).map_err(|e| EmbeddedError::Io(e.to_string()))?;
        Ok(Some(info))
    }

    /// Create a new memory (add tessera from a file).
    pub fn create_memory(
        &self,
        media_path: String,
        name: Option<String>,
        visibility: String,
    ) -> Result<MemoryInfo, EmbeddedError> {
        let vis: Visibility = visibility
            .parse()
            .map_err(|e: String| EmbeddedError::InvalidInput(e))?;
        let path = std::path::PathBuf::from(&media_path);

        if !path.exists() {
            return Err(EmbeddedError::InvalidInput(format!(
                "file not found: {media_path}"
            )));
        }

        self.with_node(|node| {
            let tessera = node
                .add_tessera(&[path], name, vis)
                .map_err(|e| EmbeddedError::Node(e.to_string()))?;
            // Announce + distribute happens via the background repair loop.
            Ok(tessera_to_memory_info(&tessera))
        })
    }

    /// List tesseras as MemoryInfo (paginated).
    pub fn get_timeline(&self, offset: u32, limit: u32) -> Result<Vec<MemoryInfo>, EmbeddedError> {
        self.with_node(|node| {
            let tesseras = node
                .list_tesseras()
                .map_err(|e| EmbeddedError::Node(e.to_string()))?;

            let items: Vec<MemoryInfo> = tesseras
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .map(|t| tessera_to_memory_info(&t))
                .collect();

            Ok(items)
        })
    }

    /// Get a single memory by hash.
    pub fn get_memory(&self, hash: String) -> Result<Option<MemoryInfo>, EmbeddedError> {
        let content_hash: tesseras::types::ContentHash =
            hash.parse()
                .map_err(|e: tesseras::types::ContentHashError| {
                    EmbeddedError::InvalidInput(e.to_string())
                })?;

        self.with_node(|node| {
            let tessera = node
                .get_tessera(&content_hash)
                .map_err(|e| EmbeddedError::Node(e.to_string()))?;

            Ok(tessera.map(|t| tessera_to_memory_info(&t)))
        })
    }

    /// Get network stats.
    pub fn get_network_stats(&self) -> Result<NetworkStats, EmbeddedError> {
        self.with_node(|node| {
            Ok(NetworkStats {
                peer_count: node.peer_count() as u32,
                is_bootstrapped: node.peer_count() > 0,
                node_id_hex: node.node_id().to_string(),
                listen_addr: node
                    .public_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_default(),
            })
        })
    }

    /// Get replication status.
    pub fn get_replication_status(&self) -> Result<ReplicationStatus, EmbeddedError> {
        self.with_node(|node| {
            let missing = node
                .check_fragments()
                .map_err(|e| EmbeddedError::Node(e.to_string()))?;

            // Count total fragments from storage
            let storage = node.storage.lock().unwrap();
            let tesseras = storage.list_tesseras().unwrap_or_default();
            let mut total = 0u32;
            let mut healthy = 0u32;
            for tessera in &tesseras {
                for memory in &tessera.memories {
                    if let Ok(frags) = storage.find_fragments(&memory.blob_hash) {
                        for meta in &frags {
                            total += 1;
                            if storage.has_blob(&meta.fragment_hash) {
                                healthy += 1;
                            }
                        }
                    }
                }
            }

            Ok(ReplicationStatus {
                total_fragments: total,
                healthy_fragments: healthy,
                missing_fragments: missing.len() as u32,
                data_shards: 3, // from config defaults
                parity_shards: 2,
            })
        })
    }
}

/// Convert a Tessera into the flat FFI-safe MemoryInfo.
fn tessera_to_memory_info(tessera: &tesseras::types::Tessera) -> MemoryInfo {
    let first_memory = tessera.memories.first();
    MemoryInfo {
        hash: tessera.hash.to_string(),
        tessera_hash: tessera.hash.to_string(),
        filename: first_memory.map(|m| m.filename.clone()).unwrap_or_default(),
        media_type: first_memory
            .map(|m| format!("{:?}", m.media_type))
            .unwrap_or_default(),
        size: first_memory.map(|m| m.size).unwrap_or(0),
        visibility: tessera.visibility.to_string(),
        created_at: tessera.created_at.to_rfc3339(),
        name: tessera.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseras::config::NodeConfig;

    /// Write a test config that binds to ephemeral port, no STUN.
    fn write_test_config(dir: &std::path::Path) {
        let data_dir = DataDir::open(dir).unwrap();
        let mut config = NodeConfig::default();
        config.listen = "127.0.0.1:0".parse().unwrap();
        config.stun_servers = Vec::new();
        data_dir.save_config(&config).unwrap();
    }

    #[test]
    fn start_and_stop() {
        let tmp = tempfile::tempdir().unwrap();
        write_test_config(tmp.path());
        let node = EmbeddedNode::start(tmp.path().to_str().unwrap().to_string()).unwrap();
        assert!(node.is_running());
        node.stop().unwrap();
    }

    #[test]
    fn identity_create_and_get() {
        let tmp = tempfile::tempdir().unwrap();
        write_test_config(tmp.path());
        let node = EmbeddedNode::start(tmp.path().to_str().unwrap().to_string()).unwrap();

        assert!(node.get_identity().unwrap().is_none());

        let info = node.create_identity("Alice".into(), None).unwrap();
        assert_eq!(info.name, "Alice");
        assert!(!info.node_id_hex.is_empty());

        let loaded = node.get_identity().unwrap().unwrap();
        assert_eq!(loaded.name, "Alice");

        node.stop().unwrap();
    }

    #[test]
    fn create_memory_and_timeline() {
        let tmp = tempfile::tempdir().unwrap();
        write_test_config(tmp.path());
        let node = EmbeddedNode::start(tmp.path().to_str().unwrap().to_string()).unwrap();

        let test_file = tmp.path().join("photo.txt");
        std::fs::write(&test_file, b"a precious memory").unwrap();

        let mem = node
            .create_memory(
                test_file.to_str().unwrap().to_string(),
                Some("My Memory".into()),
                "public".into(),
            )
            .unwrap();
        assert!(!mem.hash.is_empty());
        assert_eq!(mem.name, Some("My Memory".into()));

        let timeline = node.get_timeline(0, 10).unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].hash, mem.hash);

        node.stop().unwrap();
    }

    #[test]
    fn network_stats() {
        let tmp = tempfile::tempdir().unwrap();
        write_test_config(tmp.path());
        let node = EmbeddedNode::start(tmp.path().to_str().unwrap().to_string()).unwrap();

        let stats = node.get_network_stats().unwrap();
        assert!(!stats.node_id_hex.is_empty());

        node.stop().unwrap();
    }
}
