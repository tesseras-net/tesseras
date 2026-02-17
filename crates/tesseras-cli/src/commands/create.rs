use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tesseras_core::ports::{IdentityStore, KeyAlgorithm};
use tesseras_core::{CreateInput, FileInput, MemoryType, Visibility};
use tesseras_storage::FsIdentityStore;

use base64::Engine;

use super::init::expand_tilde;

#[derive(clap::Args)]
pub struct CreateArgs {
    /// File or directory containing files to include (recursive)
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
    /// Create a sealed (time-locked) tessera
    #[arg(long)]
    pub sealed: bool,
    /// Date when sealed tessera opens (YYYY-MM-DD, requires --sealed)
    #[arg(long)]
    pub open_after: Option<String>,
    /// Language code
    #[arg(long, default_value = "en")]
    pub language: String,
    /// Tags (comma-separated)
    #[arg(long)]
    pub tags: Option<String>,
    /// Location description
    #[arg(long)]
    pub location: Option<String>,
    /// Skip publishing to network (offline only)
    #[arg(long)]
    pub no_publish: bool,
}

pub async fn run(args: &CreateArgs, data_dir: &str, socket: &Option<PathBuf>) -> Result<()> {
    let base = expand_tilde(data_dir);

    // Auto-initialize identity and database if needed
    if super::init::ensure_initialized(&base).await? {
        eprintln!("Initialized new identity at {}", base.display());
    }

    // 1. Scan input for supported files
    let files = scan_input(&args.path)?;

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
    let visibility = if args.sealed {
        let open_after_str = args
            .open_after
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--sealed requires --open-after"))?;
        let date = chrono::NaiveDate::parse_from_str(open_after_str, "%Y-%m-%d")
            .context("--open-after must be YYYY-MM-DD")?;
        let dt = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        Visibility::Sealed { open_after: dt }
    } else {
        parse_visibility(&args.visibility)?
    };

    let identity_store = FsIdentityStore::new(base.clone());
    let encryption_public = if matches!(visibility, Visibility::Sealed { .. } | Visibility::Private)
    {
        let x_mat = identity_store
            .load_keypair(KeyAlgorithm::X25519)
            .context("encryption keys not found — run `tes init --upgrade`")?;
        let m_mat = identity_store
            .load_keypair(KeyAlgorithm::MlKem768)
            .context("encryption keys not found — run `tes init --upgrade`")?;
        Some(tesseras_core::tessera::HybridEncryptionPublic {
            x25519: x_mat
                .public
                .as_slice()
                .try_into()
                .context("X25519 public key must be 32 bytes")?,
            mlkem768: m_mat.public.clone(),
        })
    } else {
        None
    };

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
        encryption_public,
    };

    // Count files by type for summary (before moving file_inputs)
    let file_summary = {
        let photo_count = files
            .iter()
            .filter(|f| {
                f.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| matches!(e.to_lowercase().as_str(), "jpg" | "jpeg" | "png"))
            })
            .count();
        let audio_count = files
            .iter()
            .filter(|f| {
                f.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.to_lowercase() == "wav")
            })
            .count();
        let video_count = files
            .iter()
            .filter(|f| {
                f.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.to_lowercase() == "webm")
            })
            .count();
        let text_count = files
            .iter()
            .filter(|f| {
                f.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.to_lowercase() == "txt")
            })
            .count();

        let mut parts = Vec::new();
        if photo_count > 0 {
            parts.push(format!(
                "{photo_count} photo{}",
                if photo_count > 1 { "s" } else { "" }
            ));
        }
        if audio_count > 0 {
            parts.push(format!("{audio_count} audio"));
        }
        if video_count > 0 {
            parts.push(format!("{video_count} video"));
        }
        if text_count > 0 {
            parts.push(format!("{text_count} text"));
        }
        parts.join(", ")
    };

    // 5. Build service and create tessera
    let service = build_service(&base)?;
    let content_hash = service.create(input).await?;

    println!();
    println!("Memory preserved.");
    println!("  Hash:  {}", content_hash.to_base32_short(8));
    println!("  Files: {file_summary}");

    // 6. Auto-publish to network (unless --no-publish)
    if !args.no_publish {
        let socket_path = match socket {
            Some(p) => p.clone(),
            None => tesseras_rpc::default_socket_path()
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        };

        // Auto-start daemon if not running
        if !super::daemon::is_daemon_running(&base) {
            eprintln!("Starting daemon...");
            super::daemon::start_daemon(&base)?;
        }

        // Publish via RPC
        match tesseras_rpc::DaemonClient::connect(&socket_path) {
            Ok(mut client) => {
                match client.call(&tesseras_rpc::Request::Publish { hash: content_hash }) {
                    Ok(tesseras_rpc::Response::Published {
                        fragments_created, ..
                    }) => {
                        println!("  Network: {fragments_created} fragments distributed");
                    }
                    Ok(_) => {
                        eprintln!("Warning: unexpected response from daemon");
                    }
                    Err(e) => {
                        eprintln!("Warning: publish failed: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: could not connect to daemon: {e}");
                eprintln!(
                    "Tessera saved locally. Publish later with: tes net publish {}",
                    content_hash.to_base32_short(8)
                );
            }
        }
    }

    println!("  Show:  tes show {}", content_hash.to_base32_short(8));
    Ok(())
}

pub fn scan_input(path: &str) -> Result<Vec<PathBuf>> {
    let path = PathBuf::from(path);

    if path.is_file() {
        if is_supported_file(&path) {
            return Ok(vec![path]);
        } else {
            anyhow::bail!(
                "unsupported file format: {}. Supported: jpg, jpeg, png, wav, webm, txt",
                path.display()
            );
        }
    }

    if !path.is_dir() {
        anyhow::bail!("{} is not a file or directory", path.display());
    }

    // Recursive directory scan
    let mut files = Vec::new();
    scan_recursive(&path, &mut files)?;
    files.sort();

    if files.is_empty() {
        anyhow::bail!(
            "no supported files found in {}. Supported: jpg, jpeg, png, wav, webm, txt",
            path.display()
        );
    }

    Ok(files)
}

fn scan_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_recursive(&path, files)?;
        } else if path.is_file() && is_supported_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

