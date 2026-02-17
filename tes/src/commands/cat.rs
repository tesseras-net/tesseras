use anyhow::Result;
use clap::Args;

use tesseras::config::DataDir;
use tesseras::storage::Storage;
use tesseras::types::ContentHash;

#[derive(Args)]
pub struct CatArgs {
    /// Tessera hash to display
    pub hash: String,
}

pub fn run(data_dir: &DataDir, args: CatArgs) -> Result<()> {
    let storage = Storage::open(data_dir.clone())?;
    let hash: ContentHash = args.hash.parse().map_err(|e| anyhow::anyhow!("{e}"))?;

    let tessera = storage
        .find_tessera(&hash)?
        .ok_or_else(|| anyhow::anyhow!("tessera not found: {hash}"))?;

    println!("Hash:       {}", tessera.hash);
    println!(
        "Name:       {}",
        tessera.name.as_deref().unwrap_or("(unnamed)")
    );
    println!("Visibility: {}", tessera.visibility);
    println!("Created:    {}", tessera.created_at.format("%Y-%m-%d %H:%M"));
    println!("Author:     {}", hex::encode(&tessera.author[..8]));
    println!("Files:");
    for m in &tessera.memories {
        println!("  {} ({:?}, {} bytes)", m.filename, m.media_type, m.size);
    }

    Ok(())
}
