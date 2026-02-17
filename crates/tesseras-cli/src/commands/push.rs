use std::path::{Path, PathBuf};

use anyhow::Result;
use tesseras_core::{CreateInput, FileInput};

use super::create::{build_service, infer_memory_type, parse_visibility, scan_input};
use super::init::expand_tilde;

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

/// Scan multiple paths (files or directories), collecting all supported files.
fn scan_paths(paths: &[String]) -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();
    for path in paths {
        all_files.extend(scan_input(path)?);
    }
    if all_files.is_empty() {
        anyhow::bail!("no supported files found. Supported: jpg, jpeg, png, wav, webm, txt");
    }
    all_files.sort();
    all_files.dedup();
    Ok(all_files)
}

#[derive(Default)]
struct FileTypeCounts {
    photos: usize,
    audio: usize,
    video: usize,
    text: usize,
}

impl FileTypeCounts {
    fn add(&mut self, path: &Path) {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
        {
            Some(ref ext) if matches!(ext.as_str(), "jpg" | "jpeg" | "png") => self.photos += 1,
            Some(ref ext) if ext == "wav" => self.audio += 1,
            Some(ref ext) if ext == "webm" => self.video += 1,
            Some(ref ext) if ext == "txt" => self.text += 1,
            _ => {}
        }
    }

    fn total(&self) -> usize {
        self.photos + self.audio + self.video + self.text
    }

    fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.photos > 0 {
            parts.push(format!(
                "{} photo{}",
                self.photos,
                if self.photos > 1 { "s" } else { "" }
            ));
        }
        if self.audio > 0 {
            parts.push(format!("{} audio", self.audio));
        }
        if self.video > 0 {
            parts.push(format!("{} video", self.video));
        }
        if self.text > 0 {
            parts.push(format!("{} text", self.text));
        }
        if parts.is_empty() {
            "0 files".to_string()
        } else {
            parts.join(", ")
        }
    }

    fn total_size(files: &[PathBuf]) -> u64 {
        files
            .iter()
            .filter_map(|f| f.metadata().ok())
            .map(|m| m.len())
            .sum()
    }
}

pub async fn run(args: &PushArgs, data_dir: &str, socket: &Option<PathBuf>) -> Result<()> {
    let base = expand_tilde(data_dir);

    // 1. Scan input files
    let files = scan_paths(&args.paths)?;

    // 2. Dry run: show what would happen
    if args.dry_run {
        let counts = {
            let mut c = FileTypeCounts::default();
            for f in &files {
                c.add(f);
            }
            c
        };
        let total_size = FileTypeCounts::total_size(&files);
        let tier = if total_size < 4 * 1024 * 1024 {
            "small (whole-file replication)"
        } else if total_size < 1024 * 1024 * 1024 {
            "medium (Reed-Solomon 16+8)"
        } else {
            "large (Reed-Solomon 32+16)"
        };
        println!("Would create tessera with {} files:", counts.total());
        println!("  {}", counts.summary());
        println!(
            "  Estimated size: {}",
            super::list::format_size(total_size)
        );
        println!("  Tier: {tier}");
        return Ok(());
    }

    // 3. Auto-init
    if super::init::ensure_initialized(&base).await? {
        eprintln!("Initialized new identity at {}", base.display());
    }

    // 4. Build file inputs
    let file_inputs: Vec<FileInput> = files
        .iter()
        .map(|f| FileInput {
            path: f.clone(),
            context: args.name.clone(),
            memory_type: infer_memory_type(f),
        })
        .collect();

    let visibility = parse_visibility(&args.visibility)?;
    let tags = args
        .tags
        .as_deref()
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let input = CreateInput {
        files: file_inputs,
        visibility,
        language: "en".to_string(),
        tags,
        location: None,
        encryption_public: None,
    };

    // 5. Create tessera
    let mut counts = FileTypeCounts::default();
    for f in &files {
        counts.add(f);
    }
    let service = build_service(&base)?;
    let content_hash = service.create(input).await?;
    let short = content_hash.to_base32_short(8);
    println!("  Created tessera {short} ({})", counts.summary());

    // 6. Ensure daemon + publish
    let socket_path = match socket {
        Some(p) => p.clone(),
        None => tesseras_rpc::default_socket_path().map_err(|e| anyhow::anyhow!("{e}"))?,
    };

    if !super::daemon::is_daemon_running(&base) {
        eprint!("  Starting daemon...");
        super::daemon::start_daemon(&base)?;
        eprintln!(" done");
    }

    match tesseras_rpc::DaemonClient::connect(&socket_path) {
        Ok(mut client) => {
            match client.call(&tesseras_rpc::Request::Publish {
                hash: content_hash,
            }) {
                Ok(tesseras_rpc::Response::Published {
                    fragments_created, ..
                }) => {
                    println!("  Replicating... {fragments_created} fragments distributed");
                }
                Ok(_) => {
                    eprintln!("  Warning: unexpected response from daemon");
                }
                Err(e) => {
                    eprintln!("  Warning: publish failed: {e}");
                }
            }
        }
        Err(_) => {
            eprintln!("  Warning: daemon not available, tessera saved locally only.");
            eprintln!("  Run 'tes push' again later to replicate, or start the daemon.");
        }
    }

    println!("  Done. Hash: {content_hash}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_multiple_paths_collects_all_files() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.jpg"), b"fake jpg").unwrap();
        std::fs::write(dir.path().join("b.txt"), b"hello").unwrap();
        std::fs::write(dir.path().join("c.mp3"), b"unsupported").unwrap();

        let paths = vec![
            dir.path().join("a.jpg").to_string_lossy().into_owned(),
            dir.path().join("b.txt").to_string_lossy().into_owned(),
        ];
        let files = scan_paths(&paths).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn scan_paths_rejects_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        // Create a dir with no supported files
        std::fs::write(dir.path().join("c.mp3"), b"unsupported").unwrap();
        let paths = vec![dir.path().to_string_lossy().into_owned()];
        let result = scan_paths(&paths);
        assert!(result.is_err());
    }

    #[test]
    fn dry_run_summary_counts_file_types() {
        let mut counts = FileTypeCounts::default();
        counts.add(Path::new("a.jpg"));
        counts.add(Path::new("b.jpeg"));
        counts.add(Path::new("c.png"));
        counts.add(Path::new("d.txt"));
        counts.add(Path::new("e.wav"));
        counts.add(Path::new("f.webm"));
        assert_eq!(counts.photos, 3);
        assert_eq!(counts.text, 1);
        assert_eq!(counts.audio, 1);
        assert_eq!(counts.video, 1);
    }
}
