# tes verify

Verify integrity of a stored tessera.

## Usage

```bash
tes verify <HASH>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<HASH>` | Tessera content hash (64 hex characters) |

## Options

| Option | Description |
|--------|-------------|
| `--data-dir <PATH>` | Base directory for data storage (default: `~/.tesseras`) |

## What it checks

1. **Signature validity** — verifies the Ed25519 signature over the MANIFEST
2. **File integrity** — recomputes the BLAKE3 hash of every file and compares it against the MANIFEST

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Verification passed — all files intact, signature valid |
| `1` | Verification failed — corrupted files or invalid signature |

## Examples

### Successful verification

```bash
tes verify 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
```

```
Tessera: 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
Signature: VALID
  [OK] memories/a1b2c3d4/media.jpg
  [OK] memories/e5f6a7b8/media.txt
  [OK] memories/c9d0e1f2/media.wav
Verification: PASSED
```

### Failed verification

If a file has been modified or corrupted:

```
Tessera: 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
Signature: VALID
  [OK] memories/a1b2c3d4/media.jpg
  [FAILED] memories/e5f6a7b8/media.txt
  [OK] memories/c9d0e1f2/media.wav
Verification: FAILED
```

## Use cases

- **Routine integrity checks** — periodically verify that your stored tesseras haven't been corrupted
- **After transfer** — verify after copying tesseras to a new device or storage medium
- **Trust verification** — confirm that a tessera received from someone else hasn't been tampered with
