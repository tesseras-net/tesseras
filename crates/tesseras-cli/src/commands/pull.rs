use std::path::PathBuf;

use anyhow::{Context, Result};
use tesseras_core::{ContentHash, HashPrefix};

use super::create::build_service;
use super::init::expand_tilde;

#[derive(clap::Args)]
pub struct PullArgs {
    /// Tessera hash or @alias
    pub target: String,

    /// Output directory (default: current directory)
    #[arg(default_value = ".")]
    pub dest: String,

    /// Pull only the N most recent tesseras from @alias
    #[arg(long)]
    pub latest: Option<usize>,

    /// Show detailed progress
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Debug, PartialEq)]
enum PullTarget {
    Hash(String),
    Alias(String),
}

fn parse_target(input: &str) -> Result<PullTarget> {
    if let Some(alias) = input.strip_prefix('@') {
        if alias.is_empty() {
            anyhow::bail!("empty alias: use @name");
        }
        Ok(PullTarget::Alias(alias.to_string()))
    } else {
        let cleaned = input.trim();
        if cleaned.is_empty() {
            anyhow::bail!("empty target");
        }
        Ok(PullTarget::Hash(cleaned.to_string()))
    }
}

pub async fn run(args: &PullArgs, data_dir: &str, socket: &Option<PathBuf>) -> Result<()> {
    let base = expand_tilde(data_dir);
    let target = parse_target(&args.target)?;

    match target {
        PullTarget::Hash(ref hash_str) => pull_by_hash(hash_str, &args.dest, &base, socket).await,
        PullTarget::Alias(ref alias) => pull_by_alias(alias, &base).await,
    }
}

async fn pull_by_hash(
    hash_str: &str,
    dest: &str,
    base: &std::path::Path,
    socket: &Option<PathBuf>,
) -> Result<()> {
    // 1. Resolve hash (could be prefix or full)
    let content_hash: ContentHash = match HashPrefix::parse(hash_str)
        .context("invalid tessera hash")?
    {
        HashPrefix::Exact(h) => h,
        prefix => {
            if let Ok(service) = build_service(base) {
                if let Ok(record) = service.resolve_prefix(&prefix) {
                    record.hash
                } else {
                    anyhow::bail!(
                        "'{}' is not a valid tessera hash. Use full 64-char hex hash for network fetch.",
                        hash_str
                    );
                }
            } else {
                anyhow::bail!(
                    "'{}' is not a full tessera hash and no local database is available.",
                    hash_str
                );
            }
        }
    };

    let short = content_hash.to_base32_short(8);

    // 2. Check if we already have it locally
    if let Ok(service) = build_service(base) {
        if service
            .resolve_prefix(&HashPrefix::Exact(content_hash))
            .is_ok()
        {
            let dest_path = PathBuf::from(dest);
            service.export(&content_hash, &dest_path).await?;
            println!(
                "  Saved to {}",
                dest_path.join(format!("tessera-{short}")).display()
            );
            return Ok(());
        }
    }

    // 3. Ensure daemon for network fetch
    if !super::daemon::is_daemon_running(base) {
        eprint!("  Starting daemon...");
        super::daemon::start_daemon(base)?;
        eprintln!(" done");
    }

    let socket_path = match socket {
        Some(p) => p.clone(),
        None => tesseras_rpc::default_socket_path().map_err(|e| anyhow::anyhow!("{e}"))?,
    };

    eprintln!("  Downloading tessera {short}...");

    let mut client = tesseras_rpc::DaemonClient::connect(&socket_path)
        .with_context(|| "Cannot connect to daemon. Is tesd running?")?;

    let response = client
        .call(&tesseras_rpc::Request::Fetch { hash: content_hash })
        .context("fetch failed")?;

    match response {
        tesseras_rpc::Response::Fetched {
            hash,
            memories,
            bytes,
        } => {
            let short = hash.to_base32_short(8);
            let service = build_service(base)?;
            let dest_path = PathBuf::from(dest);
            service.export(&hash, &dest_path).await?;
            let size = super::list::format_size(bytes);
            println!(
                "  Saved to {}",
                dest_path.join(format!("tessera-{short}")).display()
            );
            println!("    {memories} memories, {size}");
        }
        other => anyhow::bail!("unexpected response: {other:?}"),
    }

    Ok(())
}

async fn pull_by_alias(alias: &str, base: &std::path::Path) -> Result<()> {
    let contacts_path = base.join("contacts.toml");
    if !contacts_path.exists() {
        anyhow::bail!(
            "unknown contact '{alias}'. Run 'tes contact add {alias} <public-key>' first."
        );
    }
    anyhow::bail!("pull by @alias is not yet implemented — use a tessera hash instead")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_target_hex_hash() {
        let result =
            parse_target("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();
        assert!(matches!(result, PullTarget::Hash(_)));
    }

    #[test]
    fn parse_target_short_prefix() {
        let result = parse_target("abc123").unwrap();
        assert!(matches!(result, PullTarget::Hash(h) if h == "abc123"));
    }

    #[test]
    fn parse_target_alias() {
        let result = parse_target("@wife").unwrap();
        assert_eq!(result, PullTarget::Alias("wife".to_string()));
    }

    #[test]
    fn parse_target_empty_alias_fails() {
        assert!(parse_target("@").is_err());
    }

    #[test]
    fn parse_target_empty_fails() {
        assert!(parse_target("").is_err());
    }
}
