use std::path::Path;

use crate::enums::{MemoryType, SchemaVersion, Visibility};
use crate::manifest::{Manifest, ManifestEntry};
use crate::metadata::{Location, MemoryMetadata};
use crate::ports::{
    BlobStore, Hasher, ManifestSigner, ManifestVerifier, MemoryRecord, MemoryRepository,
    TesseraRecord, TesseraRepository,
};
use crate::{ContentHash, CoreError};

pub struct CreateInput {
    pub files: Vec<FileInput>,
    pub visibility: Visibility,
    pub language: String,
    pub tags: Vec<String>,
    pub location: Option<Location>,
}

pub struct FileInput {
    pub path: std::path::PathBuf,
    pub context: Option<String>,
    pub memory_type: MemoryType,
}

pub struct VerifyReport {
    pub tessera_hash: ContentHash,
    pub signature_valid: bool,
    pub files: Vec<FileVerification>,
}

pub struct FileVerification {
    pub path: String,
    pub expected_hash: ContentHash,
    pub actual_hash: ContentHash,
    pub valid: bool,
}

pub struct TesseraService {
    repo: Box<dyn TesseraRepository>,
    memory_repo: Box<dyn MemoryRepository>,
    blobs: Box<dyn BlobStore>,
    hasher: Box<dyn Hasher>,
    signer: Box<dyn ManifestSigner>,
    verifier: Box<dyn ManifestVerifier>,
}

impl TesseraService {
    pub fn new(
        repo: Box<dyn TesseraRepository>,
        memory_repo: Box<dyn MemoryRepository>,
        blobs: Box<dyn BlobStore>,
        hasher: Box<dyn Hasher>,
        signer: Box<dyn ManifestSigner>,
        verifier: Box<dyn ManifestVerifier>,
    ) -> Self {
        Self {
            repo,
            memory_repo,
            blobs,
            hasher,
            signer,
            verifier,
        }
    }

