use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use tesseras::config::DataDir;
use tesseras::storage::Storage;
use tesseras::types::ContentHash;

#[derive(Args)]
pub struct ExportArgs {
    /// Tessera hash to export
    pub hash: String,

    /// Destination directory
    pub dest: PathBuf,
}

pub fn run(data_dir: &DataDir, args: ExportArgs) -> Result<()> {
    let storage = Storage::open(data_dir.clone())?;
    let hash: ContentHash = args.hash.parse().map_err(|e| anyhow::anyhow!("{e}"))?;

    let tessera = storage
        .find_tessera(&hash)?
        .ok_or_else(|| anyhow::anyhow!("tessera not found: {hash}"))?;

    let export_dir = if let Some(name) = &tessera.name {
        args.dest.join(name.replace(' ', "-").to_lowercase())
    } else {
        args.dest.join(&tessera.hash.to_string()[..12])
    };

    std::fs::create_dir_all(&export_dir)?;

    for memory in &tessera.memories {
        let dest_path = export_dir.join(&memory.filename);
        let mut file = std::fs::File::create(&dest_path)?;
        storage.read_blob(&memory.blob_hash, &mut file)?;
        eprintln!("  {} ({} bytes)", memory.filename, memory.size);
    }

    eprintln!("Exported to {}", export_dir.display());
    Ok(())
}
