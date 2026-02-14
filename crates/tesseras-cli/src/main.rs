mod commands;

use std::io;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_HASH"),
    " ",
    env!("GIT_DATE"),
    ")"
);

const AFTER_HELP: &str = "\
For more information about a specific command, try `tes <command> --help`
The source code for tesseras is available at: https://github.com/tesseras-net/tesseras";

/// Create and preserve human memories
#[derive(Parser)]
#[command(
    name = "tes",
    version = VERSION,
    override_usage = "tes [COMMAND]",
    after_help = AFTER_HELP,
    subcommand_required = true,
    arg_required_else_help = true,
)]
struct Cli {
    /// Use verbose output (-vv very verbose)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Do not print log messages
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,

    /// Coloring
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto, global = true)]
    color: ColorWhen,

    /// Base directory for data storage
    #[arg(long, default_value = "~/.tesseras", global = true)]
    data_dir: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize identity and local database
    Init,

    /// Create a tessera from a directory of files
    #[command(visible_alias = "c")]
    Create(commands::create::CreateArgs),

    /// Verify integrity of a stored tessera
    #[command(visible_alias = "v")]
    Verify {
        /// Tessera content hash (64 hex chars)
        hash: String,
    },

    /// Export tessera to a self-contained directory
    #[command(visible_alias = "e")]
    Export {
        /// Tessera content hash (64 hex chars)
        hash: String,
        /// Destination directory
        dest: String,
    },

    /// List local tesseras
    #[command(visible_alias = "ls")]
    List,

    /// Generate shell completions for your shell to stdout
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn setup_logging(verbose: u8, quiet: bool) {
    use tracing_subscriber::EnvFilter;

    if quiet {
        return;
    }

    let filter = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(cli.verbose, cli.quiet);

    match cli.command {
        Commands::Init => commands::init::run(&cli.data_dir).await,
        Commands::Create(ref args) => commands::create::run(args, &cli.data_dir).await,
        Commands::Verify { ref hash } => commands::verify::run(hash, &cli.data_dir).await,
        Commands::Export { ref hash, ref dest } => {
            commands::export::run(hash, dest, &cli.data_dir).await
        }
        Commands::List => commands::list::run(&cli.data_dir).await,
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "tes", &mut io::stdout());
            Ok(())
        }
    }
}