    pub async fn create(&self, input: CreateInput) -> Result<ContentHash, CoreError> {
        if input.files.is_empty() {
            return Err(CoreError::InvalidTessera("no files provided".into()));
        }

        // 1. For each file: read bytes, hash with hasher
        let mut memory_entries = Vec::new();
        for file in &input.files {
            let data = tokio::fs::read(&file.path).await?;
            let memory_hash = self.hasher.hash(&data);
            let ext = file
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin");
            let media_name = format!("media.{ext}");
            let mime_type = mime_from_ext(ext);

            memory_entries.push((memory_hash, data, media_name, mime_type, file));
        }

        // 2. Sort memory hashes, concatenate, hash → content_hash
        let mut sorted_hashes: Vec<ContentHash> =
            memory_entries.iter().map(|(h, _, _, _, _)| *h).collect();
        sorted_hashes.sort_by_key(|a| a.to_string());
        let mut hash_concat = Vec::new();
        for h in &sorted_hashes {
            hash_concat.extend_from_slice(h.as_bytes());
        }
        let content_hash = self.hasher.hash(&hash_concat);

        // 3. Build manifest entries and store blobs
        let mut manifest_entries = Vec::new();
        let mut memory_records = Vec::new();

        for (memory_hash, data, media_name, mime_type, file) in &memory_entries {
            let path = format!("memories/{memory_hash}/{media_name}");
            manifest_entries.push(ManifestEntry {
                path: path.clone(),
                hash: *memory_hash,
                mime_type: mime_type.clone(),
                size: data.len() as u64,
            });

            // Store media blob
            self.blobs
                .write(&content_hash, memory_hash, media_name, data)?;

            // Store context if present
            if let Some(ctx) = &file.context {
                self.blobs
                    .write(&content_hash, memory_hash, "context.txt", ctx.as_bytes())?;
            }

            // Store metadata JSON
            let meta = MemoryMetadata {
                version: SchemaVersion::V1,
                created_at: chrono::Utc::now(),
                memory_type: file.memory_type,
                location: input.location.clone(),
                people: vec![],
                tags: input.tags.clone(),
                language: input.language.clone(),
                description: file.context.clone().unwrap_or_default(),
            };
            let meta_json = serde_json::to_string_pretty(&meta)?;
            self.blobs.write(
                &content_hash,
                memory_hash,
                "meta.json",
                meta_json.as_bytes(),
            )?;

            // Collect memory record for DB (stored after tessera)
            memory_records.push(MemoryRecord {
                hash: *memory_hash,
                tessera_hash: content_hash,
                memory_type: serde_json::to_string(&file.memory_type)
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string(),
                media_path: path,
                context_path: file
                    .context
                    .as_ref()
                    .map(|_| format!("memories/{memory_hash}/context.txt")),
                meta_json: Some(meta_json),
                created_at: chrono::Utc::now(),
            });
        }

        // 4. Build and sign manifest
        let (_, pub_key_hex) = self.signer.sign(b""); // Get the public key
        let now = chrono::Utc::now();
        let manifest = Manifest {
            version: SchemaVersion::V1,
            created_at: now,
            creator: pub_key_hex.clone(),
            content_hash,
            entries: manifest_entries,
        };
        let manifest_text = manifest.to_string();
        let (sig_bytes, _) = self.signer.sign(manifest_text.as_bytes());

        // Store manifest and signature at well-known location: content_hash/content_hash/
        self.blobs.write(
            &content_hash,
            &content_hash,
            "MANIFEST",
            manifest_text.as_bytes(),
        )?;
        self.blobs
            .write(&content_hash, &content_hash, "ed25519.sig", &sig_bytes)?;

        // 5. Store tessera record in DB first (FK parent)
        let total_size: u64 = memory_entries
            .iter()
            .map(|(_, data, _, _, _)| data.len() as u64)
            .sum();
        let vis_str = input.visibility.to_string();
        let tessera_record = TesseraRecord {
            hash: content_hash,
            creator_pubkey: pub_key_hex,
            created_at: now,
            size_bytes: total_size,
            memory_count: memory_entries.len() as u32,
            visibility: vis_str,
            sealed_until: match &input.visibility {
                Visibility::Sealed { open_after } => Some(*open_after),
                _ => None,
            },
            is_mine: true,
        };
        self.repo.store(&tessera_record)?;

        // 6. Store memory records in DB (FK children)
        for record in &memory_records {
            self.memory_repo.store(record)?;
        }

        Ok(content_hash)
    }

    pub async fn verify(&self, hash: &ContentHash) -> Result<VerifyReport, CoreError> {
        // 1. Load TesseraRecord from DB
        let tessera = self
            .repo
            .find_by_hash(hash)?
            .ok_or_else(|| CoreError::InvalidTessera(format!("tessera not found: {hash}")))?;

        let _memories = self.memory_repo.list_by_tessera(hash)?;

        // 2. Read manifest from well-known location: content_hash/content_hash/MANIFEST
        let manifest_data = self
            .blobs
            .read(hash, hash, "MANIFEST")
            .map_err(|_| CoreError::InvalidTessera("manifest not found in blob store".into()))?;
        let manifest_text = String::from_utf8_lossy(&manifest_data).to_string();

        // 3. Parse manifest
        let manifest = Manifest::parse(&manifest_text)?;

        // 4. Verify signature
        let mut signature_valid = false;
        if let Ok(sig_bytes) = self.blobs.read(hash, hash, "ed25519.sig") {
            signature_valid = self.verifier.verify(
                manifest_text.as_bytes(),
                &sig_bytes,
                &tessera.creator_pubkey,
            );
        }

        // 5. Verify each file's hash
        let mut file_verifications = Vec::new();
        for entry in &manifest.entries {
            let parts: Vec<&str> = entry.path.split('/').collect();
            if parts.len() >= 3 {
                let memory_hash_str = parts[1];
                let filename = parts[2..].join("/");
                if let Ok(memory_hash) = ContentHash::from_str(memory_hash_str) {
                    match self.blobs.read(hash, &memory_hash, &filename) {
                        Ok(data) => {
                            let actual_hash = self.hasher.hash(&data);
                            let valid = actual_hash == entry.hash;
                            file_verifications.push(FileVerification {
                                path: entry.path.clone(),
                                expected_hash: entry.hash,
                                actual_hash,
                                valid,
                            });
                        }
                        Err(_) => {
                            file_verifications.push(FileVerification {
                                path: entry.path.clone(),
                                expected_hash: entry.hash,
                                actual_hash: ContentHash::new([0; 32]),
                                valid: false,
                            });
                        }
                    }
                }
            }
        }

        Ok(VerifyReport {
            tessera_hash: *hash,
            signature_valid,
            files: file_verifications,
        })
    }

