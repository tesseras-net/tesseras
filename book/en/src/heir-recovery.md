# Heir Key Recovery

Your tesseras can survive infrastructure failures, quantum computers, and centuries of time. But what happens when you can no longer access your own keys? Tesseras uses **Shamir's Secret Sharing** to let you distribute your cryptographic identity to trusted heirs.

## How it works

Shamir's Secret Sharing splits a secret into N shares with a threshold T. Any T shares can reconstruct the original secret. Fewer than T shares reveal **nothing** — this is information-theoretically secure, not just computationally hard to break.

For example, with threshold 2 and 3 total shares:
- Give share 1 to your spouse
- Give share 2 to your sibling
- Give share 3 to your lawyer

Any two of them can recover your identity. A single share alone is useless.

## Creating heir shares

```bash
tes heir create --threshold 2 --shares 3
```

This splits your Ed25519 identity key into 3 shares (requiring 2 to reconstruct) and saves them to `./heir-shares/`:

```
heir-shares/
├── heir_share_1.bin   # MessagePack binary
├── heir_share_1.txt   # Human-readable base64 text
├── heir_share_2.bin
├── heir_share_2.txt
├── heir_share_3.bin
└── heir_share_3.txt
```

Each share is generated in two formats:
- **Binary** (`.bin`) — compact MessagePack, suitable for USB drives or digital storage
- **Text** (`.txt`) — base64 with human-readable header, suitable for printing on paper

The text format looks like this:

```
--- TESSERAS HEIR SHARE ---
Format: v1
Owner: a1b2c3d4e5f6a7b8 (fingerprint)
Share: 1 of 3 (threshold: 2)
Session: 9f8e7d6c5b4a3210
Created: 2026-02-15

<base64-encoded data>
--- END HEIR SHARE ---
```

## Reconstructing from shares

When heirs need to recover the identity:

```bash
tes heir reconstruct heir_share_1.txt heir_share_2.bin --output-dir ./recovered-keys
```

The command auto-detects whether each file is binary or text format. It validates that all shares belong to the same session and owner, verifies checksums, and reconstructs the Ed25519 keypair.

To install the recovered keys as the active identity:

```bash
tes heir reconstruct share1.txt share2.txt --output-dir ./recovered --install
```

This backs up the current identity before replacing it.

## Inspecting a share

To view metadata about a share without exposing secret data:

```bash
tes heir info heir_share_1.txt
```

Output:
```
Heir Share Information:
  Format version: 1
  Share: 1 of 3 (threshold: 2)
  Session: 9f8e7d6c5b4a3210
  Owner fingerprint: a1b2c3d4e5f6a7b8
  Share data size: 34 bytes
  Checksum: valid
```

## Security considerations

- **Threshold choice**: a threshold of 2-of-3 or 3-of-5 is recommended for most people. Higher thresholds are more secure but require more heirs to cooperate.
- **Physical storage**: print the `.txt` files on acid-free paper and store in separate physical locations (safe deposit boxes, different homes). Paper survives decades without degradation.
- **Never store shares together**: the entire point of splitting is distribution. Keeping all shares in one place defeats the purpose.
- **Session isolation**: each `heir create` call generates a fresh session ID. Shares from different sessions cannot be mixed — this prevents confusion after key rotations.
- **Checksum verification**: each share includes a BLAKE3 checksum. Corrupted shares (OCR errors, bit rot) are detected before reconstruction is attempted.
- **Re-split after key changes**: if you regenerate your identity, create new heir shares and securely destroy the old ones.

## Design principles

- **Information-theoretic security** — T-1 shares reveal exactly zero information about the secret. This is not a computational assumption; it is mathematically proven.
- **Corruption detection** — BLAKE3 checksums catch bit rot, OCR errors, and truncation before any reconstruction attempt.
- **Format resilience** — dual output (binary + text) ensures shares survive different storage media failure modes.
- **Backward compatibility** — the secret blob is versioned, so future versions can include additional key material without breaking existing shares.
