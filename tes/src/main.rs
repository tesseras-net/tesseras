use clap::Parser;

#[derive(Parser)]
#[command(name = "tes", about = "Tesseras P2P memory network")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Add a memory to the network
    Add,
    /// Get a memory from the network
    Get,
}

fn main() {
    let _cli = Cli::parse();
    println!("tesseras v0.1.0");
}
