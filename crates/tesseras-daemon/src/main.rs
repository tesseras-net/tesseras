//! tesd: full node binary for desktop/server/RPi.

#[allow(dead_code)]
mod bootstrap;
mod config;
mod dht_adapter;
mod institutional;
mod metrics;

use tesseras_daemon::rpc;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::Parser;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

use rand::Rng;
use tesseras_core::ports::ReplicationHandler;
use tesseras_core::replication::{Attestation, FragmentEnvelope, ReplicateAck};
use tesseras_core::types::NodeId;
use tesseras_core::{ContentHash, CoreError};
use tesseras_dht::engine::DhtEngine;
use tesseras_dht::pow;
use tesseras_net::{QuinnTransport, Transport};
use tesseras_replication::ReplicationService;
use tesseras_storage::{FsBlobStore, FsFragmentStore, SqliteReciprocityLedger};

use config::DaemonConfig;
use dht_adapter::DhtPortAdapter;

#[derive(Parser, Debug)]
#[command(name = "tesd", about = "Tesseras P2P daemon")]
struct Cli {
    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Listen addresses (comma-separated, overrides config).
    /// e.g. --listen "0.0.0.0:4433,[::]:4433"
    #[arg(short, long, value_delimiter = ',')]
    listen: Vec<SocketAddr>,

    /// Bootstrap addresses (comma-separated, overrides config)
    #[arg(short, long)]
    bootstrap: Option<String>,

    /// Data directory (overrides config)
    #[arg(short, long)]
    data_dir: Option<PathBuf>,
}

/// Bridges incoming REPLICATE/ATTEST RPCs from the DhtEngine to the ReplicationService.
struct ReplicationHandlerAdapter {
    service: Arc<ReplicationService>,
}

#[async_trait::async_trait]
impl ReplicationHandler for ReplicationHandlerAdapter {
    async fn handle_replicate(
        &self,
        envelope: FragmentEnvelope,
        sender: &NodeId,
    ) -> Result<ReplicateAck, CoreError> {
        self.service
            .receive_fragment(envelope, sender)
            .await
            .map_err(|e| CoreError::Network(e.to_string()))
    }