    pub async fn export(&self, hash: &ContentHash, dest: &Path) -> Result<(), CoreError> {
        let _tessera = self
            .repo
            .find_by_hash(hash)?
            .ok_or_else(|| CoreError::InvalidTessera(format!("tessera not found: {hash}")))?;

        let tessera_dir = dest.join(format!("tessera-{hash}"));
        tokio::fs::create_dir_all(&tessera_dir).await?;

        let memories = self.memory_repo.list_by_tessera(hash)?;

        let identity_dir = tessera_dir.join("identity");
        tokio::fs::create_dir_all(&identity_dir).await?;

        // Read manifest and signature from well-known location
        if let Ok(manifest_data) = self.blobs.read(hash, hash, "MANIFEST") {
            tokio::fs::write(tessera_dir.join("MANIFEST"), &manifest_data).await?;
        }
        if let Ok(sig_data) = self.blobs.read(hash, hash, "ed25519.sig") {
            tokio::fs::write(identity_dir.join("ed25519.sig"), &sig_data).await?;
        }

        // Copy all memory files
        let memories_dir = tessera_dir.join("memories");
        tokio::fs::create_dir_all(&memories_dir).await?;

        for mem in &memories {
            let mem_dir = memories_dir.join(mem.hash.to_string());
            tokio::fs::create_dir_all(&mem_dir).await?;

            let media_filename = mem.media_path.split('/').next_back().unwrap_or("media.bin");
            if let Ok(data) = self.blobs.read(hash, &mem.hash, media_filename) {
                tokio::fs::write(mem_dir.join(media_filename), &data).await?;
            }
            if mem.context_path.is_some() {
                if let Ok(data) = self.blobs.read(hash, &mem.hash, "context.txt") {
                    tokio::fs::write(mem_dir.join("context.txt"), &data).await?;
                }
            }
            if let Ok(data) = self.blobs.read(hash, &mem.hash, "meta.json") {
                tokio::fs::write(mem_dir.join("meta.json"), &data).await?;
            }
        }

        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<TesseraRecord>, CoreError> {
        self.repo.list()
    }

    pub fn resolve_prefix(
        &self,
        prefix: &crate::types::HashPrefix,
    ) -> Result<TesseraRecord, CoreError> {
        match prefix {
            crate::types::HashPrefix::Exact(hash) => self
                .repo
                .find_by_hash(hash)?
                .ok_or_else(|| CoreError::PrefixNotFound(hash.to_base32())),
            crate::types::HashPrefix::HexPrefix(hex_prefix) => {
                let matches = self.repo.find_by_hex_prefix(hex_prefix)?;
                Self::disambiguate(matches, hex_prefix)
            }
            crate::types::HashPrefix::Base32Prefix {
                hex_prefix,
                base32_prefix,
            } => {
                let candidates = self.repo.find_by_hex_prefix(hex_prefix)?;
                let matches: Vec<_> = candidates
                    .into_iter()
                    .filter(|t| t.hash.to_base32().starts_with(base32_prefix))
                    .collect();
                Self::disambiguate(matches, base32_prefix)
            }
        }
    }

    fn disambiguate(matches: Vec<TesseraRecord>, prefix: &str) -> Result<TesseraRecord, CoreError> {
        match matches.len() {
            0 => Err(CoreError::PrefixNotFound(prefix.to_string())),
            1 => Ok(matches.into_iter().next().unwrap()),
            n => Err(CoreError::AmbiguousPrefix {
                prefix: prefix.to_string(),
                count: n,
            }),
        }
    }
}

fn mime_from_ext(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "png" => "image/png".to_string(),
        "wav" => "audio/wav".to_string(),
        "webm" => "video/webm".to_string(),
        "txt" => "text/plain".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // In-memory implementations for testing

    struct InMemoryTesseraRepo {
        data: Mutex<HashMap<String, TesseraRecord>>,
    }

    impl InMemoryTesseraRepo {
        fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    impl TesseraRepository for InMemoryTesseraRepo {
        fn store(&self, tessera: &TesseraRecord) -> Result<(), CoreError> {
            self.data
                .lock()
                .unwrap()
                .insert(tessera.hash.to_string(), tessera.clone());
            Ok(())
        }
        fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<TesseraRecord>, CoreError> {
            Ok(self.data.lock().unwrap().get(&hash.to_string()).cloned())
        }
        fn find_by_hex_prefix(&self, hex_prefix: &str) -> Result<Vec<TesseraRecord>, CoreError> {
            Ok(self
                .data
                .lock()
                .unwrap()
                .iter()
                .filter(|(k, _)| k.starts_with(hex_prefix))
                .map(|(_, v)| v.clone())
                .collect())
        }
        fn list(&self) -> Result<Vec<TesseraRecord>, CoreError> {
            Ok(self.data.lock().unwrap().values().cloned().collect())
        }
        fn delete(&self, hash: &ContentHash) -> Result<(), CoreError> {
            self.data.lock().unwrap().remove(&hash.to_string());
            Ok(())
        }
        fn exists(&self, hash: &ContentHash) -> Result<bool, CoreError> {
            Ok(self.data.lock().unwrap().contains_key(&hash.to_string()))
        }
    }

    struct InMemoryMemoryRepo {
        data: Mutex<HashMap<String, MemoryRecord>>,
    }

    impl InMemoryMemoryRepo {
        fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    impl MemoryRepository for InMemoryMemoryRepo {
        fn store(&self, memory: &MemoryRecord) -> Result<(), CoreError> {
            self.data
                .lock()
                .unwrap()
                .insert(memory.hash.to_string(), memory.clone());
            Ok(())
        }
        fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<MemoryRecord>, CoreError> {
            Ok(self.data.lock().unwrap().get(&hash.to_string()).cloned())
        }
        fn list_by_tessera(
            &self,
            tessera_hash: &ContentHash,
        ) -> Result<Vec<MemoryRecord>, CoreError> {
            let th = tessera_hash.to_string();
            Ok(self
                .data
                .lock()
                .unwrap()
                .values()
                .filter(|m| m.tessera_hash.to_string() == th)
                .cloned()
                .collect())
        }
        fn delete(&self, hash: &ContentHash) -> Result<(), CoreError> {
            self.data.lock().unwrap().remove(&hash.to_string());
            Ok(())
        }
    }

    struct InMemoryBlobStore {
        data: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl InMemoryBlobStore {
        fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }

        fn key(t: &ContentHash, m: &ContentHash, name: &str) -> String {
            format!("{}/{}/{}", t, m, name)
        }
    }

    impl BlobStore for InMemoryBlobStore {
        fn write(
            &self,
            tessera_hash: &ContentHash,
            memory_hash: &ContentHash,
            name: &str,
            data: &[u8],
        ) -> Result<(), CoreError> {
            let key = Self::key(tessera_hash, memory_hash, name);
            self.data.lock().unwrap().insert(key, data.to_vec());
            Ok(())
        }
        fn read(
            &self,
            tessera_hash: &ContentHash,
            memory_hash: &ContentHash,
            name: &str,
        ) -> Result<Vec<u8>, CoreError> {
            let key = Self::key(tessera_hash, memory_hash, name);
            self.data.lock().unwrap().get(&key).cloned().ok_or_else(|| {
                CoreError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                ))
            })
        }
        fn exists(
            &self,
            tessera_hash: &ContentHash,
            memory_hash: &ContentHash,
            name: &str,
        ) -> Result<bool, CoreError> {
            let key = Self::key(tessera_hash, memory_hash, name);
            Ok(self.data.lock().unwrap().contains_key(&key))
        }
        fn delete_tessera(&self, tessera_hash: &ContentHash) -> Result<(), CoreError> {
            let prefix = tessera_hash.to_string();
            self.data
                .lock()
                .unwrap()
                .retain(|k, _| !k.starts_with(&prefix));
            Ok(())
        }
    }

    // Test crypto implementations using blake3 and ed25519-dalek directly
    // (avoids diamond dependency with tesseras-crypto -> tesseras-core)

    struct TestHasher;
    impl Hasher for TestHasher {
        fn hash(&self, data: &[u8]) -> ContentHash {
            let hash = blake3::hash(data);
            ContentHash::new(*hash.as_bytes())
        }
    }

    struct TestSigner {
        signing_key: ed25519_dalek::SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            use rand::rngs::OsRng;
            let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
            Self { signing_key }
        }

        fn pub_key_hex(&self) -> String {
            self.signing_key
                .verifying_key()
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        }
    }

