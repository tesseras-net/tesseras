# Encryption and Sealed Tesseras

Most tesseras are public — designed to be accessible to anyone, forever. But some memories need privacy. Tesseras supports two encrypted visibility modes:

- **Private** — only the creator (and their heirs) can ever access the content
- **Sealed** — the content is time-locked and becomes accessible after a specific date

Public tesseras are never encrypted. Availability is more important than secrecy for preservation.

## How encryption works

When you create a private or sealed tessera, the following happens:

1. A random **content key** (256-bit) is generated
2. Each memory file is encrypted with **AES-256-GCM** using that content key
3. The content key is wrapped in a **sealed key envelope** using your encryption public key
4. The wrapped key is stored alongside the encrypted content

Only the holder of the corresponding private key can unwrap the content key and decrypt the content.

## Hybrid post-quantum key encapsulation

The sealed key envelope uses a **hybrid Key Encapsulation Mechanism (KEM)** combining two algorithms:

- **X25519** — a well-tested classical elliptic curve key exchange
- **ML-KEM-768** — a NIST-standardized post-quantum lattice-based KEM (formerly Kyber)

Both algorithms produce shared secrets that are combined using BLAKE3 key derivation. An attacker must break **both** algorithms to recover the content key. This follows the same principle as Tesseras' dual signatures (Ed25519 + ML-DSA): we don't know which cryptographic assumptions will hold over centuries, so we hedge our bets.

## Authenticated associated data (AAD)

AES-256-GCM supports authenticated associated data — extra information that is verified during decryption but not encrypted. Tesseras binds the following into the AAD:

- The **content hash** of the tessera (always)
- The **open_after timestamp** (for sealed tesseras only)

This prevents **ciphertext swapping attacks**: an attacker cannot copy encrypted content from one tessera to another, because the AAD will not match and decryption will fail. For sealed tesseras, this also means you cannot change the seal date — the timestamp is cryptographically bound to the ciphertext.

## Sealed tesseras: time capsules

A sealed tessera is a true time capsule. When you create one, you specify an `open_after` date. The content is encrypted and the key is sealed in an envelope that only you can open.

When the `open_after` date passes, the owner publishes the content key as a signed **Key Publication** — a standalone artifact containing the key, the tessera hash, and the owner's signature. Other nodes can verify the signature and use the published key to decrypt the content.

The tessera's manifest is never modified. The Key Publication is a separate document, preserving the immutable, content-addressed nature of tesseras.

## What about the keys?

Each identity now includes an **encryption keypair** alongside the signing keypair:

| Key type | Algorithm | Purpose |
|----------|-----------|---------|
| Ed25519 | Classical | Signing manifests and key publications |
| ML-DSA | Post-quantum | Signing (when enabled) |
| X25519 | Classical | Key encapsulation (encryption) |
| ML-KEM-768 | Post-quantum | Key encapsulation (encryption) |

The encryption keypair is generated when the identity is created. The public half is stored in the tessera's identity directory; the private half stays on the owner's device.

## Design principles

- **Encrypt as little as possible** — only private and sealed content is encrypted. Public memories stay open for long-term accessibility.
- **Dual algorithms from day one** — both classical and post-quantum cryptography, so content is protected even if one algorithm is broken.
- **Immutable manifests** — keys are published separately, never by modifying existing data.
- **Fail closed** — the system rejects attempts to create private or sealed tesseras without encryption keys.
