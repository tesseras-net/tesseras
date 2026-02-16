use anyhow::{Context, Result};
use std::path::PathBuf;
use tesseras_rpc::{DaemonClient, PublishState, Request, Response};

use super::init::expand_tilde;

pub async fn run(hash: &str, data_dir: &str, socket: &Option<PathBuf>) -> Result<()> {
    let base = expand_tilde(data_dir);
    let content_hash = super::publish::resolve_hash(hash, &base)?;

    let socket_path = match socket {
        Some(p) => p.clone(),
        None => tesseras_rpc::default_socket_path()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
    };

    let mut client = DaemonClient::connect(&socket_path).with_context(|| {
        format!(
            "Cannot connect to daemon at {}\nIs tesd running? Start it with: tesd",
            socket_path.display()
        )
    })?;

    let response = client
        .call(&Request::Status {
            hash: content_hash,
        })
        .context("status request failed")?;

    match response {
        Response::Status {
            hash,
            state,
            fragments_total,
            fragments_placed,
            peers_holding,
        } => {
            let hash_hex = hash.to_string();
            let state_str = match state {
                PublishState::Local => "Local (not published)",
                PublishState::Publishing => "Publishing...",
                PublishState::Replicated => "Replicated",
                PublishState::Healthy => "Healthy",
            };
            println!("Tessera:     {hash_hex}");
            println!("State:       {state_str}");
            println!("Fragments:   {fragments_placed}/{fragments_total} placed");
            println!("Peers:       {peers_holding} holding copies");
        }
        other => anyhow::bail!("unexpected response: {other:?}"),
    }

    Ok(())
}
