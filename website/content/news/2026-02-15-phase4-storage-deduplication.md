+++
title = "Phase 4: Storage Deduplication"
date = 2026-02-15T23:00:00+00:00
description = "A new content-addressable storage layer eliminates duplicate data across tesseras, reducing disk usage and enabling automatic garbage collection."
+++

When multiple tesseras share the same photo, the same audio clip, or the same fragment data, the old storage layer kept separate copies of each. On a node storing thousands of tesseras for the network, this duplication adds up fast. Phase 4 continues with storage deduplication: a content-addressable store (CAS) that ensures every unique piece of data is stored exactly once on disk, regardless of how many tesseras reference it.

The design is simple and proven: hash the content with BLAKE3, use the hash as the filename, and maintain a reference count in SQLite. When two tesseras include the same 5 MB photo, one file exists on disk with a refcount of 2. When one tessera is deleted, the refcount drops to 1 and the file stays. When the last reference is released, a periodic sweep cleans up the orphan.

## What was built

**CAS schema migration** (`tesseras-storage/migrations/004_dedup.sql`) — Three new tables:

- `cas_objects` — tracks every object in the store: BLAKE3 hash (primary key), byte size, reference count, and creation timestamp
- `blob_refs` — maps logical blob identifiers (tessera hash + memory hash + filename) to CAS hashes, replacing the old filesystem path convention
- `fragment_refs` — maps logical fragment identifiers (tessera hash + fragment index) to CAS hashes, replacing the old `fragments/` directory layout

Indexes on the hash columns ensure O(1) lookups during reads and reference counting.

**CasStore** (`tesseras-storage/src/cas.rs`) — The core content-addressable storage engine. Files are stored under a two-level prefix directory: `<root>/<2-char-hex-prefix>/<full-hash>.blob`. The store provides five operations:

- `put(hash, data)` — writes data to disk if not already present, increments refcount. Returns whether a dedup hit occurred.
- `get(hash)` — reads data from disk by hash
- `release(hash)` — decrements refcount. If it reaches zero, the on-disk file is deleted immediately.
- `contains(hash)` — checks existence without reading
- `ref_count(hash)` — returns the current reference count

All operations are atomic within a single SQLite transaction. The refcount is the source of truth — if the refcount says the object exists, the file must be on disk.

**CAS-backed FsBlobStore** (`tesseras-storage/src/blob.rs`) — Rewritten to delegate all storage to the CAS. When a blob is written, its BLAKE3 hash is computed and passed to `cas.put()`. A row in `blob_refs` maps the logical path (tessera + memory + filename) to the CAS hash. Reads look up the CAS hash via `blob_refs` and fetch from `cas.get()`. Deleting a tessera releases all its blob references in a single transaction.

**CAS-backed FsFragmentStore** (`tesseras-storage/src/fragment.rs`) — Same pattern for erasure-coded fragments. Each fragment's BLAKE3 checksum is already computed during Reed-Solomon encoding, so it's used directly as the CAS key. Fragment verification now checks the CAS hash instead of recomputing from scratch — if the CAS says the data is intact, it is.

**Sweep garbage collector** (`cas.rs:sweep()`) — A periodic GC pass that handles three edge cases the normal refcount path can't:

1. **Orphan files** — files on disk with no corresponding row in `cas_objects`. Can happen after a crash mid-write. Files younger than 1 hour are skipped (grace period for in-flight writes); older orphans are deleted.
2. **Leaked refcounts** — rows in `cas_objects` with refcount zero that weren't cleaned up (e.g., if the process died between decrementing and deleting). These rows are removed.
3. **Idempotent** — running sweep twice produces the same result.

The sweep is wired into the existing repair loop in `tesseras-replication`, so it runs automatically every 24 hours alongside fragment health checks.

**Migration from old layout** (`tesseras-storage/src/migration.rs`) — A copy-first migration strategy that moves data from the old directory-based layout (`blobs/<tessera>/<memory>/<file>` and `fragments/<tessera>/<index>.shard`) into the CAS. The migration:

