use anyhow::{Context, Result};
use tesseras_core::HashPrefix;

use super::create::build_service;
use super::init::expand_tilde;

pub async fn run(hash: &str, data_dir: &str) -> Result<()> {
    let prefix = HashPrefix::parse(hash).context("invalid tessera hash or prefix")?;
    let base = expand_tilde(data_dir);
    let service = build_service(&base)?;
    let record = service.resolve_prefix(&prefix)?;
    let report = service.verify(&record.hash).await?;

    println!("Tessera: {}", report.tessera_hash.to_base32());
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
