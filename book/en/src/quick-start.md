# Quick Start

This tutorial walks you through a complete workflow: creating an identity, building a tessera from files, verifying it, exporting it, and publishing it to the network.

## 1. Initialize your identity

First, set up your local identity and database:

```bash
tes init
```

```
Generated Ed25519 identity
Generating encryption keypair (X25519 + ML-KEM-768)...
Generated encryption keypair
Database initialized
Config written to /home/user/.local/share/tesseras/config.toml
Tesseras initialized at /home/user/.local/share/tesseras
```

This creates your cryptographic identity (Ed25519 for signing, X25519 + ML-KEM-768 for encryption), a SQLite database, blob storage, and a default config file under `~/.local/share/tesseras/`.

## 2. Prepare your files

Create a directory with the memories you want to preserve:

```bash
mkdir my-memories
cp ~/photos/family-dinner.jpg my-memories/
cp ~/photos/garden.jpg my-memories/
echo "A warm Sunday afternoon with the family." > my-memories/reflection.txt
```

Supported formats: `.jpg`, `.jpeg`, `.png` (images), `.wav` (audio), `.webm` (video), `.txt` (text).

## 3. Preview with dry run

See what would be included without creating anything:

```bash
tes create my-memories --dry-run
```

```
Dry run — files that would be included:
  my-memories/family-dinner.jpg (Moment)
  my-memories/garden.jpg (Moment)
  my-memories/reflection.txt (Reflection)
```

## 4. Create a tessera

```bash
tes create my-memories --tags "family,sunday" --location "Home"
```

```
Created tessera: 9y2m4a1b3e7d8f0cabc1
```

The output is the content hash in base32 encoding. You can use it (or a short prefix) in the next steps.

## 5. List your tesseras

```bash
tes ls
```

```
┌────────────┬────────────┬──────────┬────────┬────────────┐
│ Hash       │ Created    │ Memories │ Size   │ Visibility │
├────────────┼────────────┼──────────┼────────┼────────────┤
│ 9y2m4a1b3e │ 2026-02-14 │        3 │ 284 KB │ public     │
└────────────┴────────────┴──────────┴────────┴────────────┘
```

## 6. Verify integrity

Use the hash (or a short prefix) to verify that all files are intact and the signature is valid:

```bash
tes verify 9y2m4a
```

```
Tessera: 9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
Signature: VALID
  [OK] memories/a1b2c3/media.jpg
  [OK] memories/d4e5f6/media.jpg
  [OK] memories/g7h8i9/media.txt
Verification: PASSED
```

## 7. Export a self-contained copy

Export the tessera to a directory that can be read without Tesseras:

```bash
tes export 9y2m4a ./backup
```

```
Exported to ./backup/tessera-9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
```

The exported directory is fully self-contained:

```
tessera-9y2m4a1b.../
├── MANIFEST                    # Plain text index with checksums
├── README.decode               # How to read this tessera without software
├── identity/
│   ├── creator.pub.ed25519     # Your public key
│   └── signature.ed25519.sig   # Signature of the MANIFEST
├── memories/
│   ├── <hash>/
│   │   ├── media.jpg           # The photo
│   │   ├── context.txt         # Description in plain text
│   │   └── meta.json           # Structured metadata
│   └── .../
├── schema/
│   └── v1.json                 # JSON schema for metadata validation
└── decode/
    ├── formats.txt             # Explanation of all formats used
    ├── jpeg.txt                # How to decode JPEG
    └── json.txt                # How to decode JSON
```

Everything a future reader needs to understand the contents is included in the directory itself — no Tesseras software required.

## 8. Publish to the network

With the daemon running, publish your tessera for replication across the P2P network:

```bash
tes publish 9y2m4a
```

```
Published tessera 9y2m4a1b (24 fragments created)
Distribution in progress — use `tes status 9y2m4a1b` to track.
```

## 9. Check replication status

Monitor how your tessera is being distributed:

```bash
tes status 9y2m4a
```

```
Tessera:     9y2m4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef
State:       Healthy
Fragments:   24/24 placed
Peers:       0 holding copies
```

## Global options

These flags work with every command:

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Verbose output (`-vv` for very verbose) |
| `-q, --quiet` | Suppress all log messages |
| `--color <VALUE>` | Coloring: `auto`, `always`, `never` |
| `--data-dir <PATH>` | Base directory for data storage |
| `--socket <PATH>` | Path to daemon Unix socket (for `publish`, `fetch`, `status`) |
