use anyhow::{Context, Result};
use std::path::PathBuf;
use tesseras_core::HashPrefix;
use tesseras_rpc::{DaemonClient, Request, Response};

use super::create::build_service;
use super::init::expand_tilde;

pub fn resolve_hash(
    input: &str,
    base: &std::path::Path,
) -> Result<tesseras_core::ContentHash> {
    let prefix = HashPrefix::parse(input).context("invalid tessera hash or prefix")?;
    let service = build_service(base)?;
    let record = service.resolve_prefix(&prefix)?;
    Ok(record.hash)
}

pub async fn run(hash: &str, data_dir: &str, socket: &Option<PathBuf>) -> Result<()> {
    let base = expand_tilde(data_dir);

    // Resolve hash (supports short prefix)
    let content_hash = resolve_hash(hash, &base)?;

    // Connect to daemon
    let socket_path = match socket {
        Some(p) => p.clone(),
        None => tesseras_rpc::default_socket_path()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
    };

    let mut client = DaemonClient::connect(&socket_path).with_context(|| {
        format!(
            "Cannot connect to daemon at {}\nIs tesseras-daemon running? Start it with: tesseras-daemon",
            socket_path.display()
        )
    })?;

    let response = client
        .call(&Request::Publish {
            hash: content_hash,
        })
        .context("publish request failed")?;

    match response {
        Response::Published {
            hash,
            fragments_created,
        } => {
            let short = hash.to_base32_short(8);
            println!("Published tessera {short} ({fragments_created} fragments created)");
            println!("Distribution in progress — use `tes status {short}` to track.");
        }
        other => anyhow::bail!("unexpected response: {other:?}"),
    }

    Ok(())
}
