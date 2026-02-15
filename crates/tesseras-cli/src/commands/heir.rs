use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use tesseras_core::ports::{IdentityStore, KeyAlgorithm};
use tesseras_crypto::secret_blob;
use tesseras_crypto::shamir::{
    ShamirConfig, ShamirSplitter, share_from_msgpack, share_from_text, share_to_msgpack,
    share_to_text,
};
use tesseras_storage::FsIdentityStore;

use super::init::expand_tilde;

#[derive(Subcommand)]
pub enum HeirCommands {
    /// Create heir shares from your identity keys
    Create {
        /// Minimum shares needed to reconstruct (default: 2)
        #[arg(long, default_value_t = 2)]
        threshold: u8,

        /// Total number of shares to create (default: 3)
        #[arg(long, default_value_t = 3)]
        shares: u8,

        /// Output directory for share files
        #[arg(long, default_value = "./heir-shares")]
        output_dir: String,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Reconstruct identity from heir shares
    Reconstruct {
        /// Share files (at least threshold number)
        #[arg(required = true)]
        share_files: Vec<String>,

        /// Output directory for reconstructed keys
        #[arg(long)]
        output_dir: String,

        /// Install reconstructed keys to ~/.tesseras/identity/
        #[arg(long)]
        install: bool,

        /// Verify reconstructed key matches this fingerprint (hex)
        #[arg(long)]
        verify_identity: Option<String>,
    },

    /// Display information about a share file
    Info {
        /// Share file path
        share_file: String,
    },
}

fn load_share(path: &str) -> Result<tesseras_crypto::shamir::HeirShare> {
    let data = std::fs::read(path).with_context(|| format!("failed to read {path}"))?;

    // Auto-detect format
    if path.ends_with(".txt") {
        let text = String::from_utf8(data).context("share text file is not valid UTF-8")?;
        share_from_text(&text).map_err(Into::into)
    } else {
        // Try msgpack first, then text
        match share_from_msgpack(&data) {
            Ok(share) => Ok(share),
            Err(_) => {
                let text = String::from_utf8(data)
                    .context("share file is neither valid msgpack nor UTF-8 text")?;
                share_from_text(&text).map_err(Into::into)
            }
        }
    }
}

pub async fn run_create(
    threshold: u8,
    shares: u8,
    output_dir: &str,
    yes: bool,
    data_dir: &str,
) -> Result<()> {
    let base = expand_tilde(data_dir);
    let identity_store = FsIdentityStore::new(base.clone());

    // Load Ed25519 key material (required)
    let ed_material = identity_store
        .load_keypair(KeyAlgorithm::Ed25519)
        .context("no Ed25519 identity found — run `tes init` first")?;

    // Load encryption keys (optional — old installs may not have them)
    let x25519_material = identity_store.load_keypair(KeyAlgorithm::X25519).ok();
    let mlkem_material = identity_store.load_keypair(KeyAlgorithm::MlKem768).ok();

    // Both encryption keys must be present or both absent
    let (x25519_secret, mlkem_secret_ref) = match (&x25519_material, &mlkem_material) {
        (Some(x), Some(m)) => {
            let x_arr: [u8; 32] = x
                .secret
                .as_slice()
                .try_into()
                .context("X25519 secret must be 32 bytes")?;
            (Some(x_arr), Some(m.secret.as_slice()))
        }
        (None, None) => (None, None),
        _ => bail!("inconsistent encryption keys: run `tes init --upgrade`"),
    };

    let ed_secret: [u8; 32] = ed_material
        .secret
        .as_slice()
        .try_into()
        .context("Ed25519 secret must be 32 bytes")?;
    let secret_blob_data =
        secret_blob::assemble(&ed_secret, x25519_secret.as_ref(), mlkem_secret_ref);

    let key_desc = if x25519_secret.is_some() {
        format!(
            "Ed25519 + X25519 + ML-KEM-768 ({} bytes)",
            secret_blob_data.len()
        )
    } else {
        format!("Ed25519 ({} bytes)", secret_blob_data.len())
    };

    // Confirmation prompt
    if !yes {
        println!("About to create heir shares:");
        println!("  Threshold: {} of {}", threshold, shares);
        println!("  Key material: {key_desc}");
        println!("  Output: {output_dir}/");
        println!();
        println!("WARNING: These shares can reconstruct your full identity.");
        println!(
            "         Anyone with {} shares gains complete access.",
            threshold
        );
        println!();

        let confirm = dialoguer::Confirm::new()
            .with_prompt("Proceed?")
            .default(false)
            .interact()?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let config = ShamirConfig {
        threshold,
        total_shares: shares,
    };
    let heir_shares = ShamirSplitter::split(&secret_blob_data, &config, &ed_material.public)
        .context("failed to split key material")?;

    // Create output directory
    let out_path = PathBuf::from(output_dir);
    std::fs::create_dir_all(&out_path).context("failed to create output directory")?;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    for share in &heir_shares {
        let idx = share.share_index;

        // Binary (MessagePack)
        let bin_path = out_path.join(format!("heir_share_{idx}.bin"));
        let msgpack = share_to_msgpack(share)?;
        std::fs::write(&bin_path, &msgpack)?;

        // Base64 text
        let txt_path = out_path.join(format!("heir_share_{idx}.txt"));
        let text = share_to_text(share, &today)?;
        std::fs::write(&txt_path, text)?;
    }

    // Write heir_meta.json
    let meta = tesseras_core::HeirShareMeta {
        format_version: 1,
        session_id: heir_shares[0].session_id,
        threshold,
        total_shares: shares,
        created_at: chrono::Utc::now(),
    };
    let meta_path = base.join("identity/heir_meta.json");
    let meta_json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(&meta_path, meta_json)?;

    // Summary
    let fingerprint: String = heir_shares[0]
        .owner_fingerprint
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let session: String = heir_shares[0]
        .session_id
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    println!();
    println!(
        "Created {} heir shares (threshold: {} of {})",
        shares, threshold, shares
    );
    println!("Owner fingerprint: {fingerprint}");
    println!("Session: {session}");
    println!();
    for share in &heir_shares {
        let idx = share.share_index;
        println!("  Share {idx}: {}/heir_share_{idx}.{{bin,txt}}", output_dir);
    }
    println!();
    println!("IMPORTANT: Distribute shares to different people/locations.");
    println!(
        "           Any {} shares can reconstruct your full identity.",
        threshold
    );
    println!("           Store this printout separately from the shares.");

    Ok(())
}

pub async fn run_reconstruct(
    share_files: &[String],
    output_dir: &str,
    install: bool,
    verify_identity: Option<&str>,
    data_dir: &str,
) -> Result<()> {
    // Load shares
    let mut shares = Vec::new();
    for path in share_files {
        let share = load_share(path).with_context(|| format!("failed to load share: {path}"))?;
        shares.push(share);
    }

    if shares.is_empty() {
        bail!("no share files provided");
    }

    // Reconstruct
    let expected_public: Option<Vec<u8>> = if let Some(fp) = verify_identity {
        // If user provided a fingerprint, we pass their public key for verification later
        // For now, we don't have the public key from fingerprint alone, so just pass None
        // and verify the fingerprint manually after reconstruction
        let _ = fp;
        None
    } else {
        None
    };

    let recovered_blob = ShamirSplitter::reconstruct(&shares, expected_public.as_deref())
        .context("reconstruction failed")?;

    // Parse the blob using shared secret_blob module
    let parsed = secret_blob::parse(&recovered_blob)
        .context("failed to parse reconstructed secret blob")?;

    // Derive Ed25519 public key to verify
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&parsed.ed25519_secret);
    let public_key = signing_key.verifying_key();

    let fingerprint: String = {
        let hash = blake3::hash(public_key.as_bytes());
        hash.as_bytes()[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    };

    // Verify fingerprint if requested
    if let Some(expected_fp) = verify_identity {
        if fingerprint != expected_fp {
            bail!("fingerprint mismatch: reconstructed={fingerprint}, expected={expected_fp}");
        }
        println!("Fingerprint verified: {fingerprint}");
    }

    println!("Reconstruction successful.");
    println!("Owner fingerprint: {fingerprint}");

    // Write to output directory
    let out_path = PathBuf::from(output_dir);
    std::fs::create_dir_all(&out_path).context("failed to create output directory")?;

    // Write Ed25519 key pair
    std::fs::write(out_path.join("node.ed25519.key"), &parsed.ed25519_secret)?;
    std::fs::write(out_path.join("node.ed25519.pub"), public_key.as_bytes())?;

    // Write X25519 if present
    if let Some(x_secret) = &parsed.x25519_secret {
        let x_static = x25519_dalek::StaticSecret::from(*x_secret);
        let x_public = x25519_dalek::PublicKey::from(&x_static);
        std::fs::write(out_path.join("node.x25519.key"), x_secret)?;
        std::fs::write(out_path.join("node.x25519.pub"), x_public.as_bytes())?;
        println!("  X25519 keys recovered");
    }

    // Write ML-KEM-768 if present
    if let Some(mlkem_secret) = &parsed.mlkem768_secret {
        std::fs::write(out_path.join("node.mlkem768.key"), mlkem_secret)?;
        println!("  ML-KEM-768 secret recovered (public key regenerated on next init)");
    }

    println!("Keys written to {output_dir}/");

    // Install if requested
    if install {
        let base = expand_tilde(data_dir);
        let identity_dir = base.join("identity");

        println!();
        println!(
            "This will replace the current identity at {}",
            identity_dir.display()
        );
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let backup_dir = base.join(format!("identity.bak.{timestamp}"));
        println!(
            "The current identity will be backed up to {}",
            backup_dir.display()
        );

        let confirm = dialoguer::Confirm::new()
            .with_prompt("Proceed?")
            .default(false)
            .interact()?;
        if !confirm {
            println!("Aborted. Keys remain in {output_dir}/");
            return Ok(());
        }

        // Backup
        if identity_dir.exists() {
            std::fs::rename(&identity_dir, &backup_dir)?;
        }
        std::fs::create_dir_all(&identity_dir)?;

        // Copy all reconstructed keys
        for entry in std::fs::read_dir(&out_path)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("node.") && (name_str.ends_with(".key") || name_str.ends_with(".pub")) {
                std::fs::copy(entry.path(), identity_dir.join(&name))?;
            }
        }

        println!("Identity installed at {}", identity_dir.display());
    }

    Ok(())
}

pub async fn run_info(share_file: &str) -> Result<()> {
    let share = load_share(share_file)?;

    let fingerprint: String = share
        .owner_fingerprint
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let session: String = share
        .session_id
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let checksum_ok = share.verify_checksum();

    println!("Heir Share Information:");
    println!("  Format version: {}", share.format_version);
    println!(
        "  Share: {} of {} (threshold: {})",
        share.share_index, share.total_shares, share.threshold
    );
    println!("  Session: {session}");
    println!("  Owner fingerprint: {fingerprint}");
    println!("  Share data size: {} bytes", share.share_data.len());
    println!(
        "  Checksum: {}",
        if checksum_ok { "valid" } else { "INVALID" }
    );

    Ok(())
}
