# tes list

List all local tesseras.

## Usage

```bash
tes list
```

## Options

| Option | Description |
|--------|-------------|
| `--data-dir <PATH>` | Base directory for data storage (default: `~/.tesseras`) |

## Output

Displays a table with the following columns:

| Column | Description |
|--------|-------------|
| **Hash** | First 16 characters of the content hash |
| **Created** | Creation date (YYYY-MM-DD) |
| **Memories** | Number of memories in the tessera |
| **Size** | Total size (B, KB, MB, or GB) |
| **Visibility** | Visibility level (public, private, or circle) |

## Example

```bash
tes list
```

```
Hash             Created     Memories  Size    Visibility
9f2c4a1b3e7d8f0c 2026-02-14         3  284 KB  public
a3b7c2d9e4f01823 2026-02-10         1   12 KB  private
f8e7d6c5b4a39201 2026-01-28        12    4 MB  public
```

## Empty database

If no tesseras have been created yet:

```bash
tes list
```

```
No tesseras found.
```
