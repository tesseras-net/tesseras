use std::collections::HashMap;
use std::io::{self, Write};

/// Simple representation of CLI commands.
#[derive(Debug)]
enum Command {
    Info,
    Stats,
    Put { key: String, value: String },
    Get { key: String },
    Ping,
    Quit,
    Empty,
    Unknown(String),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();

    let mut store: HashMap<String, String> = HashMap::new();
    let stdin = io::stdin();

    loop {
        print!("tesseras> ");
        io::stdout().flush()?;

        let mut line = String::new();
        let bytes = stdin.read_line(&mut line)?;

        if bytes == 0 {
            println!();
            break;
        }

        let cmd = parse_command(&line);

        match cmd {
            Command::Empty => {
                continue;
            }
            Command::Info => {
                handle_info();
            }
            Command::Stats => {
                handle_stats(&store);
            }
            Command::Put { key, value } => {
                handle_put(&mut store, key, value);
            }
            Command::Get { key } => {
                handle_get(&store, key);
            }
            Command::Ping => {
                handle_ping();
            }
            Command::Quit => {
                println!("Bye 👋");
                break;
            }
            Command::Unknown(raw) => {
                eprintln!("Unknown command: {raw}");
                println!("Type /help to see basic information.");
            }
        }
    }

    Ok(())
}

/// Print the Tesseras banner.
fn print_banner() {
    const BANNER: &str = r#"
     ████████╗███████╗███████╗███████╗███████╗██████╗  █████╗ ███████╗
     ╚══██╔══╝██╔════╝██╔════╝██╔════╝██╔════╝██╔══██╗██╔══██╗██╔════╝
        ██║   █████╗  ███████╗███████╗█████╗  ██████╔╝███████║███████╗
        ██║   ██╔══╝  ╚════██║╚════██║██╔══╝  ██╔══██╗██╔══██║╚════██║
        ██║   ███████╗███████║███████║███████╗██║  ██║██║  ██║███████║
        ╚═╝   ╚══════╝╚══════╝╚══════╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝

Tesseras Networking CLI
Type /help for information or /quit to exit.
"#;

    println!("{BANNER}");
}

/// Parse a raw input line into a Command.
///
/// Supported forms:
///   /put key value
///   put key value
///   > /put key value
fn parse_command(input: &str) -> Command {
    let mut line = input.trim().to_string();

    if let Some(stripped) = line.strip_prefix('>') {
        line = stripped.trim_start().to_string();
    }

    if let Some(stripped) = line.strip_prefix('/') {
        line = stripped.trim_start().to_string();
    }

    if line.is_empty() {
        return Command::Empty;
    }

    let mut parts = line.split_whitespace();
    let cmd = parts.next().unwrap().to_lowercase();

    match cmd.as_str() {
        "help" => Command::Info,
        "stats" => Command::Stats,
        "ping" => Command::Ping,
        "quit" | "bye" | "exit" => Command::Quit,
        "put" => {
            let key = match parts.next() {
                Some(k) => k.to_string(),
                None => {
                    return Command::Unknown("missing key for put".into());
                }
            };

            let value = parts.collect::<Vec<_>>().join(" ");
            if value.is_empty() {
                return Command::Unknown("missing value for put".into());
            }

            Command::Put { key, value }
        }
        "get" => {
            let key = match parts.next() {
                Some(k) => k.to_string(),
                None => {
                    return Command::Unknown("missing key for get".into());
                }
            };

            Command::Get { key }
        }
        _ => Command::Unknown(line),
    }
}

/// Handle `/help` command.
fn handle_info() {
    println!("Tesseras - Networking");
    println!("This CLI is currently running in local MOCK mode.");
    println!("Available commands:");
    println!("  /help              - Show information about this CLI");
    println!("  /stats             - Show mock stats");
    println!("  /put <key> <value> - Store a key/value pair (local mock)");
    println!("  /get <key>         - Retrieve a value by key (local mock)");
    println!("  /ping              - Ping the local node");
    println!("  /quit | /bye       - Exit the CLI");
}

/// Handle `/stats` command.
fn handle_stats(store: &HashMap<String, String>) {
    println!("--- Tesseras Stats (mock) ---");
    println!("Stored keys (local mock): {}", store.len());
    println!("Routing table nodes      : <not implemented yet>");
    println!("Network ID               : <not implemented yet>");
    println!("------------------------------");
}

/// Handle `/put` command.
fn handle_put(store: &mut HashMap<String, String>, key: String, value: String) {
    store.insert(key.clone(), value.clone());
    println!("Stored (mock): key='{key}', value='{value}'");
}

/// Handle `/get` command.
fn handle_get(store: &HashMap<String, String>, key: String) {
    match store.get(&key) {
        Some(value) => {
            println!("Found (mock): key='{key}', value='{value}'");
        }
        None => {
            println!("Key '{key}' not found (mock).");
        }
    }
}

/// Handle `/ping` command.
fn handle_ping() {
    println!("PONG (mock)");
}

