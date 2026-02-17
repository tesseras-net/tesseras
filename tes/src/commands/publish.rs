use anyhow::Result;
use clap::Args;

use tesseras::rpc::{self, RpcRequest, RpcResponse};
use tesseras::types::ContentHash;

#[derive(Args)]
pub struct PublishArgs {
    /// Tessera hash to publish to the network
    pub hash: String,
}

/// Announce a tessera to the DHT via the running daemon.
pub fn run(data_dir: &std::path::Path, args: PublishArgs) -> Result<()> {
    let hash: ContentHash = args.hash.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    let rt = tokio::runtime::Runtime::new()?;

    // Announce the tessera pointer to DHT
    let response = rt.block_on(rpc::send_request(
        data_dir,
        &RpcRequest::AnnounceTessera { hash },
    ))?;

    match response {
        RpcResponse::Pong { peer_count, .. } => {
            eprintln!("Announced tessera {hash} to {peer_count} peers");
        }
        RpcResponse::Error(e) => anyhow::bail!("{e}"),
        other => anyhow::bail!("unexpected response: {other:?}"),
    }

    // Distribute fragments to peers
    let response = rt.block_on(rpc::send_request(
        data_dir,
        &RpcRequest::DistributeFragments { hash },
    ))?;

    match response {
        RpcResponse::Pong { peer_count, .. } => {
            eprintln!("Distributed fragments to {peer_count} peers");
        }
        RpcResponse::Error(e) => {
            eprintln!("Warning: fragment distribution failed: {e}");
        }
        _ => {}
    }

    Ok(())
}
