# Quick Start

This tutorial walks you through a complete workflow: creating an identity, building a tessera from files, verifying it, and exporting it.

## 1. Initialize your identity

First, set up your local identity and database:

```bash
tes init
```

```
Generated Ed25519 identity
Database initialized
Config written to /home/user/.tesseras/config.toml
Tesseras initialized at /home/user/.tesseras
```

This creates:

- `~/.tesseras/identity/` — your Ed25519 keypair
- `~/.tesseras/db/` — SQLite database for indexing
- `~/.tesseras/blobs/` — storage for memory files
- `~/.tesseras/config.toml` — configuration file

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

## 4. Create a tessera

```bash
tes create my-memories --tags "family,sunday" --location "Home"
```

The output includes the content hash — a 64-character hex string that uniquely identifies your tessera. Copy it for the next steps.

## 5. List your tesseras

```bash
tes list
```

```
Hash             Created     Memories  Size    Visibility
9f2c4a1b3e7d8f0c 2026-02-14         3  284 KB  public
```

## 6. Verify integrity

Use the content hash to verify that all files are intact and the signature is valid:

```bash
tes verify 9f2c4a1b3e7d8f0c...
```

```
Tessera: 9f2c4a1b3e7d8f0c...
Signature: VALID
  [OK] memories/a1b2c3/media.jpg
  [OK] memories/d4e5f6/media.jpg
  [OK] memories/g7h8i9/media.txt
Verification: PASSED
```

## 7. Export a self-contained copy

Export the tessera to a directory that can be read without Tesseras:

```bash
tes export 9f2c4a1b3e7d8f0c... ./backup
```

```
Exported to ./backup/tessera-9f2c4a1b3e7d8f0c...
```

## 8. Inspect the export

The exported directory is fully self-contained:

```
tessera-9f2c4a1b3e7d8f0c.../
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
└── decode/
    ├── formats.txt             # Explanation of all formats used
    ├── jpeg.txt                # How to decode JPEG
    └── json.txt                # How to decode JSON
```

Everything a future reader needs to understand the contents is included in the directory itself — no Tesseras software required.
