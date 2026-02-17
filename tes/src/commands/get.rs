use anyhow::Result;
use clap::Args;

use tesseras::config::DataDir;

#[derive(Args)]
pub struct GetArgs {
    /// Tessera hash to fetch
    pub hash: String,
}

pub fn run(_data_dir: &DataDir, args: GetArgs) -> Result<()> {
    eprintln!(
        "Fetching {} from network... (not yet implemented)",
        args.hash
    );
    Ok(())
}
