+++
title = "Phase 4: Heir Key Recovery with Shamir's Secret Sharing"
date = 2026-02-15
description = "Tesseras now lets you split your cryptographic identity into shares distributed to trusted heirs — any threshold of them can reconstruct your keys, but fewer reveal nothing."
+++

What happens to your memories when you die? Until now, Tesseras could preserve content across millennia — but the private and sealed keys died with their owner. Phase 4 continues with a solution: Shamir's Secret Sharing, a cryptographic scheme that lets you split your identity into shares and distribute them to the people you trust most.

The math is elegant: you choose a threshold T and a total N. Any T shares reconstruct the full secret; T-1 shares reveal absolutely nothing. This is not "almost nothing" — it is information-theoretically secure. An attacker with one fewer share than the threshold has exactly zero bits of information about the secret, no matter how much computing power they have.

## What was built

**GF(256) finite field arithmetic** (`tesseras-crypto/src/shamir/gf256.rs`) — Shamir's Secret Sharing requires arithmetic in a finite field. We implement GF(256) using the same irreducible polynomial as AES (x^8 + x^4 + x^3 + x + 1), with compile-time lookup tables for logarithm and exponentiation. All operations are constant-time via table lookups — no branches on secret data. The module includes Horner's method for polynomial evaluation and Lagrange interpolation at x=0 for secret recovery. 233 lines, exhaustively tested: all 256 elements for identity/inverse properties, commutativity, and associativity.

**ShamirSplitter** (`tesseras-crypto/src/shamir/mod.rs`) — The core split/reconstruct API. `split()` takes a secret byte slice, a configuration (threshold T, total N), and the owner's Ed25519 public key. For each byte of the secret, it constructs a random polynomial of degree T-1 over GF(256) with the secret byte as the constant term, then evaluates it at N distinct points. `reconstruct()` takes T or more shares and recovers the secret via Lagrange interpolation. Both operations include extensive validation: threshold bounds, session consistency, owner fingerprint matching, and BLAKE3 checksum verification.

**HeirShare format** — Each share is a self-contained, serializable artifact with:
- Format version (v1) for forward compatibility
- Share index (1..N) and threshold/total metadata
- Session ID (random 8 bytes) — prevents mixing shares from different split sessions
- Owner fingerprint (first 8 bytes of BLAKE3 hash of the Ed25519 public key)
- Share data (the Shamir y-values, same length as the secret)
- BLAKE3 checksum over all preceding fields

Shares are serialized in two formats: **MessagePack** (compact binary, for programmatic use) and **base64 text** (human-readable, for printing and physical storage). The text format includes a header with metadata and delimiters:

```
--- TESSERAS HEIR SHARE ---
Format: v1
Owner: a1b2c3d4e5f6a7b8 (fingerprint)
Share: 1 of 3 (threshold: 2)
Session: 9f8e7d6c5b4a3210
Created: 2026-02-15

<base64-encoded MessagePack data>
--- END HEIR SHARE ---
```

This format is designed to be printed on paper, stored in a safe deposit box, or engraved on metal. The header is informational — only the base64 payload is parsed during reconstruction.

**CLI integration** (`tesseras-cli/src/commands/heir.rs`) — Three new subcommands:

- `tes heir create` — splits your Ed25519 identity into heir shares. Prompts for confirmation (your full identity is at stake), generates both `.bin` and `.txt` files for each share, and writes `heir_meta.json` to your identity directory.
- `tes heir reconstruct` — loads share files (auto-detects binary vs text format), validates consistency, reconstructs the secret, derives the Ed25519 keypair, and optionally installs it to `~/.tesseras/identity/` (with automatic backup of the existing identity).
- `tes heir info` — displays share metadata and verifies the checksum without exposing any secret material.

**Secret blob format** — Identity keys are serialized into a versioned blob before splitting: a version byte (0x01), a flags byte (0x00 for Ed25519-only), followed by the 32-byte Ed25519 secret key. This leaves room for future expansion when X25519 and ML-KEM-768 private keys are integrated into the heir share system.

**Testing** — 20 unit tests for ShamirSplitter (roundtrip, all share combinations, insufficient shares, wrong owner, wrong session, threshold-1 boundary, large secrets up to ML-KEM-768 key size). 7 unit tests for GF(256) arithmetic (exhaustive field properties). 3 property-based tests with proptest (arbitrary secrets up to 5000 bytes, arbitrary T-of-N configurations, information-theoretic security verification). Serialization roundtrip tests for both MessagePack and base64 text formats. 2 integration tests covering the complete heir lifecycle: generate identity, split into shares, serialize, deserialize, reconstruct, verify keypair, and sign/verify with reconstructed keys.

## Architecture decisions

- **GF(256) over GF(prime)**: we use GF(256) rather than a prime field because it maps naturally to bytes — each element is a single byte, each share is the same length as the secret. No big-integer arithmetic, no modular reduction, no padding. This is the same approach used by most real-world Shamir implementations including SSSS and Hashicorp Vault.
- **Compile-time lookup tables**: the LOG and EXP tables for GF(256) are computed at compile time using `const fn`. This means zero runtime initialization cost and constant-time operations via table lookups rather than loops.
- **Session ID prevents cross-session mixing**: each call to `split()` generates a fresh random session ID. If an heir accidentally uses shares from two different split sessions (e.g., before and after a key rotation), reconstruction fails cleanly with a validation error rather than producing garbage output.
- **BLAKE3 checksums detect corruption**: each share includes a BLAKE3 checksum over its contents. This catches bit rot, transmission errors, and accidental truncation before any reconstruction attempt. A share printed on paper and scanned back via OCR will fail the checksum if a single character is wrong.
- **Owner fingerprint for identification**: shares include the first 8 bytes of BLAKE3(Ed25519 public key) as a fingerprint. This lets heirs verify which identity a share belongs to without revealing the full public key. During reconstruction, the fingerprint is cross-checked against the recovered key.
- **Dual format for resilience**: both binary (MessagePack) and text (base64) formats are generated because physical media has different failure modes than digital storage. A USB drive might fail; paper survives. A QR code might be unreadable; base64 text can be manually typed.
- **Blob versioning**: the secret is wrapped in a versioned blob (version + flags + key material) so future versions can include additional keys (X25519, ML-KEM-768) without breaking backward compatibility with existing shares.

## What comes next

- **Phase 4 continued: Resilience and Scale** — advanced NAT traversal (STUN/TURN), performance tuning (connection pooling, fragment caching, SQLite WAL), security audits, institutional node onboarding, OS packaging
- **Phase 5: Exploration and Culture** — public tessera browser by era/location/theme/language, institutional curation, genealogy integration, physical media export (M-DISC, microfilm, acid-free paper with QR)

With Shamir's Secret Sharing, Tesseras closes the last critical gap in long-term preservation. Your memories survive infrastructure failures through erasure coding. Your privacy survives quantum computers through hybrid encryption. And now, your identity survives you — passed on to the people you chose, requiring their cooperation to unlock what you left behind.
