# tes status

Show the replication status of a tessera on the network.

## Usage

```bash
tes status <HASH>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<HASH>` | Tessera content hash or prefix |

Hash prefixes are supported, just like `tes publish`.

## Options

| Option | Description |
|--------|-------------|
| `--socket <PATH>` | Path to daemon Unix socket |
| `--data-dir <PATH>` | Base directory for data storage |

## Prerequisites

The tesseras daemon must be running:

```bash
tesd
```

The tessera must exist locally.

## Replication states

| State | Meaning |
|-------|---------|
| `Local (not published)` | Tessera exists only on your machine — not yet published |
| `Publishing...` | Fragments are being distributed, critical redundancy level |
| `Replicated` | Fragments are distributed but below target redundancy |
| `Healthy` | All fragments placed, full redundancy achieved |

## Examples

### Check status of a tessera

```bash
tes status 9f2c4a1b
```

```
Tessera:     9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
State:       Healthy
Fragments:   24/24 placed
Peers:       0 holding copies
```

### Unpublished tessera

```bash
tes status a1b2
```

```
Tessera:     a1b2c3d4e5f6a7b89012345678abcdef0123456789abcdef0123456789abcdef
State:       Local (not published)
Fragments:   0/0 placed
Peers:       0 holding copies
```

## What happens under the hood

1. Resolves the hash prefix against the local database
2. Connects to the daemon via Unix socket
3. The daemon checks that the tessera exists in local storage
4. Queries the replication engine for fragment health:
   - **Healthy**: all fragments alive and placed at target redundancy
   - **Degraded** (shown as Replicated): some fragments missing but above critical threshold
   - **Critical** (shown as Publishing): below minimum redundancy, active repair needed
5. If no fragments exist at all, the state is `Local`
6. Returns the state, fragment counts, and peer count
