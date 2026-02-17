mod commands;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "tes", about = "Tesseras — P2P memory network", version)]
pub struct Cli {
    /// Path to identity/data directory (default: ~/.tesseras/)
    #[arg(long, global = true)]
    identity: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Add a memory to the network
    Add(commands::add::AddArgs),
    /// Get a memory from the network
    Get(commands::get::GetArgs),
    /// Remove a memory from the network
    Rm(commands::rm::RmArgs),
    /// List your tesseras (most recent first)
    Ls,
    /// Show tessera content and metadata
    Cat(commands::cat::CatArgs),
    /// Publish a tessera to the network (announce to DHT + distribute fragments)
    Publish(commands::publish::PublishArgs),
    /// Export tessera files to a local directory
    Export(commands::export::ExportArgs),
    /// Admin commands (bootstrap, daemon)
    Admin {
        #[command(subcommand)]
        command: commands::admin::AdminCommand,
    },
}

fn data_dir(cli: &Cli) -> Result<tesseras::config::DataDir> {
    let path = cli
        .identity
        .clone()
        .unwrap_or_else(tesseras::config::DataDir::default_path);
    Ok(tesseras::config::DataDir::open(path)?)
}

/// Ensure identity exists, create if not.
fn ensure_identity(data_dir: &tesseras::config::DataDir) -> Result<tesseras::crypto::Identity> {
    let key_path = data_dir.identity_key_path();
    if key_path.exists() {
        Ok(tesseras::crypto::Identity::load(&key_path)?)
    } else {
        let identity = tesseras::crypto::Identity::generate();
        identity.save(&key_path)?;
        eprintln!("Identity created: {}", key_path.display());
        Ok(identity)
    }
}

/// Create a Node from data directory and identity.
fn make_node(
    data_dir: &tesseras::config::DataDir,
    identity: tesseras::crypto::Identity,
) -> Result<tesseras::node::Node> {
    let config = data_dir.load_config().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(tesseras::node::Node::new(
        data_dir.clone(),
        identity,
        config,
    )?)
}

/// Check if the daemon is running (socket exists and connectable).
fn daemon_running(data_dir: &tesseras::config::DataDir) -> bool {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return false,
    };
    rt.block_on(tesseras::rpc::daemon_is_running(data_dir.root()))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let data_dir = data_dir(&cli)?;

    match cli.command {
        Commands::Add(args) => {
            let identity = ensure_identity(&data_dir)?;
            let node = make_node(&data_dir, identity)?;
            commands::add::run_with_node(&node, args)
        }
        Commands::Get(args) => {
            if daemon_running(&data_dir) {
                commands::get::run_via_rpc(data_dir.root(), args)
            } else {
                commands::get::run_standalone(&data_dir, args)
            }
        }
        Commands::Rm(args) => {
            if daemon_running(&data_dir) {
                commands::rm::run_via_rpc(data_dir.root(), args)
            } else {
                let identity = ensure_identity(&data_dir)?;
                let node = make_node(&data_dir, identity)?;
                commands::rm::run_with_node(&node, args)
            }
        }
        Commands::Ls => {
            if daemon_running(&data_dir) {
                commands::ls::run_via_rpc(data_dir.root())
            } else {
                let identity = ensure_identity(&data_dir)?;
                let node = make_node(&data_dir, identity)?;
                commands::ls::run_with_node(&node)
            }
        }
        Commands::Cat(args) => {
            if daemon_running(&data_dir) {
                commands::cat::run_via_rpc(data_dir.root(), args)
            } else {
                let _identity = ensure_identity(&data_dir)?;
                commands::cat::run(&data_dir, args)
            }
        }
        Commands::Publish(args) => {
            if !daemon_running(&data_dir) {
                anyhow::bail!(
                    "Daemon is not running. Start it with: tes admin daemon start\n\
                     Publish requires a running daemon to announce to the DHT network."
                );
            }
            commands::publish::run(data_dir.root(), args)
        }
        Commands::Export(args) => {
            let _identity = ensure_identity(&data_dir)?;
            commands::export::run(&data_dir, args)
        }
        Commands::Admin { command } => commands::admin::run(&data_dir, command),
    }
}
