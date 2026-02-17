use anyhow::Result;
use clap::Args;

use tesseras::config::DataDir;
use tesseras::rpc::{self, RpcRequest, RpcResponse};
use tesseras::storage::Storage;
use tesseras::types::{ContentHash, Tessera};

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

    display_tessera(&tessera);
    Ok(())
}

pub fn run_via_rpc(data_dir: &std::path::Path, args: CatArgs) -> Result<()> {
    let hash: ContentHash = args.hash.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    let rt = tokio::runtime::Runtime::new()?;
    let response = rt.block_on(rpc::send_request(
        data_dir,
        &RpcRequest::GetTessera { hash },
    ))?;

    match response {
        RpcResponse::Tessera(t) => {
            display_tessera(&t);
            Ok(())
        }
        RpcResponse::Error(e) => anyhow::bail!("{e}"),
        other => anyhow::bail!("unexpected response: {other:?}"),
    }
}

fn display_tessera(tessera: &Tessera) {
    println!("Hash:       {}", tessera.hash);
    println!(
        "Name:       {}",
        tessera.name.as_deref().unwrap_or("(unnamed)")
    );
    println!("Visibility: {}", tessera.visibility);
    println!(
        "Created:    {}",
        tessera.created_at.format("%Y-%m-%d %H:%M")
    );
    println!("Author:     {}", hex::encode(&tessera.author[..8]));
    println!("Files:");
    for m in &tessera.memories {
        println!("  {} ({:?}, {} bytes)", m.filename, m.media_type, m.size);
    }
}
