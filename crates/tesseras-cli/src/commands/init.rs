use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tesseras_core::ports::{IdentityStore, KeyAlgorithm, KeyMaterial};
use tesseras_crypto::ed25519::Ed25519KeyGenerator;
use tesseras_crypto::kem::HybridKem;
use tesseras_storage::FsIdentityStore;

const DEFAULT_CONFIG: &str = r#"# Tesseras configuration
[node]
# data_dir is set by --data-dir flag or TESSERAS_DATA_DIR env var
"#;

pub async fn run(data_dir: &str, upgrade: bool) -> Result<()> {
    let base = expand_tilde(data_dir);

    // Check for legacy data location
    let legacy_path = dirs::home_dir().map(|h| h.join(".tesseras"));
    if let Some(ref legacy) = legacy_path {
        if legacy.join("db/tesseras.db").exists() && base != *legacy {
            eprintln!(
                "Note: found existing data at {}. Consider moving it to {}",
                legacy.display(),
                base.display()
            );
        }
    }

    // 1. Create directory structure
    tokio::fs::create_dir_all(base.join("identity")).await?;
    tokio::fs::create_dir_all(base.join("db")).await?;
    tokio::fs::create_dir_all(base.join("blobs")).await?;

    let identity_store = FsIdentityStore::new(base.clone());

    if upgrade {
        // Upgrade mode: only add missing encryption keys
        if !identity_store.keypair_exists(KeyAlgorithm::Ed25519)? {
            anyhow::bail!("no Ed25519 identity found — run `tes init` first (without --upgrade)");
        }
        generate_encryption_keys_atomic(&identity_store)?;
    } else {
        // 2. Generate Ed25519 keypair
        if !identity_store.keypair_exists(KeyAlgorithm::Ed25519)? {
            let keypair = Ed25519KeyGenerator::generate();
            let material: KeyMaterial = (&keypair).into();
            identity_store.save_keypair(&material)?;
            println!("Generated Ed25519 identity");
        } else {
            println!("Ed25519 identity already exists");
        }

        // 3. Generate encryption keypair (X25519 + ML-KEM-768)
        generate_encryption_keys_atomic(&identity_store)?;

        // 4. Initialize SQLite with WAL mode
        let db_path = base.join("db/tesseras.db");
        let conn =
            tesseras_storage::open_database(&db_path, &tesseras_storage::StorageConfig::default())
                .context("failed to open database")?;
        drop(conn);
        println!("Database initialized");

        // 5. Write default config.toml if not present
        let config_path = base.join("config.toml");
        if !config_path.exists() {
            tokio::fs::write(&config_path, DEFAULT_CONFIG).await?;
            println!("Config written to {}", config_path.display());
        }
    }

    println!("Tesseras initialized at {}", base.display());
    Ok(())
}

/// Generate X25519 + ML-KEM-768 keypair atomically.
/// If either key already exists (both present), skip.
/// If ML-KEM write fails after X25519 was written, roll back X25519.
fn generate_encryption_keys_atomic(store: &FsIdentityStore) -> Result<()> {
    let has_x25519 = store.keypair_exists(KeyAlgorithm::X25519)?;
    let has_mlkem = store.keypair_exists(KeyAlgorithm::MlKem768)?;

    if has_x25519 && has_mlkem {
        println!("Encryption keys already exist");
        return Ok(());
    }

    if has_x25519 != has_mlkem {
        anyhow::bail!(
            "inconsistent encryption keys: X25519={has_x25519}, ML-KEM-768={has_mlkem}. \
             Remove both from identity/ and re-run init --upgrade"
        );
    }

    println!("Generating encryption keypair (X25519 + ML-KEM-768)...");
    let hybrid = HybridKem::generate_keypair();

    // Save X25519 first
    let x25519_material = KeyMaterial {
        algorithm: KeyAlgorithm::X25519,
        secret: hybrid.x25519_secret.to_bytes().to_vec(),
        public: hybrid.x25519_public.to_bytes().to_vec(),
    };
    store
        .save_keypair(&x25519_material)
        .context("failed to save X25519 keypair")?;

    // Save ML-KEM-768, roll back X25519 on failure
    let mlkem_material = KeyMaterial {
        algorithm: KeyAlgorithm::MlKem768,
        secret: hybrid.mlkem_secret.clone(),
        public: hybrid.mlkem_public.clone(),
    };
    if let Err(e) = store.save_keypair(&mlkem_material) {
        // Roll back: delete the X25519 files we just wrote
        eprintln!("ML-KEM-768 save failed, rolling back X25519: {e}");
        let _ = std::fs::remove_file(store.key_path_public(KeyAlgorithm::X25519));
        let _ = std::fs::remove_file(store.key_path_secret(KeyAlgorithm::X25519));
        return Err(e).context("failed to save ML-KEM-768 keypair");
    }

    println!("Generated encryption keypair");
    Ok(())
}

/// Ensure identity and database are initialized at `base`. Creates them silently if missing.
/// Returns Ok(true) if initialization was performed, Ok(false) if already initialized.
pub async fn ensure_initialized(base: &Path) -> Result<bool> {
    let identity_store = FsIdentityStore::new(base.to_path_buf());

    if identity_store.keypair_exists(KeyAlgorithm::Ed25519)? {
        return Ok(false); // already initialized
    }

    // Create directory structure
    tokio::fs::create_dir_all(base.join("identity")).await?;
    tokio::fs::create_dir_all(base.join("db")).await?;
    tokio::fs::create_dir_all(base.join("blobs")).await?;

    // Generate Ed25519 keypair
    let keypair = Ed25519KeyGenerator::generate();
    let material: KeyMaterial = (&keypair).into();
    identity_store.save_keypair(&material)?;

    // Generate encryption keypair
    generate_encryption_keys_atomic(&identity_store)?;

    // Initialize SQLite
    let db_path = base.join("db/tesseras.db");
    let conn =
        tesseras_storage::open_database(&db_path, &tesseras_storage::StorageConfig::default())
            .context("failed to open database")?;
    drop(conn);

    Ok(true)
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs_home() {
            return home.join(rest);
        }
    } else if path == "~" {
        if let Some(home) = dirs_home() {
            return home;
        }
    }
    PathBuf::from(path)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
