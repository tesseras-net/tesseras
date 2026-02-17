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

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let data_dir = data_dir(&cli)?;

    match cli.command {
        Commands::Add(args) => {
            let identity = ensure_identity(&data_dir)?;
            commands::add::run(&data_dir, &identity, args)
        }
        Commands::Get(args) => {
            let _identity = ensure_identity(&data_dir)?;
            commands::get::run(&data_dir, args)
        }
        Commands::Rm(args) => {
            let _identity = ensure_identity(&data_dir)?;
            commands::rm::run(&data_dir, args)
        }
        Commands::Ls => {
            let _identity = ensure_identity(&data_dir)?;
            commands::ls::run(&data_dir)
        }
        Commands::Cat(args) => {
            let _identity = ensure_identity(&data_dir)?;
            commands::cat::run(&data_dir, args)
        }
        Commands::Export(args) => {
            let _identity = ensure_identity(&data_dir)?;
            commands::export::run(&data_dir, args)
        }
        Commands::Admin { command } => commands::admin::run(&data_dir, command),
    }
}