pub fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "wav" | "webm" | "txt"
            )
        })
}

pub fn infer_memory_type(path: &Path) -> MemoryType {
    match path.extension().and_then(|e| e.to_str()) {
        Some("txt") => MemoryType::Reflection,
        _ => MemoryType::Moment,
    }
}

pub fn parse_visibility(s: &str) -> Result<Visibility> {
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

    let cas = Arc::new(tesseras_storage::CasStore::new(
        Arc::clone(&conn),
        base.join("cas"),
    ));

    Ok(tesseras_core::TesseraService::new_with_encryption(
        Box::new(tesseras_storage::SqliteTesseraRepository::new(conn.clone())),
        Box::new(tesseras_storage::SqliteMemoryRepository::new(conn.clone())),
        Box::new(tesseras_storage::FsBlobStore::new(conn, cas)),
        Box::new(hasher),
        Box::new(signer),
        Box::new(verifier),
        Box::new(CryptoEncryptor),
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

struct CryptoEncryptor;
impl tesseras_core::ContentEncryptor for CryptoEncryptor {
    fn encrypt(
        &self,
        content: &[u8],
        key: &[u8; 32],
        aad: &[u8],
    ) -> Result<Vec<u8>, tesseras_core::CoreError> {
        use tesseras_core::enums::EncryptionContext;
        let content_hash = tesseras_core::ContentHash::new(aad.try_into().unwrap_or([0u8; 32]));
        let ctx = EncryptionContext::Sealed {
            content_hash,
            open_after: chrono::Utc::now(),
        };
        let blob = tesseras_crypto::encryption::Aes256GcmEncryptor::encrypt(content, key, &ctx)
            .map_err(|e| tesseras_core::CoreError::CryptoError(e.to_string()))?;
        // Serialize EncryptedBlob as nonce (12 bytes) + ciphertext
        let mut out = Vec::with_capacity(12 + blob.ciphertext.len());
        out.extend_from_slice(&blob.nonce);
        out.extend_from_slice(&blob.ciphertext);
        Ok(out)
    }

    fn generate_content_key(&self) -> [u8; 32] {
        rand::random()
    }

    fn seal_content_key(
        &self,
        content_key: &[u8; 32],
        encryption_public: &tesseras_core::tessera::HybridEncryptionPublic,
    ) -> Result<String, tesseras_core::CoreError> {
        use tesseras_crypto::kem::HybridEncryptionPublic as CryptoHEP;
        let crypto_pub = CryptoHEP {
            x25519: encryption_public.x25519,
            mlkem768: encryption_public.mlkem768.clone(),
        };
        let envelope = tesseras_crypto::sealed::SealedKeyEnvelope::seal(content_key, &crypto_pub)
            .map_err(|e| tesseras_core::CoreError::CryptoError(e.to_string()))?;
        // Serialize: x25519_ephemeral (32) + mlkem_ciphertext (variable)
        let ct = &envelope.hybrid_ciphertext;
        let mut bytes = Vec::with_capacity(32 + ct.mlkem_ciphertext.len());
        bytes.extend_from_slice(&ct.x25519_ephemeral);
        bytes.extend_from_slice(&ct.mlkem_ciphertext);
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
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
