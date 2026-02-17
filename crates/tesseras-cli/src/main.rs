mod commands;

use std::io;
use std::path::PathBuf;

use anyhow::Result;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default())
    .valid(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .invalid(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .error(AnsiColor::Red.on_default().effects(Effects::BOLD));

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_HASH"),
    " ",
    env!("GIT_DATE"),
    ")"
);

const AFTER_HELP: &str = "\
For more information, try `tes <command> --help`
Network commands: `tes net --help`  Identity commands: `tes identity --help`
Source code: https://github.com/tesseras-net/tesseras";

/// Create and preserve human memories
#[derive(Parser)]
#[command(
    name = "tes",
    version = VERSION,
    override_usage = "tes [COMMAND]",
    after_help = AFTER_HELP,
    subcommand_required = true,
    arg_required_else_help = true,
    styles = STYLES,
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
    #[arg(long, default_value_t = default_data_dir(), global = true)]
    data_dir: String,

    /// Path to daemon Unix socket
    #[arg(long, global = true)]
    socket: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy)]
pub(crate) struct OutputConfig {
    pub json: bool,
    pub color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a tessera from a directory of files
    #[command(visible_alias = "c")]
    Create(commands::create::CreateArgs),

    /// Show detailed information about a tessera
    Show {
        /// Tessera hash or prefix (base32 or hex)
        hash: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List local tesseras
    #[command(visible_alias = "ls")]
    List,

    /// Verify integrity of a stored tessera
    #[command(visible_alias = "v")]
    Verify {
        /// Tessera hash or prefix (base32 or hex)
        hash: String,
    },

    /// Export tessera to a self-contained directory
    #[command(visible_alias = "e")]
    Export {
        /// Tessera hash or prefix (base32 or hex)
        hash: String,
        /// Destination directory
        dest: String,
    },

    /// Daemon management (start, stop, status)
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },

    /// Network operations (publish, fetch, status)
    Net {
        #[command(subcommand)]
        command: NetCommands,
    },

    /// Identity management (init, heir, institutional)
    Identity {
        #[command(subcommand)]
        command: IdentityCommands,
    },

    /// Generate shell completions for your shell to stdout
    #[command(hide = true)]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum DaemonCommands {
    /// Start the daemon in the background
    Start,
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
}

#[derive(Subcommand)]
enum NetCommands {
    /// Publish a tessera to the network
    #[command(visible_alias = "pub")]
    Publish {
        /// Tessera hash or prefix
        hash: String,
    },
    /// Fetch a tessera from the network
    Fetch {
        /// Full tessera hash (64 hex chars)
        hash: String,
    },
    /// Show replication status of a tessera
    Status {
        /// Tessera hash or prefix
        hash: String,
    },
    /// List connected peers in the routing table
    Peers,
}

#[derive(Subcommand)]
enum IdentityCommands {
    /// Initialize identity and local database
    Init {
        /// Add missing encryption keys to existing identity
        #[arg(long)]
        upgrade: bool,
    },
    /// Manage heir key recovery shares
    Heir {
        #[command(subcommand)]
        command: commands::heir::HeirCommands,
    },
    /// Manage institutional node setup
    Institutional {
        #[command(subcommand)]
        command: InstitutionalCommands,
    },
}

#[derive(Subcommand)]
enum InstitutionalCommands {
    /// Set up institutional node identity and print DNS TXT record
    Setup {
        /// Domain to verify (e.g., archive.org)
        #[arg(long)]
        domain: String,
        /// Check if DNS record is already propagated
        #[arg(long)]
        check: bool,
    },
}

fn default_data_dir() -> String {
    dirs::data_dir()
        .map(|p| p.join("tesseras"))
        .unwrap_or_else(|| std::path::PathBuf::from("~/.tesseras"))
        .to_string_lossy()
        .into_owned()
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
        Commands::Create(ref args) => {
            commands::create::run(args, &cli.data_dir, &cli.socket).await
        }
        Commands::Daemon { command } => match command {
            DaemonCommands::Start => commands::daemon::run_start(&cli.data_dir).await,
            DaemonCommands::Stop => commands::daemon::run_stop(&cli.data_dir).await,
            DaemonCommands::Status => commands::daemon::run_status(&cli.data_dir).await,
        },
        Commands::Show { ref hash, json } => {
            use std::io::IsTerminal;
            let color = match cli.color {
                ColorWhen::Always => true,
                ColorWhen::Never => false,
                ColorWhen::Auto => std::io::stdout().is_terminal(),
            };
            commands::show::run(hash, &cli.data_dir, OutputConfig { json, color }).await
        }
        Commands::List => commands::list::run(&cli.data_dir).await,
        Commands::Verify { ref hash } => commands::verify::run(hash, &cli.data_dir).await,
        Commands::Export { ref hash, ref dest } => {
            commands::export::run(hash, dest, &cli.data_dir).await
        }
        Commands::Net { command } => match command {
            NetCommands::Publish { ref hash } => {
                commands::publish::run(hash, &cli.data_dir, &cli.socket).await
            }
            NetCommands::Fetch { ref hash } => {
                commands::fetch::run(hash, &cli.data_dir, &cli.socket).await
            }
            NetCommands::Status { ref hash } => {
                commands::status::run(hash, &cli.data_dir, &cli.socket).await
            }
            NetCommands::Peers => commands::peers::run(&cli.socket).await,
        },
        Commands::Identity { command } => match command {
            IdentityCommands::Init { upgrade } => {
                commands::init::run(&cli.data_dir, upgrade).await
            }
            IdentityCommands::Heir { command } => match command {
                commands::heir::HeirCommands::Create {
                    threshold,
                    shares,
                    output_dir,
                    yes,
                } => {
                    commands::heir::run_create(threshold, shares, &output_dir, yes, &cli.data_dir)
                        .await
                }
                commands::heir::HeirCommands::Reconstruct {
                    share_files,
                    output_dir,
                    install,
                    verify_identity,
                } => {
                    commands::heir::run_reconstruct(
                        &share_files,
                        &output_dir,
                        install,
                        verify_identity.as_deref(),
                        &cli.data_dir,
                    )
                    .await
                }
                commands::heir::HeirCommands::Info { share_file } => {
                    commands::heir::run_info(&share_file).await
                }
            },
            IdentityCommands::Institutional { command } => match command {
                InstitutionalCommands::Setup { ref domain, check } => {
                    commands::institutional::run_setup(domain, check, &cli.data_dir).await
                }
            },
        },
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "tes", &mut io::stdout());
            Ok(())
        }
    }
}
