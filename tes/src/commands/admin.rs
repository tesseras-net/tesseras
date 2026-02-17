use anyhow::Result;

use tesseras::config::DataDir;

#[derive(clap::Subcommand)]
pub enum AdminCommand {
    /// Manage bootstrap nodes
    Bootstrap {
        #[command(subcommand)]
        command: BootstrapCommand,
    },
    /// Manage the daemon
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    /// Show node identity info
    Id,
}

#[derive(clap::Subcommand)]
pub enum BootstrapCommand {
    /// Add a bootstrap node
    Add { addr: String },
    /// Remove a bootstrap node
    Rm { addr: String },
    /// List bootstrap nodes
    Ls,
}

#[derive(clap::Subcommand)]
pub enum DaemonCommand {
    /// Start the daemon in the foreground
    Start,
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status,
}

pub fn run(data_dir: &DataDir, command: AdminCommand) -> Result<()> {
    match command {
        AdminCommand::Bootstrap { command } => match command {
            BootstrapCommand::Add { addr } => {
                let mut config = data_dir.load_config().map_err(|e| anyhow::anyhow!("{e}"))?;
                if !config.bootstrap.contains(&addr) {
                    config.bootstrap.push(addr.clone());
                    data_dir
                        .save_config(&config)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                }
                eprintln!("Added bootstrap: {addr}");
                Ok(())
            }
            BootstrapCommand::Rm { addr } => {
                let mut config = data_dir.load_config().map_err(|e| anyhow::anyhow!("{e}"))?;
                config.bootstrap.retain(|a| a != &addr);
                data_dir
                    .save_config(&config)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                eprintln!("Removed bootstrap: {addr}");
                Ok(())
            }
            BootstrapCommand::Ls => {
                let config = data_dir.load_config().map_err(|e| anyhow::anyhow!("{e}"))?;
                if config.bootstrap.is_empty() {
                    eprintln!("No bootstrap nodes configured.");
                } else {
                    for addr in &config.bootstrap {
                        println!("{addr}");
                    }
                }
                Ok(())
            }
        },
        AdminCommand::Daemon { command } => match command {
            DaemonCommand::Start => {
                eprintln!("Daemon start not yet implemented (requires Task 13 E2E setup).");
                eprintln!("Use the node library directly for now.");
                Ok(())
            }
            DaemonCommand::Stop => {
                let pid_path = data_dir.root().join("daemon.pid");
                if pid_path.exists() {
                    let pid_str = std::fs::read_to_string(&pid_path)?;
                    eprintln!("Stopping daemon (PID: {})...", pid_str.trim());
                    #[cfg(unix)]
                    {
                        use std::process::Command;
                        let _ = Command::new("kill").arg(pid_str.trim()).status();
                    }
                    let _ = std::fs::remove_file(&pid_path);
                    eprintln!("Daemon stopped.");
                } else {
                    eprintln!("No daemon PID file found.");
                }
                Ok(())
            }
            DaemonCommand::Status => {
                let pid_path = data_dir.root().join("daemon.pid");
                if pid_path.exists() {
                    let pid_str = std::fs::read_to_string(&pid_path)?;
                    eprintln!("Daemon PID: {}", pid_str.trim());
                    // Check if process is alive
                    #[cfg(unix)]
                    {
                        let status = std::process::Command::new("kill")
                            .args(["-0", pid_str.trim()])
                            .status();
                        if status.is_ok() && status.unwrap().success() {
                            eprintln!("Status: running");
                        } else {
                            eprintln!("Status: not running (stale PID file)");
                        }
                    }
                } else {
                    eprintln!("Daemon is not running.");
                }
                Ok(())
            }
        },
        AdminCommand::Id => {
            let key_path = data_dir.identity_key_path();
            if key_path.exists() {
                let identity = tesseras::crypto::Identity::load(&key_path)?;
                println!("Node ID:     {}", identity.node_id());
                println!("Public Key:  {}", hex::encode(identity.public_key_bytes()));
                println!("Key File:    {}", key_path.display());
            } else {
                eprintln!("No identity found. Run any command to auto-create one.");
            }
            Ok(())
        }
    }
}
