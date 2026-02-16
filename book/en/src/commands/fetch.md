# tes fetch

Fetch a tessera from the network and store it locally.

## Usage

```bash
tes fetch <HASH>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<HASH>` | Full tessera content hash (64 hex characters) |

Unlike `publish` and `status`, `fetch` requires the full 64-character hex hash because the tessera does not exist locally yet and cannot be resolved from a prefix.

## Options

| Option | Description |
|--------|-------------|
| `--socket <PATH>` | Path to daemon Unix socket |
| `--data-dir <PATH>` | Base directory for data storage |

## Prerequisites

The tesseras daemon must be running and connected to the network:

```bash
tesseras-daemon
```

## Examples

### Fetch a tessera

```bash
tes fetch 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
```

```
Fetching tessera 9f2c4a1b from network...
Fetched tessera 9f2c4a1b (3 memories, 1.2 MB)
```

### Fetch with custom socket

```bash
tes fetch 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01 \
    --socket /tmp/my-daemon.sock
```

## What happens under the hood

1. Connects to the daemon via Unix socket
2. The daemon looks up fragments for this tessera hash in local storage
3. For small tesseras: the single fragment contains the full data
4. For larger tesseras: collects enough fragments and reconstructs the original data using Reed-Solomon erasure decoding
5. Unpacks the byte buffer into individual files (MANIFEST, memories, blobs)
6. Stores each file into the local content-addressable storage (CAS)
7. Returns the number of memories and total size fetched
