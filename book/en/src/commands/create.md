# tes create

Create a tessera from a directory of files.

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
| `--language <CODE>` | Language code (e.g., `en`, `pt-BR`) | `en` |
| `--tags <LIST>` | Comma-separated tags | none |
| `--location <DESC>` | Location description | none |
| `--data-dir <PATH>` | Base directory for data storage | `~/.tesseras` |

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

## Examples

### Preview before creating

```bash
tes create ./my-photos --dry-run
```

### Create with metadata

```bash
tes create ./vacation-2026 \
    --tags "vacation,summer,beach" \
    --location "Florianópolis, Brazil" \
    --language pt-BR \
    --visibility public
```

### Non-interactive mode

```bash
tes create ./daily-log --non-interactive --tags "daily"
```

## Visibility levels

| Level | Who can access |
|-------|---------------|
| `public` | Anyone (default) |
| `private` | Only you (and designated heirs) |
| `circle` | Explicitly chosen people |

## What happens under the hood

1. Scans the directory for supported files
2. Computes a BLAKE3 hash for each file
3. Assigns a memory type based on file extension
4. Generates a MANIFEST listing all files with their checksums
5. Signs the MANIFEST with your Ed25519 private key
6. Stores the files and metadata in the local database
7. Outputs the content hash that uniquely identifies this tessera
