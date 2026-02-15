+++
title = "Phase 4: Encryption and Sealed Tesseras"
date = 2026-02-14T16:00:00+00:00
description = "Tesseras now supports private and sealed memories with hybrid post-quantum encryption — AES-256-GCM, X25519 + ML-KEM-768, and time-lock key publication."
+++

Some memories are not meant for everyone. A private journal, a letter to be opened in 2050, a family secret sealed until the grandchildren are old enough. Until now, every tessera on the network was open. Phase 4 changes that: Tesseras now encrypts private and sealed content with a hybrid cryptographic scheme designed to resist both classical and quantum attacks.

The principle remains the same — encrypt as little as possible. Public memories need availability, not secrecy. But when someone creates a private or sealed tessera, the content is now locked behind AES-256-GCM encryption with keys protected by a hybrid key encapsulation mechanism combining X25519 and ML-KEM-768. Both algorithms must be broken to access the content.

## What was built

**AES-256-GCM encryptor** (`tesseras-crypto/src/encryption.rs`) — Symmetric content encryption with random 12-byte nonces and authenticated associated data (AAD). The AAD binds ciphertext to its context: for private tesseras, the content hash is included; for sealed tesseras, both the content hash and the `open_after` timestamp are bound into the AAD. This means moving ciphertext between tesseras with different open dates causes decryption failure — you cannot trick the system into opening a sealed memory early by swapping its ciphertext into a tessera with an earlier seal date.

**Hybrid Key Encapsulation Mechanism** (`tesseras-crypto/src/kem.rs`) — Key exchange using X25519 (classical elliptic curve Diffie-Hellman) combined with ML-KEM-768 (the NIST-standardized post-quantum lattice-based KEM, formerly Kyber). Both shared secrets are combined via `blake3::derive_key` with a fixed context string ("tesseras hybrid kem v1") to produce a single 256-bit content encryption key. This follows the same "dual from day one" philosophy as the project's dual signing (Ed25519 + ML-DSA): if either algorithm is broken in the future, the other still protects the content.

**Sealed Key Envelope** (`tesseras-crypto/src/sealed.rs`) — Wraps a content encryption key using the hybrid KEM, so only the tessera owner can recover it. The KEM produces a transport key, which is XORed with the content key to produce a wrapped key stored alongside the KEM ciphertext. On unsealing, the owner decapsulates the KEM ciphertext to recover the transport key, then XORs again to recover the content key.

**Key Publication** (`tesseras-crypto/src/sealed.rs`) — A standalone signed artifact for publishing a sealed tessera's content key after its `open_after` date has passed. The owner signs the content key, tessera hash, and publication timestamp with their dual keys (Ed25519, with ML-DSA placeholder). The manifest stays immutable — the key publication is a separate document. Other nodes verify the signature against the owner's public key before using the published key to decrypt the content.

**EncryptionContext** (`tesseras-core/src/enums.rs`) — A domain type that represents the AAD context for encryption. It lives in tesseras-core rather than tesseras-crypto because it's a domain concept (not a crypto implementation detail). The `to_aad_bytes()` method produces deterministic serialization: a tag byte (0x00 for Private, 0x01 for Sealed), followed by the content hash, and for Sealed, the `open_after` timestamp as little-endian i64.

**Domain validation** (`tesseras-core/src/service.rs`) — `TesseraService::create()` now rejects Sealed and Private tesseras that don't provide encryption keys. This is a domain-level validation: the service layer enforces that you cannot create a sealed memory without the cryptographic machinery to protect it. The error message is clear: "missing encryption keys for visibility sealed until 2050-01-01."

**Core type updates** — `TesseraIdentity` now includes an optional `encryption_public: Option<HybridEncryptionPublic>` field containing both the X25519 and ML-KEM-768 public keys. `KeyAlgorithm` gained `X25519` and `MlKem768` variants. The identity filesystem layout now supports `node.x25519.key`/`.pub` and `node.mlkem768.key`/`.pub`.

**Testing** — 8 unit tests for AES-256-GCM (roundtrip, wrong key, tampered ciphertext, wrong AAD, cross-context decryption failure, unique nonces, plus 2 property-based tests for arbitrary payloads and nonce uniqueness). 5 unit tests for HybridKem (roundtrip, wrong keypair, tampered X25519, KDF determinism, plus 1 property-based test). 4 unit tests for SealedKeyEnvelope and KeyPublication. 2 integration tests covering the complete sealed and private tessera lifecycle: generate keys, create content key, encrypt, seal, unseal, decrypt, publish key, and verify — the full cycle.

## Architecture decisions

- **Hybrid KEM from day one**: X25519 + ML-KEM-768 follows the same philosophy as dual signing. We don't know which cryptographic assumptions will hold over millennia, so we combine classical and post-quantum algorithms. The cost is ~1.2 KB of additional key material per identity — trivial compared to the photos and videos in a tessera.
- **BLAKE3 for KDF**: rather than adding `hkdf` + `sha2` as new dependencies, we use `blake3::derive_key` with a fixed context string. BLAKE3's key derivation mode is specifically designed for this use case, and the project already depends on BLAKE3 for content hashing.
- **Immutable manifests**: when a sealed tessera's `open_after` date passes, the content key is published as a separate signed artifact (`KeyPublication`), not by modifying the manifest. This preserves the append-only, content-addressed nature of tesseras. The manifest was signed at creation time and never changes.
- **AAD binding prevents ciphertext swapping**: the `EncryptionContext` binds both the content hash and (for sealed tesseras) the `open_after` timestamp into the AES-GCM authenticated data. An attacker who copies encrypted content from a "sealed until 2050" tessera into a "sealed until 2025" tessera will find that decryption fails — the AAD no longer matches.
- **XOR key wrapping**: the sealed key envelope uses a simple XOR of the content key with the KEM-derived transport key, rather than an additional layer of AES-GCM. Since the transport key is a fresh random value from the KEM and is used exactly once, XOR is information-theoretically secure for this specific use case and avoids unnecessary complexity.
- **Domain validation, not storage validation**: the "missing encryption keys" check lives in `TesseraService::create()`, not in the storage layer. This follows the hexagonal architecture pattern: domain rules are enforced at the service boundary, not scattered across adapters.

## What comes next

- **Phase 4 continued: Resilience and Scale** — Shamir's Secret Sharing for heir key distribution, advanced NAT traversal (STUN/TURN), performance tuning, security audits, OS packaging
- **Phase 5: Exploration and Culture** — Public tessera browser by era/location/theme/language, institutional curation, genealogy integration, physical media export (M-DISC, microfilm, acid-free paper with QR)

Sealed tesseras make Tesseras a true time capsule. A father can now record a message for his unborn grandchild, seal it until 2060, and know that the cryptographic envelope will hold — even if the quantum computers of the future try to break it open early.
