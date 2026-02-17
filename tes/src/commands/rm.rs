use anyhow::Result;
use clap::Args;

use tesseras::config::DataDir;
use tesseras::storage::Storage;
use tesseras::types::ContentHash;

#[derive(Args)]
pub struct RmArgs {
    /// Tessera hash to remove
    pub hash: String,
}

pub fn run(data_dir: &DataDir, args: RmArgs) -> Result<()> {
    let storage = Storage::open(data_dir.clone())?;
    let hash: ContentHash = args.hash.parse().map_err(|e| anyhow::anyhow!("{e}"))?;

    let tessera = storage
        .find_tessera(&hash)?
        .ok_or_else(|| anyhow::anyhow!("tessera not found: {hash}"))?;

    for memory in &tessera.memories {
        storage.delete_blob(&memory.blob_hash)?;
    }
    storage.delete_tessera(&hash)?;

    eprintln!("Removed {hash}");
    Ok(())
}
