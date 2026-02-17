use tesseras_core::ContentHash;
use tesseras_core::pack::{PackedFile, pack};
use tesseras_core::ports::{BlobStore, MemoryRepository, TesseraRepository};

/// Read all files of a tessera from storage and pack into a single byte buffer.
pub fn pack_tessera(
    hash: &ContentHash,
    tessera_repo: &dyn TesseraRepository,
    memory_repo: &dyn MemoryRepository,
    blob_store: &dyn BlobStore,
) -> Result<Vec<u8>, anyhow::Error> {
    // Verify tessera exists
    let _record = tessera_repo
        .find_by_hash(hash)?
        .ok_or_else(|| anyhow::anyhow!("tessera not found: {hash}"))?;

    let mut files = Vec::new();

    // Pack MANIFEST
    if let Ok(data) = blob_store.read(hash, hash, "MANIFEST") {
        files.push(PackedFile {
            path: "MANIFEST".into(),
            data,
        });
    }

    // Pack signature
    if let Ok(data) = blob_store.read(hash, hash, "ed25519.sig") {
        files.push(PackedFile {
            path: "identity/ed25519.sig".into(),
            data,
        });
    }

    // Pack all memory files
    let memories = memory_repo.list_by_tessera(hash)?;
    for mem in &memories {
        let mem_hash_str = mem.hash.to_string();
        let media_filename = mem.media_path.split('/').next_back().unwrap_or("media.bin");

        if let Ok(data) = blob_store.read(hash, &mem.hash, media_filename) {
            files.push(PackedFile {
                path: format!("memories/{mem_hash_str}/{media_filename}"),
                data,
            });
        }
        if let Ok(data) = blob_store.read(hash, &mem.hash, "context.txt") {
            files.push(PackedFile {
                path: format!("memories/{mem_hash_str}/context.txt"),
                data,
            });
        }
        if let Ok(data) = blob_store.read(hash, &mem.hash, "meta.json") {
            files.push(PackedFile {
                path: format!("memories/{mem_hash_str}/meta.json"),
                data,
            });
        }
    }

    Ok(pack(&files))
}
