# tes create

Create a tessera from a directory of files.

Alias: `tes c`

## Usage

```bash
tes create <PATH> [OPTIONS]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<PATH>` | Directory containing files to include |

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --non-interactive` | Skip prompts | off |
| `--dry-run` | Preview what would be included | off |
| `--visibility <VALUE>` | Visibility level: `public`, `private`, `circle` | `public` |
| `--sealed` | Create a sealed (time-locked) tessera | off |
| `--open-after <DATE>` | Date when sealed tessera opens (YYYY-MM-DD, requires `--sealed`) | none |
| `--language <CODE>` | Language code (e.g., `en`, `pt-BR`) | `en` |
| `--tags <LIST>` | Comma-separated tags | none |
| `--location <DESC>` | Location description | none |
| `--data-dir <PATH>` | Base directory for data storage | `~/.local/share/tesseras` |

## Supported file formats

| Extension | Type | Memory type |
|-----------|------|-------------|
| `.jpg`, `.jpeg` | Image (JPEG) | Moment |
| `.png` | Image (PNG) | Moment |
| `.wav` | Audio (WAV PCM) | Moment |
| `.webm` | Video (WebM) | Moment |
| `.txt` | Plain text (UTF-8) | Reflection |

Files with other extensions are ignored.

## Memory type inference

The command automatically assigns a memory type based on the file format:

- **Text files** (`.txt`) are classified as **Reflection** — thoughts, beliefs, or opinions
- **All other formats** are classified as **Moment** — a photo, recording, or video of something happening

## Visibility levels

| Level | Who can access |
|-------|---------------|
| `public` | Anyone (default) |
| `private` | Only you (and designated heirs) — requires encryption keys |
| `circle` | Explicitly chosen people |
| `sealed` | Opens after a specified date — requires encryption keys and `--sealed --open-after` |

Private and sealed tesseras require encryption keys (X25519 + ML-KEM-768). If they are missing, run `tes init --upgrade` to generate them.

## Examples

### Preview before creating

```bash
tes create ./my-photos --dry-run
```

```
Dry run — files that would be included:
  ./my-photos/beach.jpg (Moment)
  ./my-photos/notes.txt (Reflection)
```

### Create with metadata

```bash
tes create ./vacation-2026 \
    --tags "vacation,summer,beach" \
    --location "Florianópolis, Brazil" \
    --language pt-BR \
    --visibility public
```

```
Created tessera: 9y2m4a1b3e7d8f0cabc1
```

### Non-interactive mode

```bash
tes create ./daily-log --non-interactive --tags "daily"
```

### Create a sealed (time-locked) tessera

```bash
tes create ./time-capsule \
    --sealed \
    --open-after 2050-01-01 \
    --tags "future"
```

The tessera is encrypted and cannot be read until 2050-01-01.

## What happens under the hood

1. Scans the directory for supported files
2. Computes a BLAKE3 hash for each file
3. Assigns a memory type based on file extension
4. Generates a MANIFEST listing all files with their checksums
5. Signs the MANIFEST with your Ed25519 private key
6. For private/sealed tesseras: encrypts memory contents with AES-256-GCM, seals the content key with hybrid encryption (X25519 + ML-KEM-768)
7. Stores the files and metadata in the local database
8. Outputs the content hash in base32 encoding
