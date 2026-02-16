# tes show

Show detailed information about a tessera.

## Usage

```bash
tes show <HASH>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<HASH>` | Tessera hash or prefix (base32 or hex) |

You can use the full hash or a short prefix. Both base32 and hex formats are accepted.

## Options

| Option | Description |
|--------|-------------|
| `--json` | Output as JSON |
| `--data-dir <PATH>` | Base directory for data storage (default: `~/.local/share/tesseras`) |

## Output

The command displays:

- **Tessera hash** — full base32-encoded content hash
- **Created** — creation timestamp (UTC)
- **Visibility** — public, private, circle, or sealed
- **Language** — language code (e.g., `en`, `pt-BR`)
- **Tags** — comma-separated tags (if any)
- **Location** — location description (if any)
- **Files** — list of all files with filename, memory type, and size
- **Total size** — combined size of all files
- **Signature** — Ed25519 signature status (valid or INVALID)

## Examples

### Show tessera details

```bash
tes show 9y2m4a
```

```
Tessera: 9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
Created:    2026-02-14 15:30:00 UTC
Visibility: public
Language:   en
Tags:       family, sunday
Location:   Home

Files (3):
  media.jpg        Moment        128 KB
  media.jpg        Moment        144 KB
  media.txt        Reflection      1 KB

Total size: 273 KB
Signature:  valid
```

### JSON output

```bash
tes show 9y2m4a --json
```

```json
{
  "hash": "9y2m4a1b3e7d8f0cabc123def456789012345678abcdef",
  "created_at": "2026-02-14T15:30:00+00:00",
  "visibility": "public",
  "memory_count": 3,
  "size_bytes": 279552,
  "total_file_size": 279552,
  "signature_valid": true,
  "language": "en",
  "tags": ["family", "sunday"],
  "location": "Home",
  "files": [
    {
      "path": "memories/a1b2c3d4/media.jpg",
      "mime_type": "image/jpeg",
      "size": 131072,
      "hash": "..."
    }
  ],
  "memories": [
    {
      "hash": "...",
      "memory_type": "Moment",
      "media_path": "memories/a1b2c3d4/media.jpg"
    }
  ]
}
```

## Use cases

- **Inspect before sharing** — review all metadata, tags, and files before publishing
- **Scripting** — use `--json` to pipe tessera details into other tools
- **Audit** — check signature status and file integrity at a glance
