use anyhow::{Context, Result};
use std::path::PathBuf;
use tesseras_core::ContentHash;
use tesseras_rpc::{DaemonClient, Request, Response};

pub async fn run(hash: &str, _data_dir: &str, socket: &Option<PathBuf>) -> Result<()> {
    let content_hash: ContentHash = hash
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid hash (expected 64 hex chars): {hash}"))?;

    let socket_path = match socket {
        Some(p) => p.clone(),
        None => tesseras_rpc::default_socket_path()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
    };

    eprintln!(
        "Fetching tessera {} from network...",
        &hash[..8.min(hash.len())]
    );

    let mut client = DaemonClient::connect(&socket_path).with_context(|| {
        format!(
            "Cannot connect to daemon at {}\nIs tesseras-daemon running? Start it with: tesseras-daemon",
            socket_path.display()
        )
    })?;

    let response = client
        .call(&Request::Fetch {
            hash: content_hash,
        })
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
