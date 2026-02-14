//! Integration tests: full create → verify → export cycle using real adapters.

use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tesseras_core::ports::{Hasher, ManifestSigner, ManifestVerifier};
use tesseras_core::{ContentHash, CreateInput, FileInput, MemoryType, TesseraService, Visibility};
use tesseras_storage::{FsBlobStore, SqliteMemoryRepository, SqliteTesseraRepository};

// Crypto adapters using blake3 and ed25519-dalek directly

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
        Self {
            signing_key: ed25519_dalek::SigningKey::generate(&mut OsRng),
        }
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
        let sig_array: [u8; 64] = match signature.try_into() {
            Ok(a) => a,
            Err(_) => return false,
        };
        let sig = ed25519_dalek::Signature::from_bytes(&sig_array);
        let pub_bytes: Vec<u8> = (0..public_key_hex.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(&public_key_hex[i..i + 2], 16).ok())
            .collect();
        if pub_bytes.len() != 32 {
            return false;
        }
        let pub_array: [u8; 32] = match pub_bytes.try_into() {
            Ok(a) => a,
            Err(_) => return false,
        };
        if let Ok(vk) = ed25519_dalek::VerifyingKey::from_bytes(&pub_array) {
            vk.verify(manifest, &sig).is_ok()
        } else {
            false
        }
    }
}

fn setup() -> (TesseraService, TempDir) {
    let dir = TempDir::new().unwrap();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    tesseras_storage::run_migrations(&conn).unwrap();
    let conn = Arc::new(Mutex::new(conn));
    let service = TesseraService::new(
        Box::new(SqliteTesseraRepository::new(conn.clone())),
        Box::new(SqliteMemoryRepository::new(conn)),
        Box::new(FsBlobStore::new(dir.path().join("blobs"))),
        Box::new(TestHasher),
        Box::new(TestSigner::new()),
        Box::new(TestVerifier),
    );
    (service, dir)
}

#[tokio::test]
async fn create_verify_export_cycle() {
    let (service, dir) = setup();

    // Write test files
    let input_dir = dir.path().join("input");
    std::fs::create_dir_all(&input_dir).unwrap();
    std::fs::write(input_dir.join("photo.jpg"), b"fake jpeg data").unwrap();

    let input = CreateInput {
        files: vec![FileInput {
            path: input_dir.join("photo.jpg"),
            context: Some("A beautiful day".to_string()),
            memory_type: MemoryType::Moment,
        }],
        visibility: Visibility::Public,
        language: "en".to_string(),
        tags: vec![],
        location: None,
    };

    // Create
    let hash = service.create(input).await.unwrap();

    // Verify
    let report = service.verify(&hash).await.unwrap();
    assert!(report.signature_valid);
    assert!(report.files.iter().all(|f| f.valid));

    // Export
    let export_dir = dir.path().join("export");
    service.export(&hash, &export_dir).await.unwrap();
    let tessera_dir = export_dir.join(format!("tessera-{hash}"));
    assert!(tessera_dir.join("MANIFEST").exists());
    assert!(tessera_dir.join("identity").exists());
    assert!(tessera_dir.join("memories").exists());
}

#[tokio::test]
async fn create_non_interactive_empty_context() {
    let (service, dir) = setup();
    let input_dir = dir.path().join("input");
    std::fs::create_dir_all(&input_dir).unwrap();
    std::fs::write(input_dir.join("note.txt"), "Some text").unwrap();

    let input = CreateInput {
        files: vec![FileInput {
            path: input_dir.join("note.txt"),
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
}

#[tokio::test]
async fn verify_detects_tampered_file() {
    let (service, dir) = setup();
    let input_dir = dir.path().join("input");
    std::fs::create_dir_all(&input_dir).unwrap();
    std::fs::write(input_dir.join("photo.jpg"), b"original data").unwrap();

    let input = CreateInput {
        files: vec![FileInput {
            path: input_dir.join("photo.jpg"),
            context: None,
            memory_type: MemoryType::Moment,
        }],
        visibility: Visibility::Public,
        language: "en".to_string(),
        tags: vec![],
        location: None,
    };

    let hash = service.create(input).await.unwrap();

    // Tamper with the stored blob
    let blob_dir = dir.path().join("blobs").join(hash.to_string());
    for entry in std::fs::read_dir(&blob_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            let media = path.join("media.jpg");
            if media.exists() {
                std::fs::write(&media, b"tampered data").unwrap();
            }
        }
    }

    let report = service.verify(&hash).await.unwrap();
    assert!(report.files.iter().any(|f| !f.valid));
}

#[tokio::test]
async fn verify_detects_bad_signature() {
    let (service, dir) = setup();
    let input_dir = dir.path().join("input");
    std::fs::create_dir_all(&input_dir).unwrap();
    std::fs::write(input_dir.join("note.txt"), b"test content").unwrap();

    let input = CreateInput {
        files: vec![FileInput {
            path: input_dir.join("note.txt"),
            context: None,
            memory_type: MemoryType::Reflection,
        }],
        visibility: Visibility::Public,
        language: "en".to_string(),
        tags: vec![],
        location: None,
    };

    let hash = service.create(input).await.unwrap();

    // Tamper with the signature (swap bytes to invalidate)
    let sig_path = dir
        .path()
        .join("blobs")
        .join(hash.to_string())
        .join(hash.to_string())
        .join("ed25519.sig");
    let mut sig_bytes = std::fs::read(&sig_path).unwrap();
    // Flip some bytes to invalidate the signature
    for b in sig_bytes.iter_mut().take(8) {
        *b ^= 0xff;
    }
    std::fs::write(&sig_path, &sig_bytes).unwrap();

    let report = service.verify(&hash).await.unwrap();
    assert!(!report.signature_valid);
}
