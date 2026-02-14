mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tesseras", about = "Create and preserve human memories")]
struct Cli {
    /// Base directory for data storage
    #[arg(long, default_value = "~/.tesseras", global = true)]
    data_dir: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize identity and local database
    Init,
    /// Create a tessera from a directory of files
    Create(commands::create::CreateArgs),
    /// Verify integrity of a stored tessera
    Verify {
        /// Tessera content hash (64 hex chars)
        hash: String,
    },
    /// Export tessera to a self-contained directory
    Export {
        /// Tessera content hash (64 hex chars)
        hash: String,
        /// Destination directory
        dest: String,
    },
    /// List local tesseras
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => commands::init::run(&cli.data_dir).await,
        Commands::Create(ref args) => commands::create::run(args, &cli.data_dir).await,
        Commands::Verify { ref hash } => commands::verify::run(hash, &cli.data_dir).await,
        Commands::Export { ref hash, ref dest } => {
            commands::export::run(hash, dest, &cli.data_dir).await
        }
        Commands::List => commands::list::run(&cli.data_dir).await,
    }
}
