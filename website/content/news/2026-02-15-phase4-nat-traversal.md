+++
title = "Phase 4: Punching Through NATs"
date = 2026-02-15T18:00:00+00:00
description = "Tesseras nodes can now discover their NAT type via STUN, coordinate UDP hole punching through introducers, and fall back to transparent relay forwarding when direct connectivity fails."
+++

Most people's devices sit behind a NAT — a network address translator that lets them reach the internet but prevents incoming connections. For a P2P network, this is an existential problem: if two nodes behind NATs can't talk to each other, the network fragments. Phase 4 continues with a full NAT traversal stack: STUN-based discovery, coordinated hole punching, and relay fallback.

The approach follows the same pattern as most battle-tested P2P systems (WebRTC, BitTorrent, IPFS): try the cheapest option first, escalate only when necessary. Direct connectivity costs nothing. Hole punching costs a few coordinated packets. Relaying costs sustained bandwidth from a third party. Tesseras tries them in that order.

## What was built

**NatType classification** (`tesseras-core/src/network.rs`) — A new `NatType` enum (Public, Cone, Symmetric, Unknown) added to the core domain layer. This type is shared across the entire stack: the STUN client writes it, the DHT advertises it in Pong messages, and the punch coordinator reads it to decide whether hole punching is even worth attempting (Cone-to-Cone works ~80% of the time; Symmetric-to-Symmetric almost never works).

**STUN client** (`tesseras-net/src/stun.rs`) — A minimal STUN implementation (RFC 5389 Binding Request/Response) that discovers a node's external address. The codec encodes 20-byte binding requests with a random transaction ID and decodes XOR-MAPPED-ADDRESS responses. The `discover_nat()` function queries multiple STUN servers in parallel (Google, Cloudflare by default), compares the mapped addresses, and classifies the NAT type:

- Same IP and port from all servers → **Public** (no NAT)
- Same mapped address from all servers → **Cone** (hole punching works)
- Different mapped addresses → **Symmetric** (hole punching unreliable)
- No responses → **Unknown**

Retries with exponential backoff and configurable timeouts. 12 tests covering codec roundtrips, all classification paths, and async loopback queries.

**Signed punch coordination** (`tesseras-net/src/punch.rs`) — Ed25519 signing and verification for `PunchIntro`, `RelayRequest`, and `RelayMigrate` messages. Every introduction is signed by the initiator with a 30-second timestamp window, preventing reflection attacks (where an attacker replays an old introduction to redirect traffic). The payload format is `target || external_addr || timestamp` — changing any field invalidates the signature. 6 unit tests plus 3 property-based tests with proptest (arbitrary node IDs, ports, and session tokens).

**Relay session manager** (`tesseras-net/src/relay.rs`) — Manages transparent UDP relay sessions between NATed peers. Each session has a random 16-byte token; peers prefix their packets with the token, the relay strips it and forwards. Features:

- Bidirectional forwarding (A→R→B and B→R→A)
- Rate limiting: 256 KB/s for reciprocal peers, 64 KB/s for non-reciprocal
- 10-minute maximum duration for bootstrap (non-reciprocal) sessions
- Address migration: when a peer's IP changes (Wi-Fi to cellular), a signed `RelayMigrate` updates the session without tearing it down
- Idle cleanup with configurable timeout
- 8 unit tests plus 2 property-based tests

**DHT message extensions** (`tesseras-dht/src/message.rs`) — Seven new message variants added to the DHT protocol:

| Message | Purpose |
|---------|---------|
| `PunchIntro` | "I want to connect to node X, here's my signed external address" |
| `PunchRequest` | Introducer forwards the request to the target |
| `PunchReady` | Target confirms readiness, sends its external address |
| `RelayRequest` | "Create a relay session to node X" |
| `RelayOffer` | Relay responds with its address and session token |
| `RelayClose` | Tear down a relay session |
| `RelayMigrate` | Update session after network change |

The `Pong` message was extended with NAT metadata: `nat_type`, `relay_slots_available`, and `relay_bandwidth_used_kbps`. All new fields use `#[serde(default)]` for backward compatibility — old nodes ignore what they don't recognize, new nodes fall back to defaults. 9 new serialization roundtrip tests.

**NatHandler trait and dispatch** (`tesseras-dht/src/engine.rs`) — A new `NatHandler` async trait (5 methods) injected into the DHT engine, following the same dependency injection pattern as the existing `ReplicationHandler`. The engine's message dispatch loop now routes all punch/relay messages to the handler. This keeps the DHT engine protocol-agnostic while allowing the NAT traversal logic to live in `tesseras-net`.

