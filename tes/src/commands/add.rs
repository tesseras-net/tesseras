use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use tesseras::node::Node;
use tesseras::types::Visibility;

#[derive(Args)]
pub struct AddArgs {
    /// Files to add as memories
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Name for this tessera
    #[arg(long)]
    pub name: Option<String>,

    /// Share with a specific circle
    #[arg(long)]
    pub circle: Option<String>,

    /// Mark as private (only you can access)
    #[arg(long)]
    pub private: bool,
}

pub fn run_with_node(node: &Node, args: AddArgs) -> Result<()> {
    let visibility = if args.private {
        Visibility::Private
    } else if let Some(circle) = args.circle {
        Visibility::Circle { name: circle }
    } else {
        Visibility::Public
    };

    for file_path in &args.files {
        if !file_path.exists() {
            anyhow::bail!("file not found: {}", file_path.display());
        }
    }

    let tessera = node.add_tessera(&args.files, args.name, visibility)?;
    println!("{}", tessera.hash);
    Ok(())
}
