# Next Steps

Post-simplification rewrite (2-crate workspace: `tesseras` lib + `tes` binary).

## 1. Networking & DHT Integration

- Wire the DHT engine into `Node` so it processes incoming QUIC messages
- Implement the DHT `accept_loop` that dispatches `DhtMessage` variants to `Dht::handle_message`
- Add periodic bootstrap and routing table refresh
- Implement `FindValue` for tessera pointer lookups across the network

## 2. Replication Pipeline

- Connect `replication::encode_fragments` to `Node::add_tessera` so fragments are generated on creation
- Store fragments in the blob CAS with metadata linking them to the parent tessera
- Implement fragment distribution: publish fragment pointers to DHT peers
- Build the repair loop that checks fragment availability and re-replicates missing ones

## 3. Daemon Mode (`tesd`)

- Implement `admin daemon start` — fork/daemonize with PID file
- Run the QUIC accept loop, DHT maintenance, and replication repair as background tasks
- Add Unix socket RPC so `tes` commands can talk to the running daemon

## 4. Peer-to-Peer Tessera Fetch

- When `get_tessera` misses locally, query DHT for the tessera pointer
- Fetch fragments from peers, reconstruct via `decode_fragments`
- Cache reconstructed tesseras locally

## 5. NAT Traversal & Relay

- Implement STUN-based address discovery
- Add relay protocol for peers behind symmetric NAT
- Hole-punching via coordinated QUIC connection attempts

## 6. Mobile / Embedded Node

- Rebuild `tesseras-embedded` as a thin FFI layer over the `tesseras` library
- Regenerate `flutter_rust_bridge` bindings
- Re-integrate with the Flutter app

## 7. Security Hardening

- Add per-IP Sybil protection to DHT (rate limiting, proof-of-work)
- Validate tessera signatures on ingest from peers
- Add message authentication to DHT RPCs

## 8. Persistence & Recovery

- Persist DHT routing table across restarts
- Persist reciprocity ledger (bilateral storage accounting)
- Add database migrations for schema evolution

## 9. Testing

- Add property-based tests (proptest) for serialization roundtrips
- Add network simulation tests (multi-node in-process)
- Make E2E Docker tests actually run against the daemon

## 10. Packaging & Distribution

- Update Debian/Alpine/Arch packages for the new binary names
- Update CI pipelines (SourceHut + GitHub Actions)
- Add shell completions generation to the build

## 11. Storage Quota Enforcement

- [x] Add `max_foreign_storage_bytes` and `max_total_storage_bytes` to `NodeConfig` (default 0 = unlimited)
- [x] Add `total_blob_bytes()`, `foreign_blob_bytes()`, `check_quota()` to `Storage`
- [x] Add `check_storage_quota()` to `Node`, enforce before storing foreign blobs
- [x] Enforce quota in `fetch_tessera_from_network`, `fetch_and_reconstruct_blob`, and repair loop
- [x] Call `record_bytes_stored` when storing blobs fetched from peers
- [x] Add storage usage fields to `RpcResponse::Status`
- [x] Show storage usage in daemon status output
