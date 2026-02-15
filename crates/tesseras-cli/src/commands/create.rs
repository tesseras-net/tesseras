use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tesseras_core::ports::{IdentityStore, KeyAlgorithm};
use tesseras_core::{CreateInput, FileInput, MemoryType, Visibility};
use tesseras_storage::FsIdentityStore;

use super::init::expand_tilde;

#[derive(clap::Args)]
pub struct CreateArgs {
    /// Directory containing files to include
    pub path: String,
    /// Non-interactive mode (skip prompts)
    #[arg(short = 'n', long)]
    pub non_interactive: bool,
    /// Dry run (show what would be created)
    #[arg(long)]
    pub dry_run: bool,
    /// Visibility (public, private, circle)
    #[arg(long, default_value = "public")]
    pub visibility: String,
    /// Language code
    #[arg(long, default_value = "en")]
    pub language: String,
    /// Tags (comma-separated)
    #[arg(long)]
    pub tags: Option<String>,
    /// Location description
    #[arg(long)]
    pub location: Option<String>,
}

pub async fn run(args: &CreateArgs, data_dir: &str) -> Result<()> {
    let base = expand_tilde(data_dir);

    // 1. Scan input directory for supported files
    let files = scan_directory(&args.path)?;
    if files.is_empty() {
        anyhow::bail!("No supported files found in {}", args.path);
    }

    // 2. Dry run: just print what would happen
    if args.dry_run {
        println!("Dry run — files that would be included:");
        for f in &files {
            let mt = infer_memory_type(f);
            println!("  {} ({:?})", f.display(), mt);
        }
        return Ok(());
    }

    // 3. Build FileInput list
    let file_inputs: Vec<FileInput> = files
        .iter()
        .map(|f| FileInput {
            path: f.clone(),
            context: None,
            memory_type: infer_memory_type(f),
        })
        .collect();

    // 4. Build CreateInput
    let visibility = parse_visibility(&args.visibility)?;
    let tags = args
        .tags
        .as_deref()
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    let location = args
        .location
        .as_ref()
        .map(|l| tesseras_core::metadata::Location {
            description: l.clone(),
            coordinates: None,
        });

    let input = CreateInput {
        files: file_inputs,
        visibility,
        language: args.language.clone(),
        tags,
        location,
        encryption_public: None,
    };

    // 5. Build service and create tessera
    let service = build_service(&base)?;
    let content_hash = service.create(input).await?;
    println!("Created tessera: {}", content_hash.to_base32());
    Ok(())
}

fn scan_directory(path: &str) -> Result<Vec<PathBuf>> {
    let dir = PathBuf::from(path);
    if !dir.is_dir() {
        anyhow::bail!("{} is not a directory", path);
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext.to_lowercase().as_str() {
                    "jpg" | "jpeg" | "png" | "wav" | "webm" | "txt" => {
                        files.push(path);
                    }
                    _ => {}
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

fn infer_memory_type(path: &Path) -> MemoryType {
    match path.extension().and_then(|e| e.to_str()) {
        Some("txt") => MemoryType::Reflection,
        _ => MemoryType::Moment,
    }
}

fn parse_visibility(s: &str) -> Result<Visibility> {
    match s.to_lowercase().as_str() {
        "public" => Ok(Visibility::Public),
        "private" => Ok(Visibility::Private),
        "circle" => Ok(Visibility::Circle),
        other => anyhow::bail!("unknown visibility: {other}"),
    }
}

pub fn build_service(base: &Path) -> Result<tesseras_core::TesseraService> {
    let db_path = base.join("db/tesseras.db");
    let conn =
        tesseras_storage::open_database(&db_path, &tesseras_storage::StorageConfig::default())
            .context("failed to open database")?;
    let conn = Arc::new(Mutex::new(conn));

    let identity_store = FsIdentityStore::new(base.to_path_buf());
    let key_material = identity_store
        .load_keypair(KeyAlgorithm::Ed25519)
        .map_err(|e| anyhow::anyhow!("no identity found — run 'tesseras init' first: {e}"))?;
    let keypair = tesseras_crypto::ed25519::Ed25519KeyPair::try_from(&key_material)
        .map_err(|e| anyhow::anyhow!("invalid key material: {e}"))?;

    let signer = CryptoSigner { keypair };
    let verifier = CryptoVerifier;
    let hasher = CryptoHasher;

    Ok(tesseras_core::TesseraService::new(
        Box::new(tesseras_storage::SqliteTesseraRepository::new(conn.clone())),
        Box::new(tesseras_storage::SqliteMemoryRepository::new(conn)),
        Box::new(tesseras_storage::FsBlobStore::new(base.join("blobs"))),
        Box::new(hasher),
        Box::new(signer),
        Box::new(verifier),
    ))
}

struct CryptoHasher;
impl tesseras_core::Hasher for CryptoHasher {
    fn hash(&self, data: &[u8]) -> tesseras_core::ContentHash {
        tesseras_crypto::hasher::Blake3Hasher::hash(data)
    }
}

struct CryptoSigner {
    keypair: tesseras_crypto::ed25519::Ed25519KeyPair,
}

impl tesseras_core::ManifestSigner for CryptoSigner {
    fn sign(&self, manifest: &[u8]) -> (Vec<u8>, String) {
        use ed25519_dalek::Signer;
        let sig = self.keypair.signing_key.sign(manifest);
        let pub_hex: String = self
            .keypair
            .verifying_key
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        (sig.to_bytes().to_vec(), pub_hex)
    }
}

struct CryptoVerifier;
impl tesseras_core::ManifestVerifier for CryptoVerifier {
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
