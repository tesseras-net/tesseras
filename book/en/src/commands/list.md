# tes list

List all local tesseras.

Alias: `tes ls`

## Usage

```bash
tes list
```

## Options

| Option | Description |
|--------|-------------|
| `--data-dir <PATH>` | Base directory for data storage (default: `~/.local/share/tesseras`) |

## Examples

### List tesseras

```bash
tes list
```

```
┌────────────┬────────────┬──────────┬────────┬────────────┐
│ Hash       │ Created    │ Memories │ Size   │ Visibility │
├────────────┼────────────┼──────────┼────────┼────────────┤
│ 9y2m4a1b3e │ 2026-02-14 │        3 │ 284 KB │ public     │
│ f7g8h9j0kl │ 2026-02-15 │        1 │  12 KB │ private    │
└────────────┴────────────┴──────────┴────────┴────────────┘
```

The hash column shows the first 10 characters of the base32-encoded content hash. Use this prefix with other commands (e.g., `tes verify 9y2m4a1b3e`).

### Empty database

```bash
tes list
```

```
No tesseras found.
```

## Column reference

| Column | Description |
|--------|-------------|
| Hash | First 10 chars of the base32 content hash |
| Created | Date the tessera was created (YYYY-MM-DD) |
| Memories | Number of memories in the tessera |
| Size | Total size of all files (B, KB, MB, GB) |
| Visibility | Visibility level: public, private, circle, or sealed |
