use anyhow::{Context, Result};
use std::path::PathBuf;
use tesseras_core::ContentHash;
use tesseras_core::HashPrefix;
use tesseras_rpc::{DaemonClient, Request, Response};

use super::init::expand_tilde;

pub async fn run(hash: &str, data_dir: &str, socket: &Option<PathBuf>) -> Result<()> {
    let content_hash: ContentHash =
        match HashPrefix::parse(hash).context("invalid tessera hash or prefix")? {
            HashPrefix::Exact(h) => h,
            _ => {
                // Short prefix: try resolving against local DB
                let base = expand_tilde(data_dir);
                let prefix = HashPrefix::parse(hash)?;
                let service = super::create::build_service(&base)?;
                service.resolve_prefix(&prefix)?.hash
            }
        };

    let socket_path = match socket {
        Some(p) => p.clone(),
        None => tesseras_rpc::default_socket_path().map_err(|e| anyhow::anyhow!("{e}"))?,
    };

    eprintln!(
        "Fetching tessera {} from network...",
        content_hash.to_base32_short(8)
    );

    let mut client = DaemonClient::connect(&socket_path).with_context(|| {
        format!(
            "Cannot connect to daemon at {}\nIs tesd running? Start it with: tesd",
            socket_path.display()
        )
    })?;

    let response = client
        .call(&Request::Fetch { hash: content_hash })
        .context("fetch request failed")?;

    match response {
        Response::Fetched {
            hash,
            memories,
            bytes,
        } => {
            let short = hash.to_base32_short(8);
            let size = format_bytes(bytes);
            println!("Fetched tessera {short} ({memories} memories, {size})");
        }
        other => anyhow::bail!("unexpected response: {other:?}"),
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
