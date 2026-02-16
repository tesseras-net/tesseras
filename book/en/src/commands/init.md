# tes init

Initialize identity and local database.

## Usage

```bash
tes init [OPTIONS]
```

## Description

Sets up your local Tesseras environment. This is the first command you should run after installing Tesseras.

The command creates:

| Path | Contents |
|------|----------|
| `identity/node.ed25519.pub` | Ed25519 public key (your identity) |
| `identity/node.ed25519.key` | Ed25519 secret key (signing) |
| `identity/node.x25519.pub` | X25519 public key (key exchange) |
| `identity/node.x25519.key` | X25519 secret key (encryption) |
| `identity/node.mlkem768.pub` | ML-KEM-768 public key (post-quantum key encapsulation) |
| `identity/node.mlkem768.key` | ML-KEM-768 secret key (post-quantum encryption) |
| `db/tesseras.db` | SQLite database for indexing |
| `blobs/` | Blob storage for memory files |
| `config.toml` | Configuration file |

All paths are relative to the data directory (default: `~/.local/share/tesseras`).

## Options

| Option | Description |
|--------|-------------|
| `--upgrade` | Add missing encryption keys to an existing identity |
| `--data-dir <PATH>` | Base directory for data storage (default: `~/.local/share/tesseras`) |

## Key types

**Ed25519** — Classical elliptic-curve signature key. Used to sign every tessera's MANIFEST, proving authorship. This is your primary identity on the network.

**X25519** — Diffie-Hellman key exchange, derived from Curve25519. Used to encrypt private and sealed tesseras so only you (and your heirs) can read them.

**ML-KEM-768** — Post-quantum key encapsulation mechanism (formerly CRYSTALS-Kyber). Paired with X25519 in a hybrid scheme so that your encrypted tesseras remain secure even if large-scale quantum computers are built in the future.

## Idempotent

Running `init` again is safe. Existing keys are preserved:

```bash
tes init
```

```
Ed25519 identity already exists
Encryption keys already exist
Database initialized
Tesseras initialized at /home/user/.local/share/tesseras
```

## Upgrade existing identity

If you initialized before encryption keys were available, use `--upgrade` to add them without touching your Ed25519 identity:

```bash
tes init --upgrade
```

```
Generating encryption keypair (X25519 + ML-KEM-768)...
Generated encryption keypair
Tesseras initialized at /home/user/.local/share/tesseras
```

The upgrade is atomic — if the ML-KEM-768 key fails to save, the X25519 key is rolled back so you never end up with a partial encryption identity.

## Custom data directory

```bash
tes --data-dir /mnt/usb/tesseras init
```

This creates the full directory structure under `/mnt/usb/tesseras/` instead of the default location.

## Legacy data migration

If existing data is found at `~/.tesseras` (the previous default location) while the current data directory is different, a hint is printed:

```
Note: found existing data at /home/user/.tesseras. Consider moving it to /home/user/.local/share/tesseras
```

## What happens under the hood

1. Creates the directory structure (`identity/`, `db/`, `blobs/`)
2. Generates an Ed25519 keypair (private key stays local, public key identifies you)
3. Generates a hybrid encryption keypair (X25519 + ML-KEM-768) atomically
4. Runs SQLite migrations to set up the database schema (WAL mode)
5. Writes a default `config.toml`
