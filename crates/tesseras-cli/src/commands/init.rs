use anyhow::{Context, Result};
use std::path::PathBuf;
use tesseras_core::ports::{IdentityStore, KeyAlgorithm};
use tesseras_crypto::ed25519::Ed25519KeyGenerator;
use tesseras_storage::FsIdentityStore;

const DEFAULT_CONFIG: &str = r#"# Tesseras configuration
[node]
# data_dir is set by --data-dir flag or TESSERAS_DATA_DIR env var
"#;

pub async fn run(data_dir: &str) -> Result<()> {
    let base = expand_tilde(data_dir);

    // 1. Create directory structure
    tokio::fs::create_dir_all(base.join("identity")).await?;
    tokio::fs::create_dir_all(base.join("db")).await?;
    tokio::fs::create_dir_all(base.join("blobs")).await?;

    // 2. Generate Ed25519 keypair
    let identity_store = FsIdentityStore::new(base.clone());
    if !identity_store.keypair_exists(KeyAlgorithm::Ed25519)? {
        let keypair = Ed25519KeyGenerator::generate();
        let material: tesseras_core::ports::KeyMaterial = (&keypair).into();
        identity_store.save_keypair(&material)?;
        println!("Generated Ed25519 identity");
    } else {
        println!("Ed25519 identity already exists");
    }

    // 3. Initialize SQLite
    let db_path = base.join("db/tesseras.db");
    let conn = rusqlite::Connection::open(&db_path)
        .context("failed to open database")?;
    tesseras_storage::run_migrations(&conn)
        .context("failed to run migrations")?;
    println!("Database initialized");

    // 4. Write default config.toml if not present
    let config_path = base.join("config.toml");
    if !config_path.exists() {
        tokio::fs::write(&config_path, DEFAULT_CONFIG).await?;
        println!("Config written to {}", config_path.display());
    }

    println!("Tesseras initialized at {}", base.display());
    Ok(())
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
