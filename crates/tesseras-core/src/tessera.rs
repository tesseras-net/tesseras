use serde::{Deserialize, Serialize};

use crate::{ContentHash, CoreError, Manifest, Memory};

/// Public half of a hybrid encryption keypair (X25519 + ML-KEM-768).
/// Both keys are always present together — impossible to have one without the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridEncryptionPublic {
    pub x25519: [u8; 32],
    pub mlkem768: Vec<u8>,
}

/// Metadata about heir shares created for this identity.
/// Stored in `heir_meta.json`, NOT in `TesseraIdentity` serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeirShareMeta {
    pub format_version: u8,
    #[serde(with = "hex_serde")]
    pub session_id: [u8; 8],
    pub threshold: u8,
    pub total_shares: u8,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TesseraIdentity {
    pub ed25519_public: Vec<u8>,
    pub mldsa_public: Option<Vec<u8>>,
    pub encryption_public: Option<HybridEncryptionPublic>,
    pub heir_keys: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct Tessera {
    manifest: Manifest,
    memories: Vec<Memory>,
    identity: TesseraIdentity,
}

impl Tessera {
    pub fn new(
        manifest: Manifest,
        memories: Vec<Memory>,
        identity: TesseraIdentity,
    ) -> Result<Self, CoreError> {
        if memories.is_empty() {
            return Err(CoreError::InvalidTessera(
                "at least one memory required".into(),
            ));
        }
        // Verify every manifest entry has a corresponding memory
        for entry in &manifest.entries {
            let has_memory = memories
                .iter()
                .any(|m| entry.path.contains(&m.hash.to_string()));
            if !has_memory {
                return Err(CoreError::InvalidTessera(format!(
                    "manifest entry {} has no matching memory",
                    entry.path
                )));
            }
        }
        Ok(Self {
            manifest,
            memories,
            identity,
        })
    }

    pub fn content_hash(&self) -> &ContentHash {
        &self.manifest.content_hash
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn memories(&self) -> &[Memory] {
        &self.memories
    }
    pub fn identity(&self) -> &TesseraIdentity {
        &self.identity
    }
}

mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 8], ser: S) -> Result<S::Ok, S::Error> {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        ser.serialize_str(&hex)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 8], D::Error> {
        let s = String::deserialize(de)?;
        if s.len() != 16 {
            return Err(serde::de::Error::custom("expected 16 hex chars"));
        }
        let mut bytes = [0u8; 8];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte =
                u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(serde::de::Error::custom)?;
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ManifestEntry;
    use crate::{MemoryMetadata, MemoryType, SchemaVersion};

    fn make_test_identity() -> TesseraIdentity {
        TesseraIdentity {
            ed25519_public: vec![0xaa; 32],
            mldsa_public: None,
            encryption_public: None,
            heir_keys: vec![],
        }
    }

    fn make_test_memory(hash: ContentHash) -> Memory {
        Memory {
            hash,
            media_path: "media.jpg".to_string(),
            context: None,
            metadata: MemoryMetadata {
                version: SchemaVersion::V1,
                created_at: chrono::Utc::now(),
                memory_type: MemoryType::Moment,
                location: None,
                people: vec![],
                tags: vec![],
                language: "en".to_string(),
                description: String::new(),
            },
        }
    }

    fn make_test_manifest(content_hash: ContentHash, memory_hashes: &[ContentHash]) -> Manifest {
        let entries = memory_hashes
            .iter()
            .map(|h| ManifestEntry {
                path: format!("memories/{h}/media.jpg"),
                hash: *h,
                mime_type: "image/jpeg".to_string(),
                size: 1024,
            })
            .collect();
        Manifest {
            version: SchemaVersion::V1,
            created_at: chrono::Utc::now(),
            creator: "aa".repeat(32),
            content_hash,
            entries,
        }
    }

    #[test]
    fn tessera_new_valid() {
        let memory_hash = ContentHash::new([0x01; 32]);
        let content_hash = ContentHash::new([0x02; 32]);
        let manifest = make_test_manifest(content_hash, &[memory_hash]);
        let memory = make_test_memory(memory_hash);
        let identity = make_test_identity();

        let tessera = Tessera::new(manifest, vec![memory], identity);
        assert!(tessera.is_ok());
    }

    #[test]
    fn tessera_new_rejects_zero_memories() {
        let content_hash = ContentHash::new([0x02; 32]);
        let manifest = make_test_manifest(content_hash, &[]);
        let identity = make_test_identity();

        let tessera = Tessera::new(manifest, vec![], identity);
        assert!(tessera.is_err());
    }

    #[test]
    fn tessera_new_rejects_mismatched_manifest() {
        let memory_hash = ContentHash::new([0x01; 32]);
        let other_hash = ContentHash::new([0x99; 32]);
        let content_hash = ContentHash::new([0x02; 32]);
        let manifest = make_test_manifest(content_hash, &[other_hash]);
        let memory = make_test_memory(memory_hash);
        let identity = make_test_identity();

        let tessera = Tessera::new(manifest, vec![memory], identity);
        assert!(tessera.is_err());
    }

    #[test]
    fn tessera_content_hash_accessor() {
        let memory_hash = ContentHash::new([0x01; 32]);
        let content_hash = ContentHash::new([0x02; 32]);
        let manifest = make_test_manifest(content_hash, &[memory_hash]);
        let memory = make_test_memory(memory_hash);
        let identity = make_test_identity();

        let tessera = Tessera::new(manifest, vec![memory], identity).unwrap();
        assert_eq!(tessera.content_hash(), &content_hash);
    }
}
