use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use tesseras::node::Node;
use tesseras::rpc::{self, RpcRequest, RpcResponse};
use tesseras::types::Visibility;

#[derive(Args)]
pub struct AddArgs {
    /// Files to add as memories
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Name for this tessera
    #[arg(long)]
    pub name: Option<String>,

    /// Share with a specific circle
    #[arg(long)]
    pub circle: Option<String>,

    /// Mark as private (only you can access)
    #[arg(long)]
    pub private: bool,
}

pub fn run_with_node(node: &Node, args: AddArgs) -> Result<tesseras::types::ContentHash> {
    let visibility = if args.private {
        Visibility::Private
    } else if let Some(circle) = args.circle {
        Visibility::Circle { name: circle }
    } else {
        Visibility::Public
    };

    for file_path in &args.files {
        if !file_path.exists() {
            anyhow::bail!("file not found: {}", file_path.display());
        }
    }

    let tessera = node.add_tessera(&args.files, args.name, visibility)?;
    println!("{}", tessera.hash);
    Ok(tessera.hash)
}

/// After adding locally, announce to DHT via the running daemon's RPC.
pub fn announce_via_rpc(data_dir: &std::path::Path, hash: &tesseras::types::ContentHash) {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return,
    };

    // Announce tessera pointer to DHT
    match rt.block_on(rpc::send_request(
        data_dir,
        &RpcRequest::AnnounceTessera { hash: *hash },
    )) {
        Ok(RpcResponse::Pong { peer_count, .. }) => {
            eprintln!("Published to {peer_count} peers");
        }
        Ok(RpcResponse::Error(e)) => {
            eprintln!("Warning: announce failed: {e}");
        }
        _ => {}
    }

    // Distribute fragments
    match rt.block_on(rpc::send_request(
        data_dir,
        &RpcRequest::DistributeFragments { hash: *hash },
    )) {
        Ok(RpcResponse::Pong { peer_count, .. }) => {
            eprintln!("Distributed fragments to {peer_count} peers");
        }
        Ok(RpcResponse::Error(e)) => {
            eprintln!("Warning: fragment distribution failed: {e}");
        }
        _ => {}
    }
}
