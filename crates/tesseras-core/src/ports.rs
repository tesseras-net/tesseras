use async_trait::async_trait;

use crate::{ContentHash, CoreError};

/// Tessera metadata persistence.
#[async_trait]
pub trait TesseraRepository: Send + Sync {
    async fn store(&self, tessera: &TesseraRecord) -> Result<(), CoreError>;
    async fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<TesseraRecord>, CoreError>;
    async fn list(&self) -> Result<Vec<TesseraRecord>, CoreError>;
    async fn delete(&self, hash: &ContentHash) -> Result<(), CoreError>;
    async fn exists(&self, hash: &ContentHash) -> Result<bool, CoreError>;
}

/// Memory metadata persistence.
#[async_trait]
pub trait MemoryRepository: Send + Sync {
    async fn store(&self, memory: &MemoryRecord) -> Result<(), CoreError>;
    async fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<MemoryRecord>, CoreError>;
    async fn list_by_tessera(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<Vec<MemoryRecord>, CoreError>;
    async fn delete(&self, hash: &ContentHash) -> Result<(), CoreError>;
}

/// Raw file storage.
#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn write(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
        data: &[u8],
    ) -> Result<(), CoreError>;
    async fn read(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<Vec<u8>, CoreError>;
    async fn exists(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<bool, CoreError>;
    async fn delete_tessera(&self, tessera_hash: &ContentHash) -> Result<(), CoreError>;
}

/// Identity key persistence.
#[async_trait]
pub trait IdentityStore: Send + Sync {
    async fn save_keypair(&self, material: &KeyMaterial) -> Result<(), CoreError>;
    async fn load_keypair(&self, algorithm: KeyAlgorithm) -> Result<KeyMaterial, CoreError>;
    async fn keypair_exists(&self, algorithm: KeyAlgorithm) -> Result<bool, CoreError>;
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
