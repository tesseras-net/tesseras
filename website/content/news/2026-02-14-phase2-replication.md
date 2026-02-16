+++
title = "Phase 2: Memories Survive"
date = 2026-02-14T12:00:00+00:00
description = "Tesseras now fragments, distributes, and automatically repairs data across the network using Reed-Solomon erasure coding and a bilateral reciprocity ledger."
+++

A tessera is no longer tied to a single machine. Phase 2 delivers the replication layer: data is split into erasure-coded fragments, distributed across multiple peers, and automatically repaired when nodes go offline. A bilateral reciprocity ledger ensures fair storage exchange — no blockchain, no tokens.

## What was built

**tesseras-core** (updated) — New replication domain types: `FragmentPlan` (selects fragmentation tier based on tessera size), `FragmentId` (tessera hash + index + shard count + checksum), `FragmentEnvelope` (fragment with its metadata for wire transport), `FragmentationTier` (Small/Medium/Large), `Attestation` (proof that a node holds a fragment at a given time), and `ReplicateAck` (acknowledgement of fragment receipt). Three new port traits define the hexagonal boundaries: `DhtPort` (find peers, replicate fragments, request attestations, ping), `FragmentStore` (store/read/delete/list/verify fragments), and `ReciprocityLedger` (record storage exchanges, query balances, find best peers). Maximum tessera size is 1 GB.

**tesseras-crypto** (updated) — The existing `ReedSolomonCoder` now powers fragment encoding. Data is split into shards, parity shards are computed, and any combination of data shards can reconstruct the original — as long as the number of missing shards does not exceed the parity count.

**tesseras-storage** (updated) — Two new adapters:

- `FsFragmentStore` — stores fragment data as files on disk (`{root}/{tessera_hash}/{index:03}.shard`) with a SQLite metadata index tracking tessera hash, shard index, shard count, checksum, and byte size. Verification recomputes the BLAKE3 hash and compares it to the stored checksum.
- `SqliteReciprocityLedger` — bilateral storage accounting in SQLite. Each peer has a row tracking bytes stored for them and bytes they store for us. The `balance` column is a generated column (`bytes_they_store_for_us - bytes_stored_for_them`). UPSERT ensures atomic increment of counters.

New migration (`002_replication.sql`) adds tables for fragments, fragment plans, holders, holder-fragment mappings, and reciprocity balances.

**tesseras-dht** (updated) — Four new message variants: `Replicate` (send a fragment envelope), `ReplicateAck` (confirm receipt), `AttestRequest` (ask a node to prove it holds a tessera's fragments), and `AttestResponse` (return attestation with checksums and timestamp). The engine handles these in its message dispatch loop.

**tesseras-replication** — The new crate, with five modules:

- *Fragment encoding* (`fragment.rs`): `encode_tessera()` selects the fragmentation tier based on size, then calls Reed-Solomon encoding for Medium and Large tiers. Three tiers:
  - **Small** (< 4 MB): whole-file replication to r=7 peers, no erasure coding
  - **Medium** (4–256 MB): 16 data + 8 parity shards, distributed across r=7 peers
  - **Large** (≥ 256 MB): 48 data + 24 parity shards, distributed across r=7 peers

- *Distribution* (`distributor.rs`): subnet diversity filtering limits peers per /24 IPv4 subnet (or /48 IPv6 prefix) to avoid correlated failures. If all your fragments land on the same rack, a single power outage kills them all.

- *Service* (`service.rs`): `ReplicationService` is the orchestrator. `replicate_tessera()` encodes the data, finds the closest peers via DHT, applies subnet diversity, and distributes fragments round-robin. `receive_fragment()` validates the BLAKE3 checksum, checks reciprocity balance (rejects if the sender's deficit exceeds the configured threshold), stores the fragment, and updates the ledger. `handle_attestation_request()` lists local fragments and computes their checksums as proof of possession.

- *Repair* (`repair.rs`): `check_tessera_health()` requests attestations from known holders, falls back to ping for unresponsive nodes, verifies local fragment integrity, and returns one of three actions: `Healthy`, `NeedsReplication { deficit }`, or `CorruptLocal { fragment_index }`. The repair loop runs every 24 hours (with 2-hour jitter) via `tokio::select!` with shutdown integration.

- *Configuration* (`config.rs`): `ReplicationConfig` with defaults for repair interval (24h), jitter (2h), concurrent transfers (4), minimum free space (1 GB), deficit allowance (256 MB), and per-peer storage limit (1 GB).

**tesd** (updated) — The daemon now opens a SQLite database (`db/tesseras.db`), runs migrations, creates `FsFragmentStore`, `SqliteReciprocityLedger`, and `FsBlobStore` instances, wraps the DHT engine in a `DhtPortAdapter`, builds a `ReplicationService`, and spawns the repair loop as a background task with graceful shutdown.

**Testing** — 193 tests across the workspace:

- 15 unit tests in tesseras-replication (fragment encoding tiers, checksum validation, subnet diversity, repair health checks, service receive/replicate flows)
- 3 integration tests with real storage (full encode→distribute→receive cycle for medium tessera, small whole-file replication, tampered fragment rejection)
- Tests use in-memory SQLite + tempdir fragments with mockall mocks for DHT and BlobStore
- Zero clippy warnings, clean formatting

## Architecture decisions

- **Three-tier fragmentation**: small files don't need erasure coding — the overhead isn't worth it. Medium and large files get progressively more parity shards. This avoids wasting storage on small tesseras while providing strong redundancy for large ones.
- **Owner-push distribution**: the tessera owner encodes fragments and pushes them to peers, rather than peers pulling. This simplifies the protocol (no negotiation phase) and ensures fragments are distributed immediately.
- **Bilateral reciprocity without consensus**: each node tracks its own balance with each peer locally. No global ledger, no token, no blockchain. If peer A stores 500 MB for peer B, peer B should store roughly 500 MB for peer A. Free riders lose redundancy gradually — their fragments are deprioritized for repair, but never deleted.
- **Subnet diversity**: fragments are spread across different network subnets to survive correlated failures. A datacenter outage shouldn't take out all copies of a tessera.
- **Attestation-first health checks**: the repair loop asks holders to prove possession (attestation with checksums) before declaring a tessera degraded. Only when attestation fails does it fall back to a simple ping. This catches silent data corruption, not just node departure.

## What comes next

- **Phase 3: API and Apps** — Flutter mobile/desktop app via flutter_rust_bridge, GraphQL API (async-graphql), WASM browser node
- **Phase 4: Resilience and Scale** — ML-DSA post-quantum signatures, advanced NAT traversal, Shamir's Secret Sharing for heirs, packaging for Alpine/Arch/Debian/FreeBSD/OpenBSD, CI on SourceHut
- **Phase 5: Exploration and Culture** — public tessera browser, institutional curation, genealogy integration, physical media export

Nodes can find each other and keep each other's memories alive. Next, we give people a way to hold their memories in their hands.
