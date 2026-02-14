//! tesseras-daemon: full node binary for desktop/server/RPi.

#[allow(dead_code)]
mod config;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

use rand::Rng;
use tesseras_dht::engine::DhtEngine;
use tesseras_dht::pow;
use tesseras_net::{QuinnTransport, Transport};

use config::DaemonConfig;

#[derive(Parser, Debug)]
#[command(name = "tesseras-daemon", about = "Tesseras P2P daemon")]
struct Cli {
    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Listen address (overrides config)
    #[arg(short, long)]
    listen: Option<SocketAddr>,

    /// Bootstrap addresses (comma-separated, overrides config)
    #[arg(short, long)]
    bootstrap: Option<String>,

    /// Data directory (overrides config)
    #[arg(short, long)]
    data_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse CLI args
    let cli = Cli::parse();

    // 2. Load config
    let mut config = if let Some(ref path) = cli.config {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        toml::from_str::<DaemonConfig>(&content)
            .with_context(|| format!("failed to parse config: {}", path.display()))?
    } else {
        DaemonConfig::default()
    };

    // Apply CLI overrides
    if let Some(listen) = cli.listen {
        config.node.listen_addr = listen;
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

    tracing::info!(
        listen = %config.node.listen_addr,
        data_dir = %config.node.data_dir.display(),
        "starting tesseras-daemon"
    );

    // 4. Ensure data directory exists
    std::fs::create_dir_all(&config.node.data_dir)
        .with_context(|| format!("failed to create data dir: {}", config.node.data_dir.display()))?;

    // 5. Load or generate node identity
    let identity_path = config.node.data_dir.join("identity.key");
    let identity = if identity_path.exists() {
        let bytes = std::fs::read(&identity_path)
            .context("failed to read identity file")?;
        let pubkey: [u8; 32] = bytes[..32]
            .try_into()
            .context("invalid identity file")?;
        let nonce = u64::from_le_bytes(
            bytes[32..40]
                .try_into()
                .context("invalid identity file")?,
        );
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
        std::fs::write(&identity_path, &bytes)
            .context("failed to write identity file")?;
        identity
    };

    tracing::info!(node_id = %identity.node_id, "node identity loaded");

    // 6. Create QUIC transport
    let transport = QuinnTransport::bind(config.node.listen_addr)
        .await
        .context("failed to bind QUIC transport")?;

    tracing::info!(addr = %transport.local_addr(), "QUIC transport bound");

    // 7. Create DHT engine
    let dht_config = config.to_dht_config();
    let engine = DhtEngine::new(identity, Box::new(transport), dht_config);

    // 8. Setup shutdown signal
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // 9. Spawn engine run loop
    let engine_clone = Arc::clone(&engine);
    let engine_handle = tokio::spawn(async move {
        engine_clone.run(shutdown_rx).await;
    });

    // 10. Bootstrap
    let bootstrap_addrs: Vec<SocketAddr> = if let Some(ref addrs) = cli.bootstrap {
        if addrs.is_empty() {
            vec![]
        } else {
            addrs
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect()
        }
    } else {
        config
            .bootstrap
            .hardcoded
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    if !bootstrap_addrs.is_empty() {
        tracing::info!(seeds = ?bootstrap_addrs, "bootstrapping DHT");
        match engine.bootstrap(&bootstrap_addrs).await {
            Ok(()) => tracing::info!("bootstrap successful"),
            Err(e) => tracing::warn!("bootstrap failed: {e}"),
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

    tracing::info!("goodbye");
    Ok(())
}
