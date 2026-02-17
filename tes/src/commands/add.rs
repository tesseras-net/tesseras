use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use tesseras::config::DataDir;
use tesseras::crypto::Identity;
use tesseras::storage::Storage;
use tesseras::types::{MediaType, Memory, Tessera, Visibility};

#[derive(Args)]
pub struct AddArgs {
    /// Files to add as memories
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Name for this tessera
    #[arg(long)]
    name: Option<String>,

    /// Share with a specific circle
    #[arg(long)]
    circle: Option<String>,

    /// Mark as private (only you can access)
    #[arg(long)]
    private: bool,
}

pub fn run(data_dir: &DataDir, identity: &Identity, args: AddArgs) -> Result<()> {
    let storage = Storage::open(data_dir.clone())?;

    let visibility = if args.private {
        Visibility::Private
    } else if let Some(circle) = args.circle {
        Visibility::Circle { name: circle }
    } else {
        Visibility::Public
    };

    let mut memories = Vec::new();
    for file_path in &args.files {
        if !file_path.exists() {
            anyhow::bail!("file not found: {}", file_path.display());
        }

        let filename = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".into());

        let ext = file_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        let size = std::fs::metadata(file_path)?.len();

        eprint!("Adding {}... ", filename);
        let mut reader = std::fs::File::open(file_path)?;
        let blob_hash = storage.store_blob(&mut reader)?;
        eprintln!("done");

        memories.push(Memory {
            filename,
            media_type: MediaType::from_extension(&ext),
            size,
            blob_hash,
        });
    }

    // Build tessera content for hashing: serialize memories deterministically
    let content = rmp_serde::to_vec(&memories)?;
    let hash = tesseras::crypto::hash_bytes(&content);
    let signature = identity.sign(&content);

    let tessera = Tessera {
        hash,
        author: identity.public_key_bytes(),
        signature,
        created_at: chrono::Utc::now(),
        name: args.name,
        visibility,
        memories,
    };

    storage.store_tessera(&tessera)?;

    println!("{hash}");
    Ok(())
}
