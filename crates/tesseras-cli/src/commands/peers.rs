use anyhow::{Context, Result};
use comfy_table::{Cell, Table};
use std::path::PathBuf;
use tesseras_core::Capabilities;
use tesseras_rpc::{DaemonClient, Request, Response};

pub async fn run(socket: &Option<PathBuf>) -> Result<()> {
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
        .call(&Request::Peers)
        .context("peers request failed")?;

    match response {
        Response::Peers { peers } => {
            if peers.is_empty() {
                println!("No peers in routing table.");
                return Ok(());
            }

            let mut table = Table::new();
            table.set_header(vec!["Node ID", "Address", "Capabilities"]);

            for peer in &peers {
                let node_id = peer.identity.node_id.to_string();
                let short_id = &node_id[..16];

                let addr = peer.addr.to_string();
                let caps = format_capabilities(peer.capabilities);

                table.add_row(vec![
                    Cell::new(short_id),
                    Cell::new(addr),
                    Cell::new(caps),
                ]);
            }

            println!("{table}");
            println!("{} peer(s)", peers.len());
        }
        other => anyhow::bail!("unexpected response: {other:?}"),
    }

    Ok(())
}

fn format_capabilities(caps: Capabilities) -> String {
    let flags = [
        (Capabilities::PING, "ping"),
        (Capabilities::FIND_NODE, "find_node"),
        (Capabilities::FIND_VALUE, "find_value"),
        (Capabilities::STORE, "store"),
        (Capabilities::REPLICATE, "replicate"),
        (Capabilities::ATTEST, "attest"),
        (Capabilities::RELAY, "relay"),
        (Capabilities::INSTITUTIONAL, "institutional"),
        (Capabilities::SEARCH_INDEX, "search"),
    ];

    let active: Vec<&str> = flags
        .iter()
        .filter(|(flag, _)| caps.has(*flag))
        .map(|(_, name)| *name)
        .collect();

    if active.is_empty() {
        "none".to_string()
    } else {
        active.join(", ")
    }
}
