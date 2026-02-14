# tesseras init

Initialize identity and local database.

## Usage

```bash
tesseras init
```

## Description

Sets up your local Tesseras environment. This is the first command you should run after installing Tesseras.

The command creates:

| Path | Contents |
|------|----------|
| `~/.tesseras/identity/` | Ed25519 keypair for signing tesseras |
| `~/.tesseras/db/` | SQLite database for indexing |
| `~/.tesseras/blobs/` | Blob storage for memory files |
| `~/.tesseras/config.toml` | Configuration file |

## Options

| Option | Description |
|--------|-------------|
| `--data-dir <PATH>` | Base directory for data storage (default: `~/.tesseras`) |

## Idempotent

Running `init` again is safe. If an identity already exists, it is preserved:

```bash
tesseras init
```

```
Ed25519 identity already exists
Database initialized
Tesseras initialized at /home/user/.tesseras
```

## Custom data directory

```bash
tesseras --data-dir /mnt/usb/tesseras init
```

This creates the full directory structure under `/mnt/usb/tesseras/` instead of the default location.

## What happens under the hood

1. Creates the directory structure (`identity/`, `db/`, `blobs/`)
2. Generates an Ed25519 keypair (private key stays local, public key identifies you)
3. Runs SQLite migrations to set up the database schema
4. Writes a default `config.toml`
