+++
title = "Phase 4: Performance Tuning"
date = 2026-02-15T20:00:00+00:00
description = "SQLite WAL mode with centralized pragma configuration, LRU fragment caching, QUIC connection pool lifecycle management, and attestation hot path optimization."
+++

A P2P network that can traverse NATs but chokes on its own I/O is not much use. Phase 4 continues with performance tuning: centralizing database configuration, caching fragment blobs in memory, managing QUIC connection lifecycles, and eliminating unnecessary disk reads from the attestation hot path.

The guiding principle was the same as the rest of Tesseras: do the simplest thing that actually works. No custom allocators, no lock-free data structures, no premature complexity. A centralized `StorageConfig`, an LRU cache, a connection reaper, and a targeted fix to avoid re-reading blobs that were already checksummed.

## What was built

**Centralized SQLite configuration** (`tesseras-storage/src/database.rs`) — A new `StorageConfig` struct and `open_database()` / `open_in_memory()` functions that apply all SQLite pragmas in one place: WAL journal mode, foreign keys, synchronous mode (NORMAL by default, FULL for unstable hardware like RPi + SD card), busy timeout, page cache size, and WAL autocheckpoint interval. Previously, each call site opened a connection and applied pragmas ad hoc. Now the daemon, CLI, and tests all go through the same path. 7 tests covering foreign keys, busy timeout, journal mode, migrations, synchronous modes, and on-disk WAL file creation.

**LRU fragment cache** (`tesseras-storage/src/cache.rs`) — A `CachedFragmentStore` that wraps any `FragmentStore` with a byte-aware LRU cache. Fragment blobs are cached on read and invalidated on write or delete. When the cache exceeds its configured byte limit, the least recently used entries are evicted. The cache is transparent: it implements `FragmentStore` itself, so the rest of the stack doesn't know it's there. Optional Prometheus metrics track hits, misses, and current byte usage. 3 tests: cache hit avoids inner read, store invalidates cache, eviction when over max bytes.

**Prometheus storage metrics** (`tesseras-storage/src/metrics.rs`) — A `StorageMetrics` struct with three counters/gauges: `fragment_cache_hits`, `fragment_cache_misses`, and `fragment_cache_bytes`. Registered with the Prometheus registry and wired into the fragment cache via `with_metrics()`.

**Attestation hot path fix** (`tesseras-replication/src/service.rs`) — The attestation flow previously read every fragment blob from disk and recomputed its BLAKE3 checksum. Since `list_fragments()` already returns `FragmentId` with a stored checksum, the fix is trivial: use `frag.checksum` instead of `blake3::hash(&data)`. This eliminates one disk read per fragment during attestation — for a tessera with 100 fragments, that's 100 fewer reads. A test with `expect_read_fragment().never()` verifies no blob reads happen during attestation.

**QUIC connection pool lifecycle** (`tesseras-net/src/quinn_transport.rs`) — A `PoolConfig` struct controlling max connections, idle timeout, and reaper interval. `PooledConnection` wraps each `quinn::Connection` with a `last_used` timestamp. When the pool reaches capacity, the oldest idle connection is evicted before opening a new one. A background reaper task (Tokio spawn) periodically closes connections that have been idle beyond the timeout. 4 new pool metrics: `tesseras_conn_pool_size`, `pool_hits_total`, `pool_misses_total`, `pool_evictions_total`.

**Daemon integration** (`tesd/src/config.rs`, `main.rs`) — A new `[performance]` section in the TOML config with fields for SQLite cache size, synchronous mode, busy timeout, fragment cache size, max connections, idle timeout, and reaper interval. The daemon's `main()` now calls `open_database()` with the configured `StorageConfig`, wraps `FsFragmentStore` with `CachedFragmentStore`, and binds QUIC with the configured `PoolConfig`. The direct `rusqlite` dependency was removed from the daemon crate.

**CLI migration** (`tesseras-cli/src/commands/init.rs`, `create.rs`) — Both `init` and `create` commands now use `tesseras_storage::open_database()` with the default `StorageConfig` instead of opening raw `rusqlite` connections. The `rusqlite` dependency was removed from the CLI crate.

## Architecture decisions

- **Decorator pattern for caching**: `CachedFragmentStore` wraps `Box<dyn FragmentStore>` and implements `FragmentStore` itself. This means caching is opt-in, composable, and invisible to consumers. The daemon enables it; tests can skip it.
- **Byte-aware eviction**: the LRU cache tracks total bytes, not entry count. Fragment blobs vary wildly in size (a 4KB text fragment vs a 2MB photo shard), so counting entries would give a misleading picture of memory usage.
- **No connection pool crate**: instead of pulling in a generic pool library, the connection pool is a thin wrapper around `DashMap<SocketAddr, PooledConnection>` with a Tokio reaper. QUIC connections are multiplexed, so the "pool" is really about lifecycle management (idle cleanup, max connections) rather than borrowing/returning.
- **Stored checksums over re-reads**: the attestation fix is intentionally minimal — one line changed, one disk read removed per fragment. The checksums were already stored in SQLite by `store_fragment()`, they just weren't being used.
- **Centralized pragma configuration**: a single `StorageConfig` struct replaces scattered `PRAGMA` calls. The `sqlite_synchronous_full` flag exists specifically for Raspberry Pi deployments where the kernel can crash and lose un-checkpointed WAL transactions.

## What comes next

- **Phase 4 continued** — Shamir's Secret Sharing for heirs, sealed tesseras (time-lock encryption), security audits, institutional node onboarding, storage deduplication, OS packaging
- **Phase 5: Exploration and Culture** — public tessera browser by era/location/theme/language, institutional curation, genealogy integration, physical media export (M-DISC, microfilm, acid-free paper with QR)

With performance tuning in place, Tesseras handles the common case efficiently: fragment reads hit the LRU cache, attestation skips disk I/O, idle QUIC connections are reaped automatically, and SQLite is configured consistently across the entire stack. The next steps focus on cryptographic features (Shamir, time-lock) and hardening for production deployment.
