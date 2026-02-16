use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use tesseras_core::{HashPrefix, Manifest, MemoryMetadata};

use super::create::build_service;
use super::init::expand_tilde;
use super::list::format_size;
use crate::OutputConfig;

pub async fn run(hash: &str, data_dir: &str, out: OutputConfig) -> Result<()> {
    let prefix = HashPrefix::parse(hash).context("invalid tessera hash or prefix")?;
    let base = expand_tilde(data_dir);
    let service = build_service(&base)?;
    let record = service.resolve_prefix(&prefix)?;

    // Load manifest to get file details
    let report = service.verify(&record.hash).await?;

    // Load memory records for metadata (tags, language, location)
    let memories = {
        use std::sync::{Arc, Mutex};
        let db_path = base.join("db/tesseras.db");
        let conn =
            tesseras_storage::open_database(&db_path, &tesseras_storage::StorageConfig::default())
                .context("failed to open database")?;
        let conn = Arc::new(Mutex::new(conn));
        let memory_repo = tesseras_storage::SqliteMemoryRepository::new(conn);
        use tesseras_core::MemoryRepository;
        memory_repo.list_by_tessera(&record.hash)?
    };

    // Load manifest text to parse full manifest
    let manifest = {
        use std::sync::{Arc, Mutex};
        let db_path = base.join("db/tesseras.db");
        let conn =
            tesseras_storage::open_database(&db_path, &tesseras_storage::StorageConfig::default())
                .context("failed to open database")?;
        let conn = Arc::new(Mutex::new(conn));
        let cas = Arc::new(tesseras_storage::CasStore::new(
            Arc::clone(&conn),
            base.join("cas"),
        ));
        let blobs = tesseras_storage::FsBlobStore::new(conn, cas);
        use tesseras_core::BlobStore;
        let manifest_data = blobs
            .read(&record.hash, &record.hash, "MANIFEST")
            .map_err(|_| anyhow::anyhow!("manifest not found"))?;
        let manifest_text = String::from_utf8_lossy(&manifest_data).to_string();
        Manifest::parse(&manifest_text)?
    };

    // Extract metadata from first memory record (for tags, language, location)
    let first_meta: Option<MemoryMetadata> = memories.iter().find_map(|m| {
        m.meta_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
    });

    if out.json {
        return print_json(&record, &manifest, &memories, &report, first_meta.as_ref());
    }

    // Header
    let hash_str = record.hash.to_base32();
    if out.color {
        println!("Tessera: {}", hash_str.bold());
    } else {
        println!("Tessera: {hash_str}");
    }

    println!(
        "Created:    {}",
        record.created_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("Visibility: {}", record.visibility);

    if let Some(meta) = &first_meta {
        println!("Language:   {}", meta.language);
        if !meta.tags.is_empty() {
            println!("Tags:       {}", meta.tags.join(", "));
        }
        if let Some(loc) = &meta.location {
            println!("Location:   {}", loc.description);
        }
    }

    // Files section
    println!();
    println!("Files ({}):", manifest.entries.len());
    for entry in &manifest.entries {
        let filename = entry.path.split('/').next_back().unwrap_or(&entry.path);
        let memory_type = infer_type_from_mime(&entry.mime_type);
        let size = format_size(entry.size);
        println!("  {filename:<16} {memory_type:<12} {size:>8}");
    }

    let total: u64 = manifest.entries.iter().map(|e| e.size).sum();
    println!();
    println!("Total size: {}", format_size(total));

    // Signature status
    let sig_str = if report.signature_valid {
        "valid"
    } else {
        "INVALID"
    };
    if out.color {
        if report.signature_valid {
            println!("Signature:  {}", sig_str.green());
        } else {
            println!("Signature:  {}", sig_str.red());
        }
    } else {
        println!("Signature:  {sig_str}");
    }

    Ok(())
}

fn infer_type_from_mime(mime: &str) -> &str {
    match mime {
        "text/plain" => "Reflection",
        "image/jpeg" | "image/png" => "Moment",
        "audio/wav" => "Moment",
        "video/webm" => "Moment",
        "application/json" => "Metadata",
        _ => "Unknown",
    }
}

fn print_json(
    record: &tesseras_core::TesseraRecord,
    manifest: &Manifest,
    memories: &[tesseras_core::MemoryRecord],
    report: &tesseras_core::VerifyReport,
    first_meta: Option<&MemoryMetadata>,
) -> Result<()> {
    let files: Vec<serde_json::Value> = manifest
        .entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "path": e.path,
                "mime_type": e.mime_type,
                "size": e.size,
                "hash": e.hash.to_base32(),
            })
        })
        .collect();

    let total_size: u64 = manifest.entries.iter().map(|e| e.size).sum();

    let mut obj = serde_json::json!({
        "hash": record.hash.to_base32(),
        "created_at": record.created_at.to_rfc3339(),
        "visibility": record.visibility,
        "memory_count": record.memory_count,
        "size_bytes": record.size_bytes,
        "total_file_size": total_size,
        "signature_valid": report.signature_valid,
        "files": files,
    });

    if let Some(meta) = first_meta {
        obj["language"] = serde_json::json!(meta.language);
        if !meta.tags.is_empty() {
            obj["tags"] = serde_json::json!(meta.tags);
        }
        if let Some(loc) = &meta.location {
            obj["location"] = serde_json::json!(loc.description);
        }
    }

    let memories_json: Vec<serde_json::Value> = memories
        .iter()
        .map(|m| {
            serde_json::json!({
                "hash": m.hash.to_base32(),
                "memory_type": m.memory_type,
                "media_path": m.media_path,
            })
        })
        .collect();
    obj["memories"] = serde_json::json!(memories_json);

    println!("{}", serde_json::to_string_pretty(&obj)?);
    Ok(())
}
