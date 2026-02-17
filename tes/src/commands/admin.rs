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
    /// Start the daemon in the background
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
                let mut config = data_dir
                    .load_config()
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
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
                let mut config = data_dir
                    .load_config()
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                config.bootstrap.retain(|a| a != &addr);
                data_dir
                    .save_config(&config)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                eprintln!("Removed bootstrap: {addr}");
                Ok(())
            }
            BootstrapCommand::Ls => {
                let config = data_dir
                    .load_config()
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
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
                eprintln!("Daemon start not yet implemented.");
                Ok(())
            }
            DaemonCommand::Stop => {
                eprintln!("Daemon stop not yet implemented.");
                Ok(())
            }
            DaemonCommand::Status => {
                eprintln!("Daemon status not yet implemented.");
                Ok(())
            }
        },
    }
}
