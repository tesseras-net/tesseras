use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::cas::CasStore;
use crate::error::StorageError;
use tesseras_core::ContentHash;

/// Statistics from the dedup migration.
#[derive(Debug, Default)]
pub struct MigrationStats {
    pub files_migrated: u64,
    pub files_skipped: u64,
    pub files_failed: u64,
    pub duplicates_found: u64,
    pub bytes_saved: u64,
}

/// Get the current storage version. Returns "1" for pre-CAS, "2" for CAS.
pub fn storage_version(conn: &rusqlite::Connection) -> Result<String, StorageError> {
    conn.query_row(
        "SELECT value FROM storage_meta WHERE key = 'storage_version'",
        [],
        |row| row.get(0),
    )
    .map_err(|e| StorageError::Database(e.to_string()))
}

/// Set the storage version.
fn set_storage_version(conn: &rusqlite::Connection, version: &str) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE storage_meta SET value = ?1 WHERE key = 'storage_version'",
        rusqlite::params![version],
    )
    .map_err(|e| StorageError::Database(e.to_string()))?;
    Ok(())
}

/// Migrate blobs from old layout to CAS.
/// Old layout: `<blobs_dir>/<tessera_hash>/<memory_hash>/<filename>`
fn migrate_blobs(
    blobs_dir: &Path,
    cas: &CasStore,
    conn: &Arc<Mutex<rusqlite::Connection>>,
    stats: &mut MigrationStats,
) -> Result<(), StorageError> {
    if !blobs_dir.exists() {
        return Ok(());
    }

    for tessera_entry in std::fs::read_dir(blobs_dir)? {
        let tessera_entry = tessera_entry?;
        if !tessera_entry.file_type()?.is_dir() {
            continue;
        }
        let tessera_hash_str = tessera_entry.file_name().to_string_lossy().to_string();

        for memory_entry in std::fs::read_dir(tessera_entry.path())? {
            let memory_entry = memory_entry?;
            if !memory_entry.file_type()?.is_dir() {
                continue;
            }
            let memory_hash_str = memory_entry.file_name().to_string_lossy().to_string();

            for file_entry in std::fs::read_dir(memory_entry.path())? {
                let file_entry = file_entry?;
                if !file_entry.file_type()?.is_file() {
                    continue;
                }
                let filename = file_entry.file_name().to_string_lossy().to_string();

                match std::fs::read(file_entry.path()) {
                    Ok(data) => match cas.put(&data) {
                        Ok((cas_hash, is_dedup)) => {
                            let db = conn.lock().unwrap();
                            let result = db.execute(
                                "INSERT OR IGNORE INTO blob_refs (tessera_hash, memory_hash, filename, blake3_hash)
                                 VALUES (?1, ?2, ?3, ?4)",
                                rusqlite::params![
                                    tessera_hash_str,
                                    memory_hash_str,
                                    filename,
                                    cas_hash.to_string(),
                                ],
                            );
                            drop(db);

                            match result {
                                Ok(_) => {
                                    if is_dedup {
                                        stats.duplicates_found += 1;
                                        stats.bytes_saved += data.len() as u64;
                                    }
                                    stats.files_migrated += 1;
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        path = %file_entry.path().display(),
                                        error = %e,
                                        "failed to insert blob_ref during migration"
                                    );
                                    stats.files_failed += 1;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                path = %file_entry.path().display(),
                                error = %e,
                                "failed to put blob in CAS during migration"
                            );
                            stats.files_failed += 1;
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            path = %file_entry.path().display(),
                            error = %e,
                            "failed to read file during migration"
                        );
                        stats.files_failed += 1;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Migrate fragments from old layout to CAS.
/// Old layout: `<fragments_dir>/<tessera_hash>/<index>.shard`
fn migrate_fragments(
    fragments_dir: &Path,
    cas: &CasStore,
    conn: &Arc<Mutex<rusqlite::Connection>>,
    stats: &mut MigrationStats,
) -> Result<(), StorageError> {
    if !fragments_dir.exists() {
        return Ok(());
    }

    for tessera_entry in std::fs::read_dir(fragments_dir)? {
        let tessera_entry = tessera_entry?;
        if !tessera_entry.file_type()?.is_dir() {
            continue;
        }
        let tessera_hash_str = tessera_entry.file_name().to_string_lossy().to_string();

        for shard_entry in std::fs::read_dir(tessera_entry.path())? {
            let shard_entry = shard_entry?;
            if !shard_entry.file_type()?.is_file() {
                continue;
            }
            let fname = shard_entry.file_name().to_string_lossy().to_string();
            // Parse index from "NNN.shard"
            let Some(index_str) = fname.strip_suffix(".shard") else {
                continue;
            };
            let Ok(index) = index_str.parse::<u16>() else {
                continue;
            };

            match std::fs::read(shard_entry.path()) {
                Ok(data) => match cas.put(&data) {
                    Ok((cas_hash, is_dedup)) => {
                        let db = conn.lock().unwrap();
                        let result = db.execute(
                            "INSERT OR IGNORE INTO fragment_refs (tessera_hash, fragment_index, blake3_hash)
                             VALUES (?1, ?2, ?3)",
                            rusqlite::params![
                                tessera_hash_str,
                                index,
                                cas_hash.to_string(),
                            ],
                        );
                        drop(db);

                        match result {
                            Ok(_) => {
                                if is_dedup {
                                    stats.duplicates_found += 1;
                                    stats.bytes_saved += data.len() as u64;
                                }
                                stats.files_migrated += 1;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    path = %shard_entry.path().display(),
                                    error = %e,
                                    "failed to insert fragment_ref during migration"
                                );
                                stats.files_failed += 1;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %shard_entry.path().display(),
                            error = %e,
                            "failed to put fragment in CAS during migration"
                        );
                        stats.files_failed += 1;
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        path = %shard_entry.path().display(),
                        error = %e,
                        "failed to read shard during migration"
                    );
                    stats.files_failed += 1;
                }
            }
        }
    }
    Ok(())
}

/// Run the full migration from storage_version 1 to 2.
/// Copy-first strategy: original files remain intact until migration completes.
pub fn migrate_to_cas(
    data_dir: &Path,
    cas: &CasStore,
    conn: &Arc<Mutex<rusqlite::Connection>>,
) -> Result<MigrationStats, StorageError> {
    let version = {
        let db = conn.lock().unwrap();
        storage_version(&db)?
    };

    if version != "1" {
        return Ok(MigrationStats::default());
    }

    tracing::info!("starting CAS migration from storage_version 1 to 2");

    let mut stats = MigrationStats::default();

    let blobs_dir = data_dir.join("blobs");
    migrate_blobs(&blobs_dir, cas, conn, &mut stats)?;

    let fragments_dir = data_dir.join("fragments");
    migrate_fragments(&fragments_dir, cas, conn, &mut stats)?;

    // Set storage version to 2
    {
        let db = conn.lock().unwrap();
        set_storage_version(&db, "2")?;
    }

    tracing::info!(
        files_migrated = stats.files_migrated,
        duplicates_found = stats.duplicates_found,
        bytes_saved = stats.bytes_saved,
        files_failed = stats.files_failed,
        "CAS migration complete"
    );

    // Remove old directories (best-effort, after successful migration)
    if blobs_dir.exists() {
        let _ = std::fs::remove_dir_all(&blobs_dir);
    }
    if fragments_dir.exists() {
        let _ = std::fs::remove_dir_all(&fragments_dir);
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (Arc<Mutex<rusqlite::Connection>>, Arc<CasStore>, TempDir) {
        let dir = TempDir::new().unwrap();
        let conn =
            crate::database::open_in_memory(&crate::StorageConfig::default()).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let cas = Arc::new(CasStore::new(
            Arc::clone(&conn),
            dir.path().join("cas"),
        ));
        (conn, cas, dir)
    }

    #[test]
    fn migrate_empty_is_noop() {
        let (conn, cas, dir) = setup();
        let stats = migrate_to_cas(dir.path(), &cas, &conn).unwrap();
        assert_eq!(stats.files_migrated, 0);
        let db = conn.lock().unwrap();
        assert_eq!(storage_version(&db).unwrap(), "2");
    }

    #[test]
    fn migrate_blobs_copies_to_cas() {
        let (conn, cas, dir) = setup();

        // Create old-layout blob
        let blob_dir = dir.path().join("blobs").join("aaa").join("bbb");
        std::fs::create_dir_all(&blob_dir).unwrap();
        std::fs::write(blob_dir.join("media.jpg"), b"photo data").unwrap();

        let stats = migrate_to_cas(dir.path(), &cas, &conn).unwrap();
        assert_eq!(stats.files_migrated, 1);
        assert_eq!(stats.duplicates_found, 0);

        // Verify data is in CAS
        let hash = ContentHash::new(blake3::hash(b"photo data").into());
        assert!(cas.contains(&hash).unwrap());

        // Verify blob_refs entry
        let db = conn.lock().unwrap();
        let exists: bool = db
            .prepare("SELECT 1 FROM blob_refs WHERE tessera_hash = 'aaa' AND memory_hash = 'bbb' AND filename = 'media.jpg'")
            .unwrap()
            .exists([])
            .unwrap();
        assert!(exists);
    }

    #[test]
    fn migrate_finds_duplicates() {
        let (conn, cas, dir) = setup();

        // Create two blobs with identical content in different tesseras
        let blob1 = dir.path().join("blobs").join("t1").join("m1");
        let blob2 = dir.path().join("blobs").join("t2").join("m2");
        std::fs::create_dir_all(&blob1).unwrap();
        std::fs::create_dir_all(&blob2).unwrap();
        std::fs::write(blob1.join("photo.jpg"), b"same photo").unwrap();
        std::fs::write(blob2.join("photo.jpg"), b"same photo").unwrap();

        let stats = migrate_to_cas(dir.path(), &cas, &conn).unwrap();
        assert_eq!(stats.files_migrated, 2);
        assert_eq!(stats.duplicates_found, 1);
        assert_eq!(stats.bytes_saved, 10); // len("same photo")
    }

    #[test]
    fn migrate_handles_corrupted_files() {
        let (conn, cas, dir) = setup();

        // Create a valid blob and an empty (corrupted) blob
        let blob1 = dir.path().join("blobs").join("t1").join("m1");
        std::fs::create_dir_all(&blob1).unwrap();
        std::fs::write(blob1.join("good.jpg"), b"good data").unwrap();
        std::fs::write(blob1.join("empty.jpg"), b"").unwrap(); // empty but readable

        let stats = migrate_to_cas(dir.path(), &cas, &conn).unwrap();
        // Both should migrate (empty files are valid, just unusual)
        assert_eq!(stats.files_migrated, 2);
        assert_eq!(stats.files_failed, 0);
    }

    #[test]
    fn migrate_fragments_copies_to_cas() {
        let (conn, cas, dir) = setup();

        let frag_dir = dir.path().join("fragments").join("ttt");
        std::fs::create_dir_all(&frag_dir).unwrap();
        std::fs::write(frag_dir.join("000.shard"), b"shard data 0").unwrap();
        std::fs::write(frag_dir.join("001.shard"), b"shard data 1").unwrap();

        let stats = migrate_to_cas(dir.path(), &cas, &conn).unwrap();
        assert_eq!(stats.files_migrated, 2);

        // Verify fragment_refs
        let db = conn.lock().unwrap();
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM fragment_refs WHERE tessera_hash = 'ttt'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn migrate_does_not_rerun() {
        let (conn, cas, dir) = setup();

        let blob_dir = dir.path().join("blobs").join("t1").join("m1");
        std::fs::create_dir_all(&blob_dir).unwrap();
        std::fs::write(blob_dir.join("a.jpg"), b"data").unwrap();

        let stats1 = migrate_to_cas(dir.path(), &cas, &conn).unwrap();
        assert_eq!(stats1.files_migrated, 1);

        // Second run should be no-op (version is already 2)
        let stats2 = migrate_to_cas(dir.path(), &cas, &conn).unwrap();
        assert_eq!(stats2.files_migrated, 0);
    }

    #[test]
    fn migrate_removes_old_dirs() {
        let (conn, cas, dir) = setup();

        let blob_dir = dir.path().join("blobs").join("t1").join("m1");
        std::fs::create_dir_all(&blob_dir).unwrap();
        std::fs::write(blob_dir.join("a.jpg"), b"data").unwrap();

        migrate_to_cas(dir.path(), &cas, &conn).unwrap();

        assert!(!dir.path().join("blobs").exists());
    }
}
