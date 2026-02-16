# tes export

Export a tessera to a self-contained directory.

Alias: `tes e`

## Usage

```bash
tes export <HASH> <DEST>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<HASH>` | Tessera hash or prefix (base32 or hex) |
| `<DEST>` | Destination directory |

You can use the full hash or a short prefix. Both base32 and hex formats are accepted.

## Options

| Option | Description |
|--------|-------------|
| `--data-dir <PATH>` | Base directory for data storage (default: `~/.local/share/tesseras`) |

## Examples

### Export to a backup directory

```bash
tes export 9y2m4a ./backup
```

```
Exported to ./backup/tessera-9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
```

### Export to USB drive

```bash
tes export 9y2m4a /mnt/usb/tesseras
```

## Exported directory structure

The exported directory is fully self-contained — readable without Tesseras software:

```
tessera-9y2m4a1b.../
├── MANIFEST                    # Plain text index with checksums
├── README.decode               # How to read this tessera without software
├── identity/
│   ├── creator.pub.ed25519     # Creator's public key
│   └── signature.ed25519.sig   # Signature of the MANIFEST
├── memories/
│   ├── <hash>/
│   │   ├── media.jpg           # The photo/audio/video/text
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

Everything a future reader needs to understand the contents is included — no Tesseras software required.

## Key feature: self-contained

The exported directory is designed to be readable **without Tesseras software**. It includes:

- **MANIFEST** — a plain-text file listing every file with its BLAKE3 checksum, readable by any text editor
- **README.decode** — human-readable instructions for understanding the contents
- **decode/** — detailed explanations of every file format used (JPEG, WAV, JSON, UTF-8)

This means someone thousands of years from now, with no knowledge of Tesseras, can still understand and access the memories.

## Use cases

- **Offline backup** — copy to USB drives, external hard disks, or NAS
- **Archival media** — burn to M-DISC, write to tape, or print QR codes
- **Sharing** — send a self-contained copy to someone without Tesseras
- **Migration** — move tesseras between systems without using the network
