use anyhow::Result;
use clap::Args;

use tesseras::config::DataDir;
use tesseras::rpc::{self, RpcRequest, RpcResponse};
use tesseras::types::ContentHash;

#[derive(Args)]
pub struct GetArgs {
    /// Tessera hash to fetch
    pub hash: String,
}

/// Fetch via the running daemon's RPC (daemon does DHT lookup + fetch).
pub fn run_via_rpc(data_dir: &std::path::Path, args: GetArgs) -> Result<()> {
    let hash: ContentHash = args.hash.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    let rt = tokio::runtime::Runtime::new()?;
    let response = rt.block_on(rpc::send_request(
        data_dir,
        &RpcRequest::FetchTesseraFromNetwork { hash },
    ))?;

    match response {
        RpcResponse::Tessera(t) => {
            eprintln!("Fetched tessera {}", t.hash);
            eprintln!("Name:       {}", t.name.as_deref().unwrap_or("(unnamed)"));
            eprintln!("Files:      {}", t.memories.len());
            let total_size: u64 = t.memories.iter().map(|m| m.size).sum();
            eprintln!("Total size: {} bytes", total_size);
            Ok(())
        }
        RpcResponse::Error(e) => anyhow::bail!("{e}"),
        other => anyhow::bail!("unexpected response: {other:?}"),
    }
}

/// Fetch without a daemon: start a temporary node, bootstrap, fetch, shutdown.
pub fn run_standalone(data_dir: &DataDir, args: GetArgs) -> Result<()> {
    let hash: ContentHash = args.hash.parse().map_err(|e| anyhow::anyhow!("{e}"))?;

    // Check local storage first
    {
        let storage = tesseras::storage::Storage::open(data_dir.clone())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        if let Some(t) = storage
            .find_tessera(&hash)
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            eprintln!("Found tessera {} (local)", t.hash);
            eprintln!("Name:       {}", t.name.as_deref().unwrap_or("(unnamed)"));
            eprintln!("Files:      {}", t.memories.len());
            return Ok(());
        }
    }

    // Not local — start a temporary node and fetch from network
    let key_path = data_dir.identity_key_path();
    let identity = if key_path.exists() {
        tesseras::crypto::Identity::load(&key_path)?
    } else {
        let id = tesseras::crypto::Identity::generate();
        id.save(&key_path)?;
        id
    };

    let config = data_dir.load_config().map_err(|e| anyhow::anyhow!("{e}"))?;

    if config.bootstrap.is_empty() {
        anyhow::bail!(
            "no bootstrap nodes configured and daemon is not running.\n\
             Add bootstrap nodes with: tes admin bootstrap add <addr>"
        );
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let mut node = tesseras::node::Node::new(data_dir.clone(), identity, config)?;
        let _addr = node.start().await?;

        let discovered = node.bootstrap().await?;
        eprintln!("Connected to {discovered} peers");

        match node.fetch_tessera_from_network(&hash).await? {
            Some(t) => {
                eprintln!("Fetched tessera {}", t.hash);
                eprintln!("Name:       {}", t.name.as_deref().unwrap_or("(unnamed)"));
                eprintln!("Files:      {}", t.memories.len());
                let total_size: u64 = t.memories.iter().map(|m| m.size).sum();
                eprintln!("Total size: {} bytes", total_size);
            }
            None => {
                anyhow::bail!("tessera not found on network: {hash}");
            }
        }

        node.shutdown();
        Ok(())
    })
}
