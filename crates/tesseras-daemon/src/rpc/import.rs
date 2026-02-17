//! Import a fetched tessera into local storage by parsing the MANIFEST
//! and creating TesseraRecord + MemoryRecord entries.

use std::collections::HashMap;

use tesseras_core::ContentHash;
use tesseras_core::manifest::Manifest;
use tesseras_core::pack::PackedFile;
use tesseras_core::ports::{BlobStore, MemoryRepository, TesseraRepository};

/// Import unpacked files into local storage (tessera repo, memory repo, blob store).
///
/// Returns `(memory_count, total_bytes)` on success.
pub fn import_tessera(
    files: &[PackedFile],
    tessera_repo: &dyn TesseraRepository,
    memory_repo: &dyn MemoryRepository,
    blob_store: &dyn BlobStore,
) -> Result<(u32, u64), anyhow::Error> {
    // 1. Find and parse MANIFEST
    let manifest_file = files
        .iter()
        .find(|f| f.path == "MANIFEST")
        .ok_or_else(|| anyhow::anyhow!("no MANIFEST in fetched tessera"))?;

    let manifest_text = String::from_utf8_lossy(&manifest_file.data);
    let manifest = Manifest::parse(&manifest_text)?;
    let content_hash = manifest.content_hash;

    // Skip if already stored
    if tessera_repo.exists(&content_hash)? {
        let total_bytes: u64 = files.iter().map(|f| f.data.len() as u64).sum();
        let memories = memory_repo.list_by_tessera(&content_hash)?;
        return Ok((memories.len() as u32, total_bytes));
    }

    // 2. Group memory files by memory hash
    //    Paths look like: "memories/<hash>/media.jpg", "memories/<hash>/context.txt", etc.
    let mut memory_files: HashMap<String, Vec<&PackedFile>> = HashMap::new();
    let mut total_bytes: u64 = 0;

    for file in files {
        total_bytes += file.data.len() as u64;
        if let Some(rest) = file.path.strip_prefix("memories/") {
            if let Some(mem_hash_str) = rest.split('/').next() {
                memory_files
                    .entry(mem_hash_str.to_string())
                    .or_default()
                    .push(file);
            }
        }
    }

    // 3. Store MANIFEST blob (using content_hash for both tessera and memory hash)
    blob_store.write(
        &content_hash,
        &content_hash,
        "MANIFEST",
        &manifest_file.data,
    )?;

    // 4. Store signature blob if present
    for file in files {
        if file.path == "identity/ed25519.sig" || file.path == "ed25519.sig" {
            blob_store.write(&content_hash, &content_hash, "ed25519.sig", &file.data)?;
        }
    }

    // 5. Determine visibility from manifest
    let visibility = match &manifest.encryption {
        Some(enc) if enc.open_after.is_some() => "sealed",
        Some(_) => "private",
        None => "public",
    };

    let sealed_until = manifest.encryption.as_ref().and_then(|e| e.open_after);

    // 6. Store TesseraRecord (must come before memories due to FK)
    let tessera_record = tesseras_core::ports::TesseraRecord {
        hash: content_hash,
        creator_pubkey: manifest.creator.clone(),
        created_at: manifest.created_at,
        size_bytes: total_bytes,
        memory_count: memory_files.len() as u32,
        visibility: visibility.to_string(),
        sealed_until,
        is_mine: false,
    };
    tessera_repo.store(&tessera_record)?;

    // 7. Store each memory's blobs and MemoryRecord
    for (mem_hash_str, mem_files) in &memory_files {
        let mem_hash: ContentHash = mem_hash_str
            .parse()
            .unwrap_or_else(|_| ContentHash::new([0u8; 32]));

        let mut media_path = String::new();
        let mut context_path: Option<String> = None;
        let mut meta_json: Option<String> = None;
        let mut memory_type = "moment".to_string();

        for file in mem_files {
            let filename = file.path.split('/').next_back().unwrap_or("");

            // Store blob
            blob_store.write(&content_hash, &mem_hash, filename, &file.data)?;

            if filename == "meta.json" {
                let json_str = String::from_utf8_lossy(&file.data).to_string();
                // Extract memory_type from meta.json via simple string search
                // Format: "type": "moment" or "type": "reflection"
                if let Some(type_val) = extract_json_string(&json_str, "type") {
                    memory_type = type_val;
                }
                meta_json = Some(json_str);
            } else if filename == "context.txt" {
                context_path = Some(format!("memories/{mem_hash_str}/{filename}"));
            } else if filename != "checksum.blake3" {
                media_path = format!("memories/{mem_hash_str}/{filename}");
            }
        }

        if media_path.is_empty() {
            // Fallback: use first non-metadata file
            if let Some(f) = mem_files.iter().find(|f| {
                let name = f.path.split('/').next_back().unwrap_or("");
                name != "meta.json" && name != "context.txt" && name != "checksum.blake3"
            }) {
                media_path = f.path.clone();
            }
        }

        let memory_record = tesseras_core::ports::MemoryRecord {
            hash: mem_hash,
            tessera_hash: content_hash,
            memory_type,
            media_path,
            context_path,
            meta_json,
            created_at: manifest.created_at,
        };
        memory_repo.store(&memory_record)?;
    }

    Ok((memory_files.len() as u32, total_bytes))
}

/// Extract a string value from JSON without a full parser.
/// Looks for `"key": "value"` patterns.
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\"");
    let key_pos = json.find(&pattern)?;
    let after_key = &json[key_pos + pattern.len()..];
    // Skip whitespace and colon
    let after_colon = after_key.trim_start().strip_prefix(':')?;
    let after_ws = after_colon.trim_start();
    // Extract quoted value
    let after_quote = after_ws.strip_prefix('"')?;
    let end = after_quote.find('"')?;
    Some(after_quote[..end].to_string())
}
