+++
title = "Phase 3: Memories in Your Hands"
date = 2026-02-14T14:00:00+00:00
description = "Tesseras now has a Flutter app and an embedded Rust node — anyone can create and preserve memories from their phone."
+++

People can now hold their memories in their hands. Phase 3 delivers what the previous phases built toward: a mobile app where someone downloads Tesseras, creates an identity, takes a photo, and that memory enters the preservation network. No cloud accounts, no subscriptions, no company between you and your memories.

## What was built

**tesseras-embedded** — A full P2P node that runs inside a mobile app. The `EmbeddedNode` struct owns a Tokio runtime, SQLite database, QUIC transport, Kademlia DHT engine, replication service, and tessera service — the same stack as the desktop daemon, compiled into a shared library. A global singleton pattern (`Mutex<Option<EmbeddedNode>>`) ensures one node per app lifecycle. On start, it opens the database, runs migrations, loads or generates an Ed25519 identity with proof-of-work node ID, binds QUIC on an ephemeral port, wires up DHT and replication, and spawns the repair loop. On stop, it sends a shutdown signal and drains gracefully.

Eleven FFI functions are exposed to Dart via flutter_rust_bridge: lifecycle (`node_start`, `node_stop`, `node_is_running`), identity (`create_identity`, `get_identity`), memories (`create_memory`, `get_timeline`, `get_memory`), and network status (`get_network_stats`, `get_replication_status`). All types crossing the FFI boundary are flat structs with only `String`, `Option<String>`, `Vec<String>`, and primitives — no trait objects, no generics, no lifetimes.

Four adapter modules bridge core ports to concrete implementations: `Blake3HasherAdapter`, `Ed25519SignerAdapter`/`Ed25519VerifierAdapter` for cryptography, `DhtPortAdapter` for DHT operations, and `ReplicationHandlerAdapter` for incoming fragment and attestation RPCs.

The `bundled-sqlite` feature flag compiles SQLite from source, required for Android and iOS where the system library may not be available. Cargokit configuration passes this flag automatically in both debug and release builds.

**Flutter app** — A Material Design 3 application with Riverpod state management, targeting Android, iOS, Linux, macOS, and Windows from a single codebase.

The *onboarding flow* is three screens: a welcome screen explaining the project in one sentence ("Preserve your memories across millennia. No cloud. No company."), an identity creation screen that triggers Ed25519 keypair generation in Rust, and a confirmation screen showing the user's name and cryptographic identity.

The *timeline screen* displays memories in reverse chronological order with image previews, context text, and chips for memory type and visibility. Pull-to-refresh reloads from the Rust node. A floating action button opens the *memory creation screen*, which supports photo selection from gallery or camera via `image_picker`, optional context text, memory type and visibility dropdowns, and comma-separated tags. Creating a memory calls the Rust FFI synchronously, then returns to the timeline.

The *network screen* shows two cards: node status (peer count, DHT size, bootstrap state, uptime) and replication health (total fragments, healthy fragments, repairing fragments, replication factor). The *settings screen* displays the user's identity — name, truncated node ID, truncated public key, and creation date.

Three Riverpod providers manage state: `nodeProvider` starts the embedded node on app launch using the app documents directory and stops it on dispose; `identityProvider` loads the existing profile or creates a new one; `timelineProvider` fetches the memory list with pagination.

**Testing** — 9 Rust unit tests in tesseras-embedded covering node lifecycle (start/stop without panic), identity persistence across restarts, restart cycles without SQLite corruption, network event streaming, stats retrieval, memory creation and timeline retrieval, and single memory lookup by hash. 2 Flutter tests: an integration test verifying Rust initialization and app startup, and a widget smoke test.

## Architecture decisions

- **Embedded node, not client-server**: the phone runs the full P2P stack, not a thin client talking to a remote daemon. This means memories are preserved even without internet. Users with a Raspberry Pi or VPS can optionally connect the app to their daemon via GraphQL for higher availability, but it's not required.
- **Synchronous FFI**: all flutter_rust_bridge functions are marked `#[frb(sync)]` and block on the internal Tokio runtime. This simplifies the Dart side (no async bridge complexity) while the Rust side handles concurrency internally. Flutter's UI thread stays responsive because Riverpod wraps calls in async providers.
- **Global singleton**: a `Mutex<Option<EmbeddedNode>>` global ensures the node lifecycle is predictable — one start, one stop, no races. Mobile platforms kill processes aggressively, so simplicity in lifecycle management is a feature.
- **Flat FFI types**: no Rust abstractions leak across the FFI boundary. Every type is a plain struct with strings and numbers. This makes the auto-generated Dart bindings reliable and easy to debug.
- **Three-screen onboarding**: identity creation is the only required step. No email, no password, no server registration. The app generates a cryptographic identity locally and is ready to use.

## What comes next

- **Phase 4: Resilience and Scale** — Advanced NAT traversal (STUN/TURN), Shamir's Secret Sharing for heirs, sealed tesseras with time-lock encryption, performance tuning, security audits, OS packaging for Alpine/Arch/Debian/FreeBSD/OpenBSD
- **Phase 5: Exploration and Culture** — Public tessera browser by era/location/theme/language, institutional curation, genealogy integration, physical media export (M-DISC, microfilm, acid-free paper with QR)

The infrastructure is complete. The network exists, replication works, and now anyone with a phone can participate. What remains is hardening what we have and opening it to the world.
