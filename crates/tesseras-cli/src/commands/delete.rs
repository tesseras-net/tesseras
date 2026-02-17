use std::path::PathBuf;

use anyhow::{Context, Result};
use tesseras_rpc::{DaemonClient, Request, Response};

/// Delete a tessera and propagate retraction to the network.
pub async fn run(hash: &str, socket: &Option<PathBuf>) -> Result<()> {
    let socket_path = crate::commands::daemon::resolve_socket(socket)?;
    let mut client =
        DaemonClient::connect(&socket_path).context("failed to connect to daemon")?;

    let resp = client
        .call(&Request::Delete {
            hash: hash.to_string(),
        })
        .context("RPC call failed")?;

    match resp {
        Response::Deleted {
            hash,
            tombstone_published,
        } => {
            println!("Deleted {}", &hash[..16.min(hash.len())]);
            if tombstone_published {
                println!("  Retraction propagated to network");
            } else {
                println!("  Retraction queued (will propagate when online)");
            }
        }
        Response::Error { message, .. } => {
            anyhow::bail!("{message}");
        }
        _ => anyhow::bail!("unexpected response from daemon"),
    }

    Ok(())
}
