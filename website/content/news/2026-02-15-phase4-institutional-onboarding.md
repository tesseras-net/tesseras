+++
title = "Phase 4: Institutional Node Onboarding"
date = 2026-02-15T22:00:00+00:00
description = "Libraries, archives, and museums can now join the Tesseras network as verified institutional nodes with DNS-based identity, full-text search indexes, and configurable storage pledges."
+++

A P2P network of individuals is fragile. Hard drives die, phones get lost, people lose interest. The long-term survival of humanity's memories depends on institutions — libraries, archives, museums, universities — that measure their lifetimes in centuries. Phase 4 continues with institutional node onboarding: verified organizations can now pledge storage, run searchable indexes, and participate in the network with a distinct identity.

The design follows a principle of trust but verify: institutions identify themselves via DNS TXT records (the same mechanism used by SPF, DKIM, and DMARC for email), pledge a storage budget, and receive reciprocity exemptions so they can store fragments for others without expecting anything in return. In exchange, the network treats their fragments as higher-quality replicas and limits over-reliance on any single institution through diversity constraints.

## What was built

**Capability bits** (`tesseras-core/src/network.rs`) — Two new flags added to the `Capabilities` bitfield: `INSTITUTIONAL` (bit 7) and `SEARCH_INDEX` (bit 8). A new `institutional_default()` constructor returns the full Phase 2 capability set plus these two bits and `RELAY`. Normal nodes advertise `phase2_default()` which lacks institutional flags. Serialization roundtrip tests verify the new bits survive MessagePack encoding.

**Search types** (`tesseras-core/src/search.rs`) — Three new domain types for the search subsystem:

- `SearchFilters` — query parameters: `memory_type`, `visibility`, `language`, `date_range`, `geo` (bounding box), `page`, `page_size`
- `SearchHit` — a single result: content hash plus a `MetadataExcerpt` (title, description, memory type, creation date, visibility, language, tags)
- `GeoFilter` — bounding box with `min_lat`, `max_lat`, `min_lon`, `max_lon` for spatial queries

All types derive `Serialize`/`Deserialize` for wire transport and `Clone`/`Debug` for diagnostics.

**Institutional daemon config** (`tesd/src/config.rs`) — A new `[institutional]` TOML section with `domain` (the DNS domain to verify), `pledge_bytes` (storage commitment in bytes), and `search_enabled` (toggle for the FTS5 index). The `to_dht_config()` method now sets `Capabilities::institutional_default()` when institutional config is present, so institutional nodes advertise the right capability bits in Pong responses.

**DNS TXT verification** (`tesd/src/institutional.rs`) — Async DNS resolution using `hickory-resolver` to verify institutional identity. The daemon looks up `_tesseras.<domain>` TXT records and parses key-value fields: `v` (version), `node` (hex-encoded node ID), and `pledge` (storage pledge in bytes). Verification checks:

1. A TXT record exists at `_tesseras.<domain>`
2. The `node` field matches the daemon's own node ID
3. The `pledge` field is present and valid

On startup, the daemon attempts DNS verification. If it succeeds, the node runs with institutional capabilities. If it fails, the node logs a warning and downgrades to a normal full node — no crash, no manual intervention.

**CLI setup command** (`tesseras-cli/src/institutional.rs`) — A new `institutional setup` subcommand that guides operators through onboarding:

1. Reads the node's identity from the data directory
2. Prompts for domain name and pledge size
3. Generates the exact DNS TXT record to add: `v=tesseras1 node=<hex> pledge=<bytes>`
4. Writes the institutional section to the daemon's config file
5. Prints next steps: add the TXT record, restart the daemon

**SQLite search index** (`tesseras-storage`) — A migration (`003_institutional.sql`) that creates three structures:

- `search_content` — an FTS5 virtual table for full-text search over tessera metadata (title, description, creator, tags, language)
- `geo_index` — an R-tree virtual table for spatial bounding-box queries over latitude/longitude
- `geo_map` — a mapping table linking R-tree row IDs to content hashes

The `SqliteSearchIndex` adapter implements the `SearchIndex` port trait with `index_tessera()` (insert/update) and `search()` (query with filters). FTS5 queries support natural language search; geo queries use R-tree `INTERSECT` for bounding box lookups. Results are ranked by FTS5 relevance score.

