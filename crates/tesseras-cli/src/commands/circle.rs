use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;
use tesseras_rpc::{DaemonClient, Request, Response};

#[derive(Subcommand)]
pub enum CircleCommands {
    /// Create a new circle
    Create {
        /// Circle name
        name: String,
    },
    /// Delete a circle
    Delete {
        /// Circle name
        name: String,
    },
    /// List all circles
    #[command(visible_alias = "ls")]
    List,
    /// Add a member to a circle
    Add {
        /// Circle name
        circle: String,
        /// Member alias (e.g. @alice)
        alias: String,
        /// Member's public key (hex)
        pubkey: String,
    },
    /// Remove a member from a circle
    Remove {
        /// Circle name
        circle: String,
        /// Member alias
        alias: String,
    },
}

pub async fn run(command: &CircleCommands, socket: &Option<PathBuf>) -> Result<()> {
    let socket_path = crate::commands::daemon::resolve_socket(socket)?;
    let mut client =
        DaemonClient::connect(&socket_path).context("failed to connect to daemon")?;

    match command {
        CircleCommands::Create { name } => {
            let resp = client
                .call(&Request::CircleCreate {
                    name: name.clone(),
                })
                .context("RPC call failed")?;

            match resp {
                Response::CircleCreated { name } => println!("Created circle '{name}'"),
                Response::Error { message, .. } => anyhow::bail!("{message}"),
                _ => anyhow::bail!("unexpected response"),
            }
        }
        CircleCommands::Delete { name } => {
            let resp = client
                .call(&Request::CircleDelete {
                    name: name.clone(),
                })
                .context("RPC call failed")?;

            match resp {
                Response::CircleDeleted { name } => println!("Deleted circle '{name}'"),
                Response::Error { message, .. } => anyhow::bail!("{message}"),
                _ => anyhow::bail!("unexpected response"),
            }
        }
        CircleCommands::List => {
            let resp = client
                .call(&Request::CircleList { name: None })
                .context("RPC call failed")?;

            match resp {
                Response::Circles { circles } => {
                    if circles.is_empty() {
                        println!("No circles");
                        return Ok(());
                    }
                    for c in &circles {
                        println!(
                            "  {} ({} members, created {})",
                            c.name, c.member_count, c.created_at
                        );
                    }
                }
                Response::Error { message, .. } => anyhow::bail!("{message}"),
                _ => anyhow::bail!("unexpected response"),
            }
        }
        CircleCommands::Add {
            circle,
            alias,
            pubkey,
        } => {
            let resp = client
                .call(&Request::CircleAddMember {
                    circle: circle.clone(),
                    alias: alias.clone(),
                    pubkey: pubkey.clone(),
                })
                .context("RPC call failed")?;

            match resp {
                Response::CircleMemberAdded => {
                    println!("Added {alias} to circle '{circle}'")
                }
                Response::Error { message, .. } => anyhow::bail!("{message}"),
                _ => anyhow::bail!("unexpected response"),
            }
        }
        CircleCommands::Remove { circle, alias } => {
            let resp = client
                .call(&Request::CircleRemoveMember {
                    circle: circle.clone(),
                    alias: alias.clone(),
                })
                .context("RPC call failed")?;

            match resp {
                Response::CircleMemberRemoved => {
                    println!("Removed {alias} from circle '{circle}'")
                }
                Response::Error { message, .. } => anyhow::bail!("{message}"),
                _ => anyhow::bail!("unexpected response"),
            }
        }
    }

    Ok(())
}
