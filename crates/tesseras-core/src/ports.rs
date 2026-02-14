use crate::replication::{Attestation, FragmentEnvelope, FragmentId, ReplicateAck};
use crate::types::NodeId;
use crate::{ContentHash, CoreError, NodeInfo};

/// Tessera metadata persistence.
pub trait TesseraRepository: Send + Sync {
    fn store(&self, tessera: &TesseraRecord) -> Result<(), CoreError>;
    fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<TesseraRecord>, CoreError>;
    /// Find tesseras whose hex hash starts with the given prefix.
    fn find_by_hex_prefix(&self, hex_prefix: &str) -> Result<Vec<TesseraRecord>, CoreError>;
    fn list(&self) -> Result<Vec<TesseraRecord>, CoreError>;
    fn delete(&self, hash: &ContentHash) -> Result<(), CoreError>;
    fn exists(&self, hash: &ContentHash) -> Result<bool, CoreError>;
}

/// Memory metadata persistence.
pub trait MemoryRepository: Send + Sync {
    fn store(&self, memory: &MemoryRecord) -> Result<(), CoreError>;
    fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<MemoryRecord>, CoreError>;
    fn list_by_tessera(&self, tessera_hash: &ContentHash) -> Result<Vec<MemoryRecord>, CoreError>;
    fn delete(&self, hash: &ContentHash) -> Result<(), CoreError>;
}

/// Raw file storage.
pub trait BlobStore: Send + Sync {
    fn write(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
        data: &[u8],
    ) -> Result<(), CoreError>;
    fn read(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<Vec<u8>, CoreError>;
    fn exists(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<bool, CoreError>;
    fn delete_tessera(&self, tessera_hash: &ContentHash) -> Result<(), CoreError>;
}

/// Identity key persistence.
pub trait IdentityStore: Send + Sync {
    fn save_keypair(&self, material: &KeyMaterial) -> Result<(), CoreError>;
    fn load_keypair(&self, algorithm: KeyAlgorithm) -> Result<KeyMaterial, CoreError>;
    fn keypair_exists(&self, algorithm: KeyAlgorithm) -> Result<bool, CoreError>;
}

/// Boundary type for key material (decoupled from crypto backends).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMaterial {
    pub algorithm: KeyAlgorithm,
    pub secret: Vec<u8>,
    pub public: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAlgorithm {
    Ed25519,
    MlDsa,
}

/// Content hasher port (abstracts BLAKE3 or other hash).
pub trait Hasher: Send + Sync {
    fn hash(&self, data: &[u8]) -> ContentHash;
}

/// Manifest signing port.
pub trait ManifestSigner: Send + Sync {
    /// Sign manifest bytes, return (ed25519_sig_bytes, public_key_hex).
    fn sign(&self, manifest: &[u8]) -> (Vec<u8>, String);
}

/// Network operations needed by the replication engine.
#[async_trait::async_trait]
pub trait DhtPort: Send + Sync {
    async fn find_closest_nodes(&self, target: &NodeId) -> Vec<NodeInfo>;
    async fn replicate_fragment(
        &self,
        target: &NodeInfo,
        fragment: &FragmentEnvelope,
    ) -> Result<ReplicateAck, CoreError>;
    async fn request_attestation(
        &self,
        target: &NodeInfo,
        tessera_hash: &ContentHash,
    ) -> Result<Attestation, CoreError>;
    async fn ping(&self, target: &NodeInfo) -> bool;
}

/// Local storage for erasure-coded fragments. Sync (consistent with existing storage traits).
pub trait FragmentStore: Send + Sync {
    fn store_fragment(&self, id: &FragmentId, data: &[u8]) -> Result<(), CoreError>;
    fn read_fragment(&self, id: &FragmentId) -> Result<Vec<u8>, CoreError>;
    fn delete_fragment(&self, id: &FragmentId) -> Result<(), CoreError>;
    fn list_fragments(&self, tessera_hash: &ContentHash) -> Result<Vec<FragmentId>, CoreError>;
    fn verify_fragment(&self, id: &FragmentId) -> Result<bool, CoreError>;
}

/// Bilateral reciprocity tracking. Sync.
pub trait ReciprocityLedger: Send + Sync {
    fn record_stored_for_peer(&self, peer: &NodeId, bytes: u64) -> Result<(), CoreError>;
    fn record_peer_stores_for_us(&self, peer: &NodeId, bytes: u64) -> Result<(), CoreError>;
    fn balance(&self, peer: &NodeId) -> Result<i64, CoreError>;
    fn best_peers_for_replication(&self, count: usize) -> Result<Vec<NodeId>, CoreError>;
}

/// Manifest verification port.
pub trait ManifestVerifier: Send + Sync {
    /// Verify manifest bytes against signature and public key.
    fn verify(&self, manifest: &[u8], signature: &[u8], public_key_hex: &str) -> bool;
}

/// Flat record for DB storage (not the domain aggregate).
#[derive(Debug, Clone)]
pub struct TesseraRecord {
    pub hash: ContentHash,
    pub creator_pubkey: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub size_bytes: u64,
    pub memory_count: u32,
    pub visibility: String,
    pub sealed_until: Option<chrono::DateTime<chrono::Utc>>,
    pub is_mine: bool,
}

/// Flat record for DB storage.
#[derive(Debug, Clone)]
pub struct MemoryRecord {
    pub hash: ContentHash,
    pub tessera_hash: ContentHash,
    pub memory_type: String,
    pub media_path: String,
    pub context_path: Option<String>,
    pub meta_json: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
