use anyhow::{Context, Result};
use std::path::PathBuf;
use std::str::FromStr;
use tesseras_core::ContentHash;

use super::create::build_service;
use super::init::expand_tilde;

pub async fn run(hash: &str, dest: &str, data_dir: &str) -> Result<()> {
    let content_hash =
        ContentHash::from_str(hash).context("invalid tessera hash (expected 64 hex chars)")?;
    let base = expand_tilde(data_dir);
    let dest = PathBuf::from(dest);
    let service = build_service(&base)?;
    service.export(&content_hash, &dest).await?;
    println!(
        "Exported to {}",
        dest.join(format!("tessera-{content_hash}")).display()
    );
    Ok(())
}
