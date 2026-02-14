+++
title = "Phase 1: Nodes Find Each Other"
date = 2026-02-14
description = "Tesseras nodes can now discover peers, form a Kademlia DHT over QUIC, and publish and find tessera pointers across the network."
+++

Tesseras is no longer a local-only tool. Phase 1 delivers the networking layer: nodes discover each other through a Kademlia DHT, communicate over QUIC, and publish tessera pointers that any peer on the network can find. A tessera created on node A is now findable from node C.

## What was built

**tesseras-core** (updated) — New network domain types: `TesseraPointer` (lightweight reference to a tessera's holders and fragment locations), `NodeIdentity` (node ID + public key + proof-of-work nonce), `NodeInfo` (identity + address + capabilities), and `Capabilities` (bitflags for what a node supports: DHT, storage, relay, replication).

**tesseras-net** — The transport layer, built on QUIC via quinn. The `Transport` trait defines the port: `send`, `recv`, `disconnect`, `local_addr`. Two adapters implement it:

- `QuinnTransport` — real QUIC with self-signed TLS, ALPN negotiation (`tesseras/1`), connection pooling via DashMap, and a background accept loop that handles incoming streams.
- `MemTransport` + `SimNetwork` — in-memory channels for deterministic testing without network I/O. Every integration test in the DHT crate runs against this.

The wire protocol uses length-prefixed MessagePack: a 4-byte big-endian length header followed by an rmp-serde payload. `WireMessage` carries a version byte, request ID, and a body that can be a request, response, or protocol-level error. Maximum message size is 64 KiB.

**tesseras-dht** — A complete Kademlia implementation:

- *Routing table*: 160 k-buckets with k=20. Least-recently-seen eviction, move-to-back on update, ping-check before replacing a full bucket's oldest entry.
- *XOR distance*: 160-bit XOR metric with bucket indexing by highest differing bit.
- *Proof-of-work*: nodes grind a nonce until `BLAKE3(pubkey || nonce)[..20]` has 8 leading zero bits (~256 hash attempts on average). Cheap enough for any device, expensive enough to make Sybil attacks impractical at scale.
- *Protocol messages*: Ping/Pong, FindNode/FindNodeResponse, FindValue/FindValueResult, Store — all serialized with MessagePack via serde.
- *Pointer store*: bounded in-memory store with configurable TTL (24 hours default) and max entries (10,000 default). When full, evicts pointers furthest from the local node ID, following Kademlia's distance-based responsibility model.
- *DhtEngine*: the main orchestrator. Handles incoming RPCs, runs iterative lookups (alpha=3 parallelism), bootstrap, publish, and find. The `run()` method drives a `tokio::select!` loop with maintenance timers: routing table refresh every 60 seconds, pointer expiry every 5 minutes.

**tesseras-daemon** — A full-node binary. Parses CLI args (bind address, bootstrap peers, data directory), generates a PoW-valid node identity, binds a QUIC endpoint, bootstraps into the network, and runs the DHT engine. Graceful shutdown on Ctrl+C via tokio signal handling.

**Infrastructure** — OpenTofu configuration for two Hetzner Cloud bootstrap nodes (cx22 instances in Falkenstein, Germany and Helsinki, Finland). Cloud-init provisioning script creates a dedicated `tesseras` user, writes a config file, and sets up a systemd service. Firewall rules open UDP 4433 (QUIC) and restrict metrics to internal access.

**Testing** — 139 tests across the workspace:

- 47 unit tests in tesseras-dht (routing table, distance, PoW, pointer store, message serialization, engine RPCs)
- 5 multi-node integration tests (3-node bootstrap, 10-node lookup convergence, publish-and-find, node departure detection, PoW rejection)
- 14 tests in tesseras-net (codec roundtrips, transport send/recv, backpressure, disconnect)
- Docker Compose smoke tests with 3 containerized nodes communicating over real QUIC
- Zero clippy warnings, clean formatting

## Architecture decisions

- **Transport as a port**: the `Transport` trait is the only interface between the DHT engine and the network. Swapping QUIC for any other protocol means implementing four methods. All DHT tests use the in-memory adapter, making them fast and deterministic.
- **One stream per RPC**: each DHT request-response pair uses a fresh bidirectional QUIC stream. No multiplexing complexity, no head-of-line blocking between independent operations. QUIC handles the multiplexing at the connection level.
- **MessagePack over Protobuf**: compact binary encoding without code generation or schema files. Serde integration means adding a field to a message is a one-line change. Trade-off: no built-in schema evolution guarantees, but at this stage velocity matters more.
- **PoW instead of stake or reputation**: a node identity costs ~256 BLAKE3 hashes. This runs in under a second on any hardware, including a Raspberry Pi, but generating thousands of identities for a Sybil attack becomes expensive. No tokens, no blockchain, no external dependencies.
- **Iterative lookup with routing table updates**: discovered nodes are added to the routing table as they're encountered during iterative lookups, following standard Kademlia behavior. This ensures the routing table improves organically as nodes interact.

## What comes next

- **Phase 2: Replication** — Reed-Solomon erasure coding over the network, fragment distribution, automatic repair loops, bilateral reciprocity ledger (no blockchain, no tokens)
- **Phase 3: API and Apps** — Flutter mobile/desktop app via flutter_rust_bridge, GraphQL API (async-graphql), WASM browser node
- **Phase 4: Resilience and Scale** — ML-DSA post-quantum signatures, advanced NAT traversal, Shamir's Secret Sharing for heirs, packaging for Alpine/Arch/Debian/FreeBSD/OpenBSD, CI on SourceHut
- **Phase 5: Exploration and Culture** — public tessera browser, institutional curation, genealogy integration, physical media export

Nodes can find each other. Next, they learn to keep each other's memories alive.
