use anyhow::{Context, Result};
use std::str::FromStr;
use tesseras_core::ContentHash;

use super::create::build_service;
use super::init::expand_tilde;

pub async fn run(hash: &str, data_dir: &str) -> Result<()> {
    let content_hash =
        ContentHash::from_str(hash).context("invalid tessera hash (expected 64 hex chars)")?;
    let base = expand_tilde(data_dir);
    let service = build_service(&base)?;
    let report = service.verify(&content_hash).await?;

    println!("Tessera: {}", report.tessera_hash);
    println!(
        "Signature: {}",
        if report.signature_valid {
            "VALID"
        } else {
            "INVALID"
        }
    );
    for file in &report.files {
        let status = if file.valid { "OK" } else { "FAILED" };
        println!("  [{status}] {}", file.path);
    }
    let all_valid = report.signature_valid && report.files.iter().all(|f| f.valid);
    if all_valid {
        println!("Verification: PASSED");
    } else {
        println!("Verification: FAILED");
        std::process::exit(1);
    }
    Ok(())
}
