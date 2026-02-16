# Roadmap

## Phase 0 — Foundation (3-4 months)

**Goal**: working CLI that creates, verifies, and exports tesseras offline.

**Crates**: `tesseras-core`, `tesseras-crypto`, `tesseras-storage`, `tesseras-cli`
**Tools**: `tessera-export`, `tessera-import`, `tessera-verify`

Tasks:

1. **tesseras-core**: domain types (Tessera, Memory, MemoryType, Visibility, Manifest, NodeId, ContentHash). Serialization of tessera directory format. Manifest parser/generator.
2. **tesseras-crypto**: BLAKE3 hashing. Ed25519 keygen/sign/verify. ML-DSA keygen/sign/verify. Dual signature creation and verification.
3. **tesseras-storage**: SQLite schema and migrations. CRUD for tesseras and memories. Blob storage on filesystem. Integrity verification.
4. **tesseras-cli**: `init` (generate identity, config, db), `create` (import directory, prompt context, convert formats, generate manifest/checksums/signatures), `verify` (check checksums and signatures), `export` (to open directory), `list`.
5. **tessera-import**: batch import from directories, WhatsApp exports, Google Photos takeout. Media format conversion (HEIC→JPEG, H.265→WebM+JPEG, AAC→WAV).
6. **README.decode**: multilingual self-describing document + format decoding instructions in `decode/`. Minimum: English, Mandarin, Spanish, Arabic, Hindi, Portuguese.
7. **Tests**: serialization roundtrip, full create→verify→export cycle, property-based manifest parsing.

**Deliverable**: `tesseras create ./my-photos` creates a valid tessera. `tesseras verify ./tessera-abc` validates integrity. Everything offline.

## Phase 1 — Basic Network (3-4 months)

**Goal**: functional DHT where nodes discover each other and publish/find tessera pointers.

**Crates**: `tesseras-net`, `tesseras-dht`, update `tesd`

Tasks:

1. **tesseras-net**: QUIC transport (quinn), connection manager, TLS 1.3 with self-signed certs, pooling, NAT detection, mDNS local discovery.
2. **tesseras-dht**: Kademlia routing table (k=20, 160 buckets), XOR distance, iterative lookup (alpha=3), PING/FIND_NODE/FIND_VALUE/STORE RPCs, bucket refresh, eviction policy, bootstrap process.
3. **tesd**: main binary with QUIC listener, DHT init, local storage. Publish pointers on startup, respond to queries, background tasks, graceful shutdown.
4. **Wire protocol**: MessagePack serialization, request/response correlation over QUIC streams, timeouts.
5. **Bootstrap infra**: 3-5 nodes (2x Hetzner EU, 1x DO São Paulo, 1x RPi). DNS TXT records.
6. **Tests**: routing table correctness, 10+ node integration, lookup convergence, NAT traversal.

**Deliverable**: 3+ nodes form a network. Publishing on node A is findable from node C.

## Phase 2 — Replication (2-3 months)

**Goal**: tesseras are fragmented, distributed, and automatically repaired.

**Crates**: `tesseras-replication`, update `tesseras-crypto`, `tesseras-dht`

Tasks:

1. **tesseras-crypto**: Reed-Solomon erasure coding. Fragment/reconstruct API.
2. **tesseras-replication**: fragment manager, distribution engine, repair loop, reciprocity ledger, attestation, repair budget.
3. **REPLICATE and ATTEST RPCs** in DHT message handling.
4. **Prometheus metrics** for replication and network.
5. **Tests**: erasure roundtrip, replication under churn, reciprocity accounting, attestation verification.

**Deliverable**: tessera survives node failures. Fragments repair automatically.

## Phase 3 — API and Apps (3-4 months)

**Goal**: normal people can use Tesseras through a beautiful interface.

**Crates**: `tesseras-api`, `tesseras-embedded`. **Apps**: Flutter.

Tasks:

1. **tesseras-api**: GraphQL schema (queries, mutations, subscriptions), auth, file upload.
2. **tesseras-embedded**: flutter_rust_bridge integration, all core functionality via FFI, compile to Android/iOS/Linux/macOS/Windows.
3. **Flutter app**: camera, memory creation flow, timeline, explorer, network dashboard, settings, offline mode, background sync, adaptive layout, export.
4. **Onboarding**: download → create identity (auto) → record first memory → done. No mention of P2P internals.
5. **Tests**: GraphQL integration, FFI roundtrip, widget tests.

**Deliverable**: someone downloads the app, creates a memory with photo + audio, it's preserved on the network.

## Phase 4 — Resilience and Scale (ongoing)

- Advanced NAT traversal (STUN/TURN)
- Shamir's Secret Sharing for heirs
- Sealed tesseras (time-lock encryption)
- Performance tuning (connection pooling, fragment caching, SQLite WAL)
- Security audits
- Institutional node onboarding
- Storage deduplication

## Phase 5 — Exploration and Culture (future)

- Public tessera browser by era/location/theme/language
- Institutional curation
- Genealogy integration (FamilySearch, Ancestry)
- Physical media export (M-DISC, microfilm, acid-free paper with QR)
- AI-assisted context (prompts, auto-transcribe)
- 20+ language README.decode
- Academic partnerships