1. Checks the storage version in `storage_meta` (version 1 = old layout, version 2 = CAS)
2. Walks the old `blobs/` and `fragments/` directories
3. Computes BLAKE3 hashes and inserts into CAS via `put()` — duplicates are automatically deduplicated
4. Creates corresponding `blob_refs` / `fragment_refs` entries
5. Removes old directories only after all data is safely in CAS
6. Updates the storage version to 2

The migration runs on daemon startup, is idempotent (safe to re-run), and reports statistics: files migrated, duplicates found, bytes saved.

**Prometheus metrics** (`tesseras-storage/src/metrics.rs`) — Ten new metrics for observability:

| Metric | Description |
|--------|-------------|
| `cas_objects_total` | Total unique objects in the CAS |
| `cas_bytes_total` | Total bytes stored |
| `cas_dedup_hits_total` | Number of writes that found an existing object |
| `cas_bytes_saved_total` | Bytes saved by deduplication |
| `cas_gc_refcount_deletions_total` | Objects deleted when refcount reached zero |
| `cas_gc_sweep_orphans_cleaned_total` | Orphan files removed by sweep |
| `cas_gc_sweep_leaked_refs_cleaned_total` | Leaked refcount rows cleaned |
| `cas_gc_sweep_skipped_young_total` | Young orphans skipped (grace period) |
| `cas_gc_sweep_duration_seconds` | Time spent in sweep GC |

**Property-based tests** — Two proptest tests verify CAS invariants under random inputs:

- `refcount_matches_actual_refs` — after N random put/release operations, the refcount always matches the actual number of outstanding references
- `cas_path_is_deterministic` — the same hash always produces the same filesystem path

**Integration test updates** — All integration tests across `tesseras-core`, `tesseras-replication`, `tesseras-embedded`, and `tesseras-cli` updated for the new CAS-backed constructors. Tamper-detection tests updated to work with the CAS directory layout.

347 tests pass across the workspace. Clippy clean with `-D warnings`.

## Architecture decisions

- **BLAKE3 as CAS key**: the content hash we already compute for integrity verification doubles as the deduplication key. No additional hashing step — the hash computed during `create` or `replicate` is reused as the CAS address.
- **SQLite refcount over filesystem reflinks**: we considered using filesystem-level copy-on-write (reflinks on btrfs/XFS), but that would tie Tesseras to specific filesystems. SQLite refcounting works on any filesystem, including FAT32 on cheap USB drives and ext4 on Raspberry Pis.
- **Two-level hex prefix directories**: storing all CAS objects in a flat directory would slow down filesystems with millions of entries. The `<2-char prefix>/` split limits any single directory to ~65k entries before a second prefix level is needed. This matches the approach used by Git's object store.
- **Grace period for orphan files**: the sweep GC skips files younger than 1 hour to avoid deleting objects that are being written by a concurrent operation. This is a pragmatic choice — it trades a small window of potential orphans for crash safety without requiring fsync or two-phase commit.
- **Copy-first migration**: the migration copies data to CAS before removing old directories. If the process is interrupted, the old data is still intact and migration can be re-run. This is slower than moving files but guarantees no data loss.
- **Sweep in repair loop**: rather than adding a separate GC timer, the CAS sweep piggybacks on the existing 24-hour repair loop. This keeps the daemon simple — one background maintenance cycle handles both fragment health and storage cleanup.

## What comes next

- **Phase 4 continued** — security audits, OS packaging (Alpine, Arch, Debian, OpenBSD, FreeBSD)
- **Phase 5: Exploration and Culture** — public tessera browser by era/location/theme/language, institutional curation, genealogy integration (FamilySearch, Ancestry), physical media export (M-DISC, microfilm, acid-free paper with QR), AI-assisted context

Storage deduplication completes the storage efficiency story for Tesseras. A node that stores fragments for thousands of users — common for institutional nodes and always-on full nodes — now pays the disk cost of unique data only. Combined with Reed-Solomon erasure coding (which already minimizes redundancy at the network level), the system achieves efficient storage at both the local and distributed layers.