    impl ManifestSigner for TestSigner {
        fn sign(&self, manifest: &[u8]) -> (Vec<u8>, String) {
            use ed25519_dalek::Signer;
            let sig = self.signing_key.sign(manifest);
            (sig.to_bytes().to_vec(), self.pub_key_hex())
        }
    }

    struct TestVerifier;
    impl ManifestVerifier for TestVerifier {
        fn verify(&self, manifest: &[u8], signature: &[u8], public_key_hex: &str) -> bool {
            use ed25519_dalek::Verifier;
            if signature.len() != 64 {
                return false;
            }
            let sig_array: [u8; 64] = signature.try_into().unwrap();
            let sig = ed25519_dalek::Signature::from_bytes(&sig_array);
            let pub_bytes: Vec<u8> = (0..public_key_hex.len())
                .step_by(2)
                .filter_map(|i| u8::from_str_radix(&public_key_hex[i..i + 2], 16).ok())
                .collect();
            if pub_bytes.len() != 32 {
                return false;
            }
            let pub_array: [u8; 32] = pub_bytes.try_into().unwrap();
            if let Ok(vk) = ed25519_dalek::VerifyingKey::from_bytes(&pub_array) {
                vk.verify(manifest, &sig).is_ok()
            } else {
                false
            }
        }
    }

