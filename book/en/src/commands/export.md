# tes export

Export a tessera as a self-contained directory.

## Usage

```bash
tes export <HASH> <DEST>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<HASH>` | Tessera content hash (64 hex characters) |
| `<DEST>` | Destination directory |

## Options

| Option | Description |
|--------|-------------|
| `--data-dir <PATH>` | Base directory for data storage (default: `~/.tesseras`) |

## Output structure

The export creates a directory named `tessera-<hash>` inside the destination:

```
tessera-9f2c4a1b.../
├── MANIFEST                    # Plain text index with checksums
├── README.decode               # Human-readable decoding instructions
├── identity/
│   ├── creator.pub.ed25519     # Creator's public key
│   └── signature.ed25519.sig   # Signature of the MANIFEST
├── memories/
│   ├── <content-hash>/
│   │   ├── media.jpg           # Primary media file
│   │   ├── context.txt         # Human context in plain UTF-8
│   │   └── meta.json           # Structured metadata
│   └── .../
├── schema/
│   └── v1.json                 # JSON schema for metadata validation
└── decode/
    ├── formats.txt             # Explanation of all formats used
    ├── jpeg.txt                # How to decode JPEG
    ├── wav.txt                 # How to decode WAV
    └── json.txt                # How to decode JSON
```

## Example

```bash
tes export 9f2c4a1b3e7d8f0cabc123def4567890... ./backup
```

```
Exported to ./backup/tessera-9f2c4a1b3e7d8f0cabc123def4567890...
```

## Key feature: self-contained

The exported directory is designed to be readable **without Tesseras software**. It includes:

- **MANIFEST** — a plain-text file listing every file with its BLAKE3 checksum, readable by any text editor
- **README.decode** — human-readable instructions for understanding the contents
- **decode/** — detailed explanations of every file format used (JPEG, WAV, JSON, UTF-8)

This means someone thousands of years from now, with no knowledge of Tesseras, can still understand and access the memories.

## Use cases

- **Backup** — export to an external drive, USB stick, or cloud storage
- **Sharing** — give someone a complete copy of a tessera
- **Archival** — store on write-once media (DVD, Blu-ray, tape)
- **Migration** — move tesseras between machines without needing the database
