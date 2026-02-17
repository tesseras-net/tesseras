use std::net::SocketAddr;

use crate::config::{DataDir, NodeConfig};
use crate::crypto::{self, Identity};
use crate::dht::Dht;
use crate::net::QuicTransport;
use crate::storage::Storage;
use crate::types::{ContentHash, MediaType, Memory, Tessera, Visibility};

/// The Node orchestrator: ties storage, DHT, QUIC, and replication together.
pub struct Node {
    pub storage: Storage,
    pub identity: Identity,
    pub config: NodeConfig,
    pub dht: Dht,
    transport: Option<QuicTransport>,
}

impl Node {
    /// Create a new node with the given data directory.
    pub fn new(data_dir: DataDir, identity: Identity, config: NodeConfig) -> Result<Self, NodeError> {
        let storage = Storage::open(data_dir).map_err(|e| NodeError::Storage(e.to_string()))?;
        let node_id = identity.node_id();
        let dht = Dht::new(node_id);

        Ok(Self {
            storage,
            identity,
            config,
            dht,
            transport: None,
        })
    }

    /// Start the QUIC transport listener.
    pub async fn start_listener(&mut self) -> Result<SocketAddr, NodeError> {
        let transport = QuicTransport::bind(self.config.listen)
            .await
            .map_err(|e| NodeError::Network(e.to_string()))?;
        let addr = transport
            .local_addr()
            .map_err(|e| NodeError::Network(e.to_string()))?;
        self.transport = Some(transport);
        Ok(addr)
    }

    /// Add a tessera from local files.
    pub fn add_tessera(
        &self,
        files: &[std::path::PathBuf],
        name: Option<String>,
        visibility: Visibility,
    ) -> Result<Tessera, NodeError> {
        let mut memories = Vec::new();
        for file_path in files {
            let filename = file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed".into());

            let ext = file_path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();

            let size = std::fs::metadata(file_path)
                .map_err(|e| NodeError::Io(e.to_string()))?
                .len();

            let mut reader = std::fs::File::open(file_path)
                .map_err(|e| NodeError::Io(e.to_string()))?;
            let blob_hash = self
                .storage
                .store_blob(&mut reader)
                .map_err(|e| NodeError::Storage(e.to_string()))?;

            memories.push(Memory {
                filename,
                media_type: MediaType::from_extension(&ext),
                size,
                blob_hash,
            });
        }

        let content =
            rmp_serde::to_vec(&memories).map_err(|e| NodeError::Serialization(e.to_string()))?;
        let hash = crypto::hash_bytes(&content);
        let signature = self.identity.sign(&content);

        let tessera = Tessera {
            hash,
            author: self.identity.public_key_bytes(),
            signature,
            created_at: chrono::Utc::now(),
            name,
            visibility,
            memories,
        };

        self.storage
            .store_tessera(&tessera)
            .map_err(|e| NodeError::Storage(e.to_string()))?;

        Ok(tessera)
    }

    /// Get a tessera by hash (local lookup).
    pub fn get_tessera(&self, hash: &ContentHash) -> Result<Option<Tessera>, NodeError> {
        self.storage
            .find_tessera(hash)
            .map_err(|e| NodeError::Storage(e.to_string()))
    }

    /// Remove a tessera and its blobs.
    pub fn remove_tessera(&self, hash: &ContentHash) -> Result<(), NodeError> {
        let tessera = self
            .storage
            .find_tessera(hash)
            .map_err(|e| NodeError::Storage(e.to_string()))?
            .ok_or_else(|| NodeError::NotFound(hash.to_string()))?;

        for memory in &tessera.memories {
            self.storage
                .delete_blob(&memory.blob_hash)
                .map_err(|e| NodeError::Storage(e.to_string()))?;
        }

        self.storage
            .delete_tessera(hash)
            .map_err(|e| NodeError::Storage(e.to_string()))?;

        Ok(())
    }

    /// List all tesseras.
    pub fn list_tesseras(&self) -> Result<Vec<Tessera>, NodeError> {
        self.storage
            .list_tesseras()
            .map_err(|e| NodeError::Storage(e.to_string()))
    }

    /// Get the node's ID.
    pub fn node_id(&self) -> crate::types::NodeId {
        self.identity.node_id()
    }

    /// Get the local address this node is listening on.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.transport.as_ref()?.local_addr().ok()
    }

    /// Close the transport.
    pub fn shutdown(&self) {
        if let Some(transport) = &self.transport {
            transport.close();
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("not found: {0}")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node() -> (tempfile::TempDir, Node) {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = DataDir::open(tmp.path()).unwrap();
        let identity = Identity::generate();
        let config = NodeConfig::default();
        let node = Node::new(data_dir, identity, config).unwrap();
        (tmp, node)
    }

    #[test]
    fn add_and_get_tessera() {
        let (tmp, node) = test_node();

        // Create test file
        let test_file = tmp.path().join("test.txt");
        std::fs::write(&test_file, b"hello world").unwrap();

        let tessera = node
            .add_tessera(
                &[test_file],
                Some("Test".into()),
                Visibility::Public,
            )
            .unwrap();

        assert_eq!(tessera.name, Some("Test".into()));
        assert_eq!(tessera.memories.len(), 1);

        let found = node.get_tessera(&tessera.hash).unwrap().unwrap();
        assert_eq!(found.name, Some("Test".into()));
    }

    #[test]
    fn remove_tessera() {
        let (tmp, node) = test_node();

        let test_file = tmp.path().join("test.txt");
        std::fs::write(&test_file, b"to be removed").unwrap();

        let tessera = node
            .add_tessera(&[test_file], None, Visibility::Private)
            .unwrap();

        node.remove_tessera(&tessera.hash).unwrap();
        assert!(node.get_tessera(&tessera.hash).unwrap().is_none());
    }

    #[test]
    fn list_tesseras() {
        let (tmp, node) = test_node();

        let file1 = tmp.path().join("a.txt");
        let file2 = tmp.path().join("b.txt");
        std::fs::write(&file1, b"file a").unwrap();
        std::fs::write(&file2, b"file b").unwrap();

        node.add_tessera(&[file1], Some("A".into()), Visibility::Public)
            .unwrap();
        node.add_tessera(&[file2], Some("B".into()), Visibility::Public)
            .unwrap();

        let list = node.list_tesseras().unwrap();
        assert_eq!(list.len(), 2);
    }
}
