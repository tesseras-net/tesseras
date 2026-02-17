use std::path::PathBuf;

use anyhow::Result;

#[derive(clap::Args)]
pub struct PushArgs {
    /// Files or directories to include in the tessera
    #[arg(required = true)]
    pub paths: Vec<String>,

    /// Human-readable name for this memory
    #[arg(long)]
    pub name: Option<String>,

    /// Comma-separated tags
    #[arg(long)]
    pub tags: Option<String>,

    /// Visibility: public (default), private, circle
    #[arg(long, default_value = "public")]
    pub visibility: String,

    /// Show what would be created without writing
    #[arg(long)]
    pub dry_run: bool,

    /// Show detailed progress
    #[arg(long)]
    pub verbose: bool,
}

pub async fn run(args: &PushArgs, data_dir: &str, socket: &Option<PathBuf>) -> Result<()> {
    todo!("push implementation")
}
