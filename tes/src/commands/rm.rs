use anyhow::Result;
use clap::Args;

use tesseras::node::Node;
use tesseras::rpc::{self, RpcRequest, RpcResponse};
use tesseras::types::ContentHash;

#[derive(Args)]
pub struct RmArgs {
    /// Tessera hash to remove
    pub hash: String,
}

pub fn run_with_node(node: &Node, args: RmArgs) -> Result<()> {
    let hash: ContentHash = args.hash.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    node.remove_tessera(&hash)?;
    eprintln!("Removed {hash}");
    Ok(())
}

pub fn run_via_rpc(data_dir: &std::path::Path, args: RmArgs) -> Result<()> {
    let hash: ContentHash = args.hash.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    let rt = tokio::runtime::Runtime::new()?;
    let response = rt.block_on(rpc::send_request(
        data_dir,
        &RpcRequest::RemoveTessera { hash },
    ))?;

    match response {
        RpcResponse::Ok => {
            eprintln!("Removed {hash}");
            Ok(())
        }
        RpcResponse::Error(e) => anyhow::bail!("{e}"),
        other => anyhow::bail!("unexpected response: {other:?}"),
    }
}