    fn build_service() -> TesseraService {
        TesseraService::new(
            Box::new(InMemoryTesseraRepo::new()),
            Box::new(InMemoryMemoryRepo::new()),
            Box::new(InMemoryBlobStore::new()),
            Box::new(TestHasher),
            Box::new(TestSigner::new()),
            Box::new(TestVerifier),
        )
    }

    #[tokio::test]
    async fn create_single_memory_tessera() {
        let service = build_service();

        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("photo.jpg");
        std::fs::write(&file_path, b"fake jpeg data").unwrap();

        let input = CreateInput {
            files: vec![FileInput {
                path: file_path,
                context: Some("A beautiful day".to_string()),
                memory_type: MemoryType::Moment,
            }],
            visibility: Visibility::Public,
            language: "en".to_string(),
            tags: vec![],
            location: None,
        };

        let hash = service.create(input).await.unwrap();
        assert_ne!(hash, ContentHash::new([0; 32]));

        let list = service.list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].hash, hash);
    }

    #[tokio::test]
    async fn create_rejects_empty_input() {
        let service = build_service();

        let input = CreateInput {
            files: vec![],
            visibility: Visibility::Public,
            language: "en".to_string(),
            tags: vec![],
            location: None,
        };

        let result = service.create(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_and_verify() {
        let service = build_service();

        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("note.txt");
        std::fs::write(&file_path, b"Some text content").unwrap();

        let input = CreateInput {
            files: vec![FileInput {
                path: file_path,
                context: None,
                memory_type: MemoryType::Reflection,
            }],
            visibility: Visibility::Public,
            language: "en".to_string(),
            tags: vec![],
            location: None,
        };

        let hash = service.create(input).await.unwrap();
        let report = service.verify(&hash).await.unwrap();
        assert!(report.signature_valid);
        assert!(report.files.iter().all(|f| f.valid));
    }
}