    async fn handle_attest_request(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<Attestation, CoreError> {
        self.service
            .handle_attestation_request(tessera_hash)
            .map_err(|e| CoreError::Network(e.to_string()))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse CLI args
    let cli = Cli::parse();

    // 2. Load config (precedence: --config > XDG_CONFIG_HOME > /etc > defaults)
    let mut config = if let Some(ref path) = cli.config {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        toml::from_str::<DaemonConfig>(&content)
            .with_context(|| format!("failed to parse config: {}", path.display()))?
    } else {
        let xdg_config = dirs::config_dir()
            .map(|d| d.join("tesseras").join("config.toml"))
            .filter(|p| p.exists());

        if let Some(ref path) = xdg_config {
            tracing::info!(path = %path.display(), "loading user config");
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read config: {}", path.display()))?;
            toml::from_str::<DaemonConfig>(&content)
                .with_context(|| format!("failed to parse config: {}", path.display()))?
        } else {
            // No --config, no XDG config: use defaults.
            // System service uses --config /etc/tesseras/config.toml explicitly.
            // User service / manual run gets data_dir = ~/.local/share/tesseras.
            tracing::info!("no config file found, using defaults");
            DaemonConfig::default()
        }
    };

    // Apply CLI overrides
    if !cli.listen.is_empty() {
        config.node.listen_addrs = cli.listen;
    }
    if let Some(ref data_dir) = cli.data_dir {
        config.node.data_dir = data_dir.clone();
    }

    // 3. Initialize logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    let effective_addrs = config.node.effective_addrs();
    tracing::info!(
        listen = ?effective_addrs,
        data_dir = %config.node.data_dir.display(),
        "starting tesd"
    );

    // 4. Ensure data directory exists
    std::fs::create_dir_all(&config.node.data_dir).with_context(|| {
        format!(
            "failed to create data dir: {}",
            config.node.data_dir.display()
        )
    })?;

    // 4b. Acquire exclusive process lock
    let _storage_lock = tesseras_storage::StorageLock::acquire(&config.node.data_dir)
        .context("cannot start daemon")?;

    // 5. Load or generate node identity
    let identity_path = config.node.data_dir.join("identity.key");
    let identity = if identity_path.exists() {
        let bytes = std::fs::read(&identity_path).context("failed to read identity file")?;
        let pubkey: [u8; 32] = bytes[..32].try_into().context("invalid identity file")?;
        let nonce = u64::from_le_bytes(bytes[32..40].try_into().context("invalid identity file")?);
        let node_id = pow::compute_node_id(&pubkey, nonce);
        tesseras_core::NodeIdentity {
            node_id,
            public_key: pubkey,
            nonce,
        }
    } else {
        // Generate a random keypair for now (Phase 1 simplification)
        let mut rng = rand::thread_rng();
        let mut pubkey = [0u8; 32];
        rng.fill(&mut pubkey);
        tracing::info!("generating new node identity (PoW)...");
        let identity = pow::generate_node_identity(&pubkey);
        // Save identity
        let mut bytes = Vec::with_capacity(40);
        bytes.extend_from_slice(&identity.public_key);
        bytes.extend_from_slice(&identity.nonce.to_le_bytes());
        std::fs::write(&identity_path, &bytes).context("failed to write identity file")?;
        identity
    };

    tracing::info!(node_id = %identity.node_id, "node identity loaded");

    // 5b. Verify institutional DNS identity (if configured)
    if let Some(ref inst_config) = config.institutional {
        match institutional::verify_dns(&inst_config.domain, &identity).await {
            Ok(()) => {
                tracing::info!(
                    domain = %inst_config.domain,
                    pledge_bytes = inst_config.pledge_bytes,
                    "institutional identity verified via DNS"
                );
            }
            Err(e) => {
                tracing::warn!(
                    domain = %inst_config.domain,
                    error = %e,
                    "institutional DNS verification failed, starting as normal full node"
                );
                // Downgrade to non-institutional config
                config.institutional = None;
            }
        }
    }

    // 6. Create QUIC transport (multi-endpoint for dual-stack)
    let pool_config = config.to_pool_config();
    let transport = QuinnTransport::bind_multiple_with_config(&effective_addrs, pool_config)
        .await
        .context("failed to bind QUIC transport")?;

    tracing::info!(addrs = ?transport.local_addrs(), "QUIC transport bound");

    // 7. Create DHT engine
    let dht_config = config.to_dht_config();
    let engine = DhtEngine::new(identity.clone(), Box::new(transport), dht_config);

    // 7b. Open SQLite database with WAL mode and pragmas
    let db_path = config.node.data_dir.join("db").join("tesseras.db");
    std::fs::create_dir_all(db_path.parent().unwrap())?;
    let storage_config = config.to_storage_config();
    let conn = tesseras_storage::open_database(&db_path, &storage_config)
        .with_context(|| format!("failed to open database: {}", db_path.display()))?;
    tracing::info!("database opened with WAL mode");
    let conn = Arc::new(Mutex::new(conn));

    // 7c. Create CAS store (shared by blob and fragment stores)
    let cas = Arc::new(tesseras_storage::CasStore::new(
        Arc::clone(&conn),
        config.node.data_dir.join("cas"),
    ));

    // 7c2. Run CAS dedup migration if needed (storage_version 1 -> 2)
    let migration_stats = tesseras_storage::migrate_to_cas(&config.node.data_dir, &cas, &conn)
        .with_context(|| "failed to run CAS dedup migration")?;
    if migration_stats.files_migrated > 0 {
        tracing::info!(
            files = migration_stats.files_migrated,
            duplicates = migration_stats.duplicates_found,
            bytes_saved = migration_stats.bytes_saved,
            failed = migration_stats.files_failed,
            "CAS dedup migration completed"
        );
    }

    // 7d. Create storage instances with LRU fragment cache
    let fs_fragments = FsFragmentStore::new(Arc::clone(&conn), Arc::clone(&cas));
    let fragment_store = tesseras_storage::CachedFragmentStore::new(
        Box::new(fs_fragments),
        (storage_config.fragment_cache_size_mb as usize) * 1024 * 1024,
    );
    let reciprocity_ledger = SqliteReciprocityLedger::new(Arc::clone(&conn));
    let blob_store = FsBlobStore::new(Arc::clone(&conn), Arc::clone(&cas));

    // 7d. Create replication service
    let dht_adapter = DhtPortAdapter::new(Arc::clone(&engine));
    let replication_config = config.to_replication_config();
    let replication = Arc::new(
        ReplicationService::new(
            identity,
            Box::new(dht_adapter),
            Box::new(fragment_store),
            Box::new(reciprocity_ledger),
            Box::new(blob_store),
            replication_config,
        )
        .with_cas(Arc::clone(&cas)),
    );

    // 7e. Wire replication handler into DHT engine
    let handler = ReplicationHandlerAdapter {
        service: Arc::clone(&replication),
    };
    engine.set_replication_handler(Arc::new(handler));

    // 7f. Setup RPC socket listener
    let socket_path = rpc::resolve_socket_path(&config.rpc.socket_path)?;
    let rpc_handler = Arc::new(rpc::handler::RpcHandler {
        tessera_repo: Arc::new(tesseras_storage::SqliteTesseraRepository::new(conn.clone())),
        memory_repo: Arc::new(tesseras_storage::SqliteMemoryRepository::new(conn.clone())),
        blob_store: Arc::new(FsBlobStore::new(conn.clone(), Arc::clone(&cas))),
        fragment_store: Arc::new(FsFragmentStore::new(conn.clone(), Arc::clone(&cas))),
        replication: Arc::clone(&replication),
        cas: Arc::clone(&cas),
        dht_engine: Arc::clone(&engine),
    });

    // 8. Setup shutdown signal
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // 9. Spawn engine run loop
    let engine_clone = Arc::clone(&engine);
    let engine_handle = tokio::spawn(async move {
        engine_clone.run(shutdown_rx).await;
    });

    // 9b. Spawn repair loop
    let repl_shutdown_rx = shutdown_tx.subscribe();
    let replication_clone = Arc::clone(&replication);
    let repl_handle = tokio::spawn(async move {
        replication_clone.run_repair_loop(repl_shutdown_rx).await;
    });

    // 9c. Spawn RPC listener
    let rpc_shutdown = shutdown_tx.subscribe();
    let rpc_handle = tokio::spawn(rpc::run_listener(socket_path, rpc_handler, rpc_shutdown));

    // 10. Bootstrap (SRV discovery with CLI override)
    let bootstrap_addrs: Vec<SocketAddr> = if let Some(ref addrs) = cli.bootstrap {
        // CLI override: resolve manually provided addresses
        let raw: Vec<String> = if addrs.is_empty() {
            vec![]
        } else {
            addrs.split(',').map(|s| s.trim().to_string()).collect()
        };
        let mut resolved = Vec::new();
        for addr in &raw {
            match tokio::net::lookup_host(addr).await {
                Ok(addrs) => resolved.extend(addrs),
                Err(e) => tracing::warn!(addr = %addr, error = %e, "failed to resolve CLI bootstrap address"),
            }
        }
        resolved
    } else {
        // Default: SRV discovery with hardcoded fallback
        bootstrap::resolve_bootstrap_peers(&config.bootstrap).await
    };

    // Store seeds in engine for automatic re-bootstrap
    engine.set_seeds(bootstrap_addrs.clone()).await;

    if !bootstrap_addrs.is_empty() {
        tracing::info!(seeds = ?bootstrap_addrs, "bootstrapping DHT");
        let mut bootstrap_ok = false;
        for attempt in 0..3u32 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt));
                tracing::info!(?delay, attempt, "retrying bootstrap");
                tokio::time::sleep(delay).await;
            }
            match engine.bootstrap(&bootstrap_addrs).await {
                Ok(()) => {
                    tracing::info!("bootstrap successful");
                    bootstrap_ok = true;
                    break;
                }
                Err(e) => tracing::warn!(attempt, "bootstrap failed: {e}"),
            }
        }
        if !bootstrap_ok {
            tracing::warn!("all bootstrap attempts failed, running with empty routing table");
        }
    } else {
        tracing::info!("no bootstrap nodes configured, running as seed");
    }

    tracing::info!(
        routing_table = engine.routing_table_size().await,
        "daemon ready"
    );

    // 11. Wait for shutdown signal (Ctrl+C)
    tokio::signal::ctrl_c()
        .await
        .context("failed to listen for ctrl-c")?;

    tracing::info!("shutting down...");
    shutdown_tx.send(true).ok();

    // Graceful shutdown with timeout
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), engine_handle).await;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), repl_handle).await;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), rpc_handle).await;

    tracing::info!("goodbye");
    Ok(())
}
