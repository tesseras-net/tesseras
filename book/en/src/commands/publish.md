# tes publish

Publish a tessera to the network for replication and long-term preservation.

## Usage

```bash
tes publish <HASH>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<HASH>` | Tessera content hash or prefix |

Hash prefixes are supported — if your tessera hash starts with `a1b2c3`, you can use `tes publish a1b2c3` and the CLI will resolve the full hash from your local database.

## Options

| Option | Description |
|--------|-------------|
| `--socket <PATH>` | Path to daemon Unix socket |
| `--data-dir <PATH>` | Base directory for data storage |

## Prerequisites

The tesseras daemon must be running:

```bash
tesseras-daemon
```

The tessera must exist locally (created with `tes create`).

## Examples

### Publish by full hash

```bash
tes publish 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
```

```
Published tessera 9f2c4a1b (2 fragments created)
Distribution in progress — use `tes status 9f2c4a1b` to track.
```

### Publish by short prefix

```bash
tes publish 9f2c
```

```
Published tessera 9f2c4a1b (24 fragments created)
Distribution in progress — use `tes status 9f2c4a1b` to track.
```

### Publish with custom socket

```bash
tes publish a1b2 --socket /tmp/my-daemon.sock
```

## What happens under the hood

1. Resolves the hash prefix against the local database to find the full content hash
2. Connects to the daemon via Unix socket
3. The daemon reads all tessera files (MANIFEST, signatures, memories, blobs) from local storage
4. Packs everything into a single byte buffer using MessagePack serialization
5. For small tesseras (< 4 MB): replicates the raw data as a single fragment to r=7 peers
6. For larger tesseras: applies Reed-Solomon erasure coding, producing redundant fragments
7. Distributes fragments across the network via the DHT, preferring peers with positive reciprocity
8. Returns the number of fragments created
