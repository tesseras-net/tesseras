# Running a Node

The `tes` binary includes a built-in daemon that runs a full Tesseras node participating in the peer-to-peer network. It listens for connections over QUIC, joins the distributed hash table (DHT), and enables other nodes to discover and find tessera pointers.

## Starting the daemon

```bash
tes admin daemon start
```

On first run, the daemon:

1. Creates the data directory (`~/.local/share/tesseras` on Linux, `~/Library/Application Support/tesseras` on macOS)
2. Generates a node identity with proof-of-work (takes about 1 second)
3. Binds a QUIC listener on `0.0.0.0:4433`
4. Bootstraps into the network by contacting seed nodes
5. Prints `daemon ready` when fully operational

## Command-line options

```
tes admin daemon start [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config <PATH>` | Path to a TOML config file | None (uses built-in defaults) |
| `-l, --listen <ADDR>` | Address and port to listen on | `0.0.0.0:4433` |
| `-b, --bootstrap <ADDRS>` | Comma-separated bootstrap addresses | `bootstrap1.tesseras.net:4433,bootstrap2.tesseras.net:4433` |
| `-d, --data-dir <PATH>` | Data directory | Platform-specific (see above) |

CLI options override values from the config file.

## Examples

Run with defaults (join the public network):

```bash
tes admin daemon start
```

Run as a seed node (no bootstrap, other nodes connect to you):

```bash
tes admin daemon start --bootstrap ""
```

Run on a custom port with a specific data directory:

```bash
tes admin daemon start --listen 0.0.0.0:5000 --data-dir /var/lib/tesseras
```

Bootstrap from a specific node:

```bash
tes admin daemon start --bootstrap "192.168.1.50:4433"
```

Join a local network of multiple nodes:

```bash
tes admin daemon start --bootstrap "192.168.1.10:4433,192.168.1.11:4433"
```

## Node identity

Each node has a unique identity stored in `<data-dir>/identity.key`. This file contains a 32-byte public key and an 8-byte proof-of-work nonce.

The node ID is derived from the public key: `BLAKE3(pubkey || nonce)` truncated to 20 bytes. The nonce must produce a hash with 8 leading zero bits, which takes about 256 hash attempts. This lightweight proof-of-work makes creating thousands of fake identities expensive while costing legitimate users less than a second.

The identity is generated automatically on first run and reused on subsequent runs. If you delete `identity.key`, a new identity will be generated.

## Logging

The daemon uses structured logging via `tracing`. Control the log level with the `RUST_LOG` environment variable:

```bash
# Default (info level)
tes admin daemon start

# Debug logging
RUST_LOG=debug tes admin daemon start

# Only show warnings and errors
RUST_LOG=warn tes admin daemon start

# Debug for DHT, info for everything else
RUST_LOG=info,tesseras_dht=debug tes admin daemon start
```

## Shutting down

Press **Ctrl+C** to initiate graceful shutdown. The daemon will:

1. Stop accepting new connections
2. Finish in-flight operations (up to 5 seconds)
3. Close all QUIC connections
4. Exit cleanly

## Firewall

The daemon communicates over UDP port 4433 (QUIC). If you're behind a firewall, ensure this port is open for both inbound and outbound UDP traffic.

```bash
# Example: Linux with ufw
sudo ufw allow 4433/udp
```