The migration also adds an `is_institutional` column to the `reciprocity` table, handled idempotently via `pragma_table_info` checks (SQLite's `ALTER TABLE ADD COLUMN` lacks `IF NOT EXISTS`).

**Reciprocity bypass** (`tesseras-replication/src/service.rs`) — Institutional nodes are exempt from reciprocity checks. When `receive_fragment()` is called, if the sender's node ID is marked as institutional in the reciprocity ledger, the balance check is skipped entirely. This means institutions can store fragments for the entire network without needing to "earn" credits first — their DNS-verified identity and storage pledge serve as their credential.

**Node-type diversity constraint** (`tesseras-replication/src/distributor.rs`) — A new `apply_institutional_diversity()` function limits how many replicas of a single tessera can land on institutional nodes. The cap is `ceil(replication_factor / 3.5)` — with the default `r=7`, at most 2 of 7 replicas go to institutions. This prevents the network from becoming dependent on a small number of large institutions: if a university's servers go down, at least 5 replicas remain on independent nodes.

**DHT message extensions** (`tesseras-dht/src/message.rs`) — Two new message variants:

| Message | Purpose |
|---------|---------|
| `Search` | Client sends query string, filters, and page number |
| `SearchResult` | Institutional node responds with hits and total count |

The `encode()` function was switched from positional to named MessagePack serialization (`rmp_serde::to_vec_named`) to handle `SearchFilters`' optional fields correctly — positional encoding breaks when `skip_serializing_if` omits fields.

**Prometheus metrics** (`tesd/src/metrics.rs`) — Eight institutional-specific metrics:

- `tesseras_institutional_pledge_bytes` — configured storage pledge
- `tesseras_institutional_stored_bytes` — actual bytes stored
- `tesseras_institutional_pledge_utilization_ratio` — stored/pledged ratio
- `tesseras_institutional_peers_served` — unique peers served fragments
- `tesseras_institutional_search_index_total` — tesseras in the search index
- `tesseras_institutional_search_queries_total` — search queries received
- `tesseras_institutional_dns_verification_status` — 1 if DNS verified, 0 otherwise
- `tesseras_institutional_dns_verification_last` — Unix timestamp of last verification

**Integration tests** — Two tests in `tesseras-replication/tests/integration.rs`:

- `institutional_peer_bypasses_reciprocity` — verifies that an institutional peer with a massive deficit (-999,999 balance) is still allowed to store fragments, while a non-institutional peer with the same deficit is rejected
- `institutional_node_accepts_fragment_despite_deficit` — full async test using `ReplicationService` with mocked DHT, fragment store, reciprocity ledger, and blob store: sends a fragment from an institutional sender and verifies it's accepted

322 tests pass across the workspace. Clippy clean with `-D warnings`.

## Architecture decisions

- **DNS TXT over PKI or blockchain**: DNS is universally deployed, universally understood, and already used for domain verification (SPF, DKIM, Let's Encrypt). Institutions already manage DNS. No certificate authority, no token, no on-chain transaction — just a TXT record. If an institution loses control of their domain, the verification naturally fails on the next check.
- **Graceful degradation on DNS failure**: if DNS verification fails at startup, the daemon downgrades to a normal full node instead of refusing to start. This prevents operational incidents — a DNS misconfiguration shouldn't take a node offline.
- **Diversity cap at `ceil(r / 3.5)`**: with `r=7`, at most 2 replicas go to institutions. This is conservative — it ensures the network never depends on institutions for majority quorum, while still benefiting from their storage capacity and uptime.
- **Named MessagePack encoding**: switching from positional to named encoding adds ~15% overhead per message but eliminates a class of serialization bugs when optional fields are present. The DHT is not bandwidth-constrained at the message level, so the tradeoff is worth it.
- **Reciprocity exemption over credit grants**: rather than giving institutions a large initial credit balance (which is arbitrary and needs tuning), we exempt them entirely. Their DNS-verified identity and public storage pledge replace the bilateral reciprocity mechanism.
- **FTS5 + R-tree in SQLite**: full-text search and spatial indexing are built into SQLite as loadable extensions. No external search engine (Elasticsearch, Meilisearch) needed. This keeps the deployment a single binary with a single database file — critical for institutional operators who may not have a DevOps team.

## What comes next

- **Phase 4 continued** — storage deduplication (content-addressable store with BLAKE3 keying), security audits, OS packaging (Alpine, Arch, Debian, OpenBSD, FreeBSD)
- **Phase 5: Exploration and Culture** — public tessera browser by era/location/theme/language, institutional curation, genealogy integration (FamilySearch, Ancestry), physical media export (M-DISC, microfilm, acid-free paper with QR), AI-assisted context

Institutional onboarding closes a critical gap in Tesseras' preservation model. Individual nodes provide grassroots resilience — thousands of devices across the globe, each storing a few fragments. Institutional nodes provide anchoring — organizations with professional infrastructure, redundant storage, and multi-decade operational horizons. Together, they form a network where memories can outlast both individual devices and individual institutions.
