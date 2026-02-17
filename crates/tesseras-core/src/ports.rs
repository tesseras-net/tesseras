use crate::replication::{Attestation, FragmentEnvelope, FragmentId, ReplicateAck};
use crate::types::NodeId;
use crate::{ContentHash, CoreError, NodeInfo};
use zeroize::Zeroize;

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

impl Drop for KeyMaterial {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAlgorithm {
    Ed25519,
    MlDsa,
    X25519,
    MlKem768,
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

/// Handler for incoming REPLICATE/ATTEST RPCs, set on DhtEngine after construction.
#[async_trait::async_trait]
pub trait ReplicationHandler: Send + Sync {
    async fn handle_replicate(
        &self,
        envelope: FragmentEnvelope,
        sender: &NodeId,
    ) -> Result<ReplicateAck, CoreError>;

    async fn handle_attest_request(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<Attestation, CoreError>;
}

/// Local storage for erasure-coded fragments. Sync (consistent with existing storage traits).
pub trait FragmentStore: Send + Sync {
    fn store_fragment(&self, id: &FragmentId, data: &[u8]) -> Result<(), CoreError>;
    fn read_fragment(&self, id: &FragmentId) -> Result<Vec<u8>, CoreError>;
    fn delete_fragment(&self, id: &FragmentId) -> Result<(), CoreError>;
    fn list_fragments(&self, tessera_hash: &ContentHash) -> Result<Vec<FragmentId>, CoreError>;
    fn verify_fragment(&self, id: &FragmentId) -> Result<bool, CoreError>;
    /// List all tessera hashes that have local fragments.
    fn list_tessera_hashes(&self) -> Result<Vec<ContentHash>, CoreError>;
}

/// Bilateral reciprocity tracking. Sync.
pub trait ReciprocityLedger: Send + Sync {
    fn record_stored_for_peer(&self, peer: &NodeId, bytes: u64) -> Result<(), CoreError>;
    fn record_peer_stores_for_us(&self, peer: &NodeId, bytes: u64) -> Result<(), CoreError>;
    fn balance(&self, peer: &NodeId) -> Result<i64, CoreError>;
    fn best_peers_for_replication(&self, count: usize) -> Result<Vec<NodeId>, CoreError>;
    fn mark_institutional(&self, peer: &NodeId) -> Result<(), CoreError>;
    fn is_institutional(&self, peer: &NodeId) -> Result<bool, CoreError>;
}

/// Search index for public tesseras (institutional nodes).
#[allow(clippy::too_many_arguments)]
pub trait SearchIndex: Send + Sync {
    fn index_tessera(
        &self,
        hash: &ContentHash,
        title: Option<&str>,
        description: Option<&str>,
        memory_type: Option<&str>,
        language: Option<&str>,
        tags: &[String],
        visibility: &str,
        created_at: &chrono::DateTime<chrono::Utc>,
        lat: Option<f64>,
        lon: Option<f64>,
    ) -> Result<(), CoreError>;

    fn remove_tessera(&self, hash: &ContentHash) -> Result<(), CoreError>;

    fn search(
        &self,
        query: &str,
        filters: &crate::search::SearchFilters,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<crate::search::SearchHit>, u64), CoreError>;
}

/// Content encryption port for sealed/private tesseras.
pub trait ContentEncryptor: Send + Sync {
    /// Encrypt content, returning ciphertext as opaque bytes (nonce + ciphertext).
    fn encrypt(&self, content: &[u8], key: &[u8; 32], aad: &[u8]) -> Result<Vec<u8>, CoreError>;

    /// Generate a random 256-bit content key.
    fn generate_content_key(&self) -> [u8; 32];

    /// Encapsulate content key to owner's public key.
    /// Returns base64-encoded envelope for storage in manifest.
    fn seal_content_key(
        &self,
        content_key: &[u8; 32],
        encryption_public: &crate::tessera::HybridEncryptionPublic,
    ) -> Result<String, CoreError>;
}

/// Manifest verification port.
pub trait ManifestVerifier: Send + Sync {
    /// Verify manifest bytes against signature and public key.
    fn verify(&self, manifest: &[u8], signature: &[u8], public_key_hex: &str) -> bool;
}

/// Repository for tombstone records (retracted tesseras).
pub trait TombstoneRepository: Send + Sync {
    fn store(&self, tombstone: &crate::Tombstone) -> Result<(), CoreError>;
    fn find(&self, hash: &ContentHash) -> Result<Option<crate::Tombstone>, CoreError>;
    fn exists(&self, hash: &ContentHash) -> Result<bool, CoreError>;
    fn list(&self) -> Result<Vec<crate::Tombstone>, CoreError>;
}

/// A named circle with its encryption key.
#[derive(Debug, Clone)]
pub struct CircleRecord {
    pub name: String,
    pub symmetric_key: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A member of a circle.
#[derive(Debug, Clone)]
pub struct CircleMemberRecord {
    pub circle_name: String,
    pub alias: String,
    pub pubkey: String,
    pub wrapped_key: Vec<u8>,
    pub added_at: chrono::DateTime<chrono::Utc>,
}

/// Repository for named circles (Trusted Wheel).
pub trait CircleRepository: Send + Sync {
    fn create_circle(&self, name: &str, symmetric_key: &[u8]) -> Result<(), CoreError>;
    fn delete_circle(&self, name: &str) -> Result<(), CoreError>;
    fn list_circles(&self) -> Result<Vec<CircleRecord>, CoreError>;
    fn find_circle(&self, name: &str) -> Result<Option<CircleRecord>, CoreError>;
    fn add_member(
        &self,
        circle: &str,
        alias: &str,
        pubkey: &str,
        wrapped_key: &[u8],
    ) -> Result<(), CoreError>;
    fn remove_member(&self, circle: &str, alias: &str) -> Result<(), CoreError>;
    fn list_members(&self, circle: &str) -> Result<Vec<CircleMemberRecord>, CoreError>;
    fn find_member_wrapped_key(
        &self,
        circle: &str,
        pubkey: &str,
    ) -> Result<Option<Vec<u8>>, CoreError>;
}

/// Queued operation types for offline push/pull/delete.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QueuedOperation {
    Push { hash: ContentHash },
    Pull { hash: ContentHash },
    Delete { hash: ContentHash },
    Retract { hash: ContentHash },
}

/// Entry in the persistent operation queue.
#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: i64,
    pub operation: QueuedOperation,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error: Option<String>,
    pub retries: u32,
}

/// Persistent operation queue for offline operations.
pub trait OperationQueue: Send + Sync {
    fn enqueue(&self, op: &QueuedOperation) -> Result<i64, CoreError>;
    fn dequeue_pending(&self) -> Result<Option<QueueEntry>, CoreError>;
    fn mark_completed(&self, id: i64) -> Result<(), CoreError>;
    fn mark_failed(&self, id: i64, error: &str) -> Result<(), CoreError>;
    fn increment_retries(&self, id: i64) -> Result<(), CoreError>;
    fn list_pending(&self) -> Result<Vec<QueueEntry>, CoreError>;
    fn list_recent(&self, limit: u32) -> Result<Vec<QueueEntry>, CoreError>;
    /// Returns (pending, completed, failed) counts.
    fn count_by_status(&self) -> Result<(u32, u32, u32), CoreError>;
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