**Mobile reconnection types** (`tesseras-embedded/src/reconnect.rs`) — A three-phase reconnection state machine for mobile devices:

1. **QuicMigration** (0-2s) — try QUIC connection migration for all active peers
2. **ReStun** (2-5s) — re-discover external address via STUN
3. **ReEstablish** (5-10s) — reconnect peers that migration couldn't save

Peers are reconnected in priority order: bootstrap nodes first, then nodes holding our fragments, then nodes whose fragments we hold, then general DHT neighbors. A new `NetworkChanged` event variant was added to the FFI event stream so the Flutter app can show reconnection progress.

**Daemon NAT configuration** (`tesd/src/config.rs`) — A new `[nat]` section in the TOML config with STUN server list, relay toggle, max relay sessions, bandwidth limits (reciprocal vs bootstrap), and idle timeout. All fields have sensible defaults; relay is disabled by default.

**Prometheus metrics** (`tesseras-net/src/metrics.rs`) — 16 metrics across four subsystems:

- **STUN**: requests, failures, latency histogram
- **Punch**: attempts/successes/failures (by NAT type pair), latency histogram
- **Relay**: active sessions, total sessions, bytes forwarded, idle timeouts, rate limit hits
- **Reconnect**: network changes, attempts/successes by phase, duration histogram

6 tests verifying registration, increment, label cardinality, and double-registration detection.

**Integration tests** — Two end-to-end tests using `MemTransport` (in-memory simulated network):

- `punch_integration.rs` — Full 3-node hole-punch flow: A sends signed `PunchIntro` to introducer I, I verifies and forwards `PunchRequest` to B, B verifies the original signature and sends `PunchReady` back, A and B exchange messages directly. Also tests that a bad signature is correctly rejected.
- `relay_integration.rs` — Full 3-node relay flow: A requests relay from R, R creates session and sends `RelayOffer` to both peers, A and B exchange token-prefixed packets through R, A migrates to a new address mid-session, A closes the session, and the test verifies the session is torn down and further forwarding fails.

**Property tests** — 7 proptest-based tests covering: signature round-trips for all three signed message types (arbitrary node IDs, ports, tokens), NAT classification determinism (same inputs always produce same output), STUN binding request validity, session token uniqueness, and relay rejection of too-short packets.

**Justfile targets** — `just test-nat` runs all NAT traversal tests across `tesseras-net` and `tesseras-dht`. `just test-chaos` is a placeholder for future Docker Compose chaos tests with `tc netem`.

## Architecture decisions

- **STUN over TURN**: we implement STUN (discovery) and custom relay rather than full TURN. TURN requires authenticated allocation and is designed for media relay; our relay is simpler — token-prefixed UDP forwarding with rate limits. This keeps the protocol minimal and avoids depending on external TURN servers.
- **Signatures on introductions**: every `PunchIntro` is signed by the initiator. Without this, an attacker could send forged introductions to redirect a node's hole-punch attempts to an attacker-controlled address (a reflection attack). The 30-second timestamp window limits replay.
- **Reciprocal bandwidth tiers**: relay nodes give 4x more bandwidth (256 vs 64 KB/s) to peers with good reciprocity scores. This incentivizes nodes to store fragments for others — if you contribute, you get better relay service when you need it.
- **Backward-compatible Pong extension**: new NAT fields in `Pong` use `#[serde(default)]` and `Option<T>`. Old nodes that don't understand these fields simply skip them during deserialization. No protocol version bump needed.
- **NatHandler as async trait**: the NAT traversal logic is injected into the DHT engine via a trait, just like `ReplicationHandler`. This keeps the DHT engine focused on routing and peer management, and allows the NAT implementation to be swapped or disabled without touching core DHT code.

## What comes next

- **Phase 4 continued** — performance tuning (connection pooling, fragment caching, SQLite WAL), security audits, institutional node onboarding, OS packaging
- **Phase 5: Exploration and Culture** — public tessera browser by era/location/theme/language, institutional curation, genealogy integration, physical media export (M-DISC, microfilm, acid-free paper with QR)

With NAT traversal, Tesseras can connect nodes regardless of their network topology. Public nodes talk directly. Cone-NATed nodes punch through with an introducer's help. Symmetric-NATed or firewalled nodes relay through willing peers. The network adapts to the real world, where most devices are behind a NAT and network conditions change constantly.
