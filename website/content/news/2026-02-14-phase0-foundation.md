+++
title = "Phase 0: Foundation Laid"
date = 2026-02-14
description = "The foundation crates for Tesseras are now in place — core domain types, cryptographic primitives, SQLite storage, and a working CLI."
+++

The first milestone of the Tesseras project is complete. Phase 0 establishes the foundation that every future component will build on: domain types, cryptography, storage, and a usable command-line interface.

## What was built

**tesseras-core** — The domain layer defines the tessera format: `ContentHash` (BLAKE3, 32 bytes), `NodeId` (Kademlia, 20 bytes), memory types (Moment, Reflection, Daily, Relation, Object), visibility modes (Private, Circle, Public, PublicAfterDeath, Sealed), and a plain-text manifest format that can be parsed by any programming language for the next thousand years. The application service layer (`TesseraService`) handles create, verify, export, and list operations through port traits, following hexagonal architecture.

**tesseras-crypto** — Ed25519 key generation, signing, and verification. A dual-signature framework (Ed25519 + ML-DSA placeholder) ready for post-quantum migration. BLAKE3 content hashing. Reed-Solomon erasure coding behind a feature flag for future replication.

**tesseras-storage** — SQLite index via sqlx with compile-time checked migrations. Filesystem blob store with content-addressable layout (`blobs/<tessera_hash>/<memory_hash>/<filename>`). Identity key persistence on disk.

**tesseras-cli** — A working `tesseras` binary with five commands:
- `init` — generates Ed25519 identity, creates SQLite database
- `create <dir>` — scans a directory for media files, creates a signed tessera
- `verify <hash>` — checks signature and file integrity
- `export <hash> <dest>` — writes a self-contained tessera directory
- `list` — shows a table of stored tesseras

**Testing** — 67+ tests across the workspace: unit tests in every module, property-based tests (proptest) for hex roundtrips and manifest serialization, integration tests covering the full create-verify-export cycle including tampered file and invalid signature detection. Zero clippy warnings.

## Architecture decisions

- **Hexagonal architecture**: crypto operations are injected via trait objects (`Box<dyn Hasher>`, `Box<dyn ManifestSigner>`, `Box<dyn ManifestVerifier>`), keeping the core crate free of concrete crypto dependencies.
- **Feature flags**: the `service` feature on tesseras-core gates the async application layer. The `classical` and `erasure` features on tesseras-crypto control which algorithms are compiled in.
- **Plain-text manifest**: parseable without any binary format library, with explicit `blake3:` hash prefixes and human-readable layout.

## What comes next

Phase 0 is the local-only foundation. The road ahead:

- **Phase 1: Networking** — QUIC transport (quinn), Kademlia DHT for peer discovery, NAT traversal
- **Phase 2: Replication** — Reed-Solomon erasure coding over the network, repair loops, bilateral reciprocity (no blockchain, no tokens)
- **Phase 3: Clients** — Flutter mobile/desktop app via flutter_rust_bridge, GraphQL API, WASM browser node
- **Phase 4: Hardening** — ML-DSA post-quantum signatures, packaging for Alpine/Arch/Debian/FreeBSD/OpenBSD, CI on SourceHut

The tessera format is stable. Everything built from here connects to and extends what exists today.
