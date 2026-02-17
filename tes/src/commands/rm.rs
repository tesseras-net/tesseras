use anyhow::Result;
use clap::Args;

use tesseras::node::Node;
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
