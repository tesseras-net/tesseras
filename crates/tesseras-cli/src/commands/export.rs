use anyhow::{Context, Result};
use std::path::PathBuf;
use tesseras_core::HashPrefix;

use super::create::build_service;
use super::init::expand_tilde;

pub async fn run(hash: &str, dest: &str, data_dir: &str) -> Result<()> {
    let prefix = HashPrefix::parse(hash).context("invalid tessera hash or prefix")?;
    let base = expand_tilde(data_dir);
    let service = build_service(&base)?;
    let record = service.resolve_prefix(&prefix)?;
    let dest = PathBuf::from(dest);
    service.export(&record.hash, &dest).await?;
    println!(
        "Exported to {}",
        dest.join(format!("tessera-{}", record.hash)).display()
    );
    Ok(())
}
