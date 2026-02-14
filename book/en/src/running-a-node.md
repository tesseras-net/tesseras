# Running a Node

The `tesseras-daemon` binary runs a full Tesseras node that participates in the peer-to-peer network. It listens for connections over QUIC, joins the distributed hash table (DHT), and enables other nodes to discover and find tessera pointers.

## Starting the daemon

```bash
tesseras-daemon
```

On first run, the daemon:

1. Creates the data directory (`~/.local/share/tesseras` on Linux, `~/Library/Application Support/tesseras` on macOS)
2. Generates a node identity with proof-of-work (takes about 1 second)
3. Binds a QUIC listener on `0.0.0.0:4433`
4. Bootstraps into the network by contacting seed nodes
5. Prints `daemon ready` when fully operational

## Command-line options

```
tesseras-daemon [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config <PATH>` | Path to a TOML config file | None (uses built-in defaults) |
| `-l, --listen <ADDR>` | Address and port to listen on | `0.0.0.0:4433` |
| `-b, --bootstrap <ADDRS>` | Comma-separated bootstrap addresses | `boot1.tesseras.net:4433,boot2.tesseras.net:4433` |
| `-d, --data-dir <PATH>` | Data directory | Platform-specific (see above) |

CLI options override values from the config file.

## Examples

Run with defaults (join the public network):

```bash
tesseras-daemon
```

Run as a seed node (no bootstrap, other nodes connect to you):

```bash
tesseras-daemon --bootstrap ""
```

Run on a custom port with a specific data directory:

```bash
tesseras-daemon --listen 0.0.0.0:5000 --data-dir /var/lib/tesseras
```

Bootstrap from a specific node:

```bash
tesseras-daemon --bootstrap "192.168.1.50:4433"
```

Join a local network of multiple nodes:

```bash
tesseras-daemon --bootstrap "192.168.1.10:4433,192.168.1.11:4433"
```

## Node identity

Each node has a unique identity stored in `<data-dir>/identity.key`. This file contains a 32-byte public key and an 8-byte proof-of-work nonce.

The node ID is derived from the public key: `BLAKE3(pubkey || nonce)` truncated to 20 bytes. The nonce must produce a hash with 8 leading zero bits, which takes about 256 hash attempts. This lightweight proof-of-work makes creating thousands of fake identities expensive while costing legitimate users less than a second.

The identity is generated automatically on first run and reused on subsequent runs. If you delete `identity.key`, a new identity will be generated.

## Logging

The daemon uses structured logging via `tracing`. Control the log level with the `RUST_LOG` environment variable:

```bash
# Default (info level)
tesseras-daemon

# Debug logging
RUST_LOG=debug tesseras-daemon

# Only show warnings and errors
RUST_LOG=warn tesseras-daemon

# Debug for DHT, info for everything else
RUST_LOG=info,tesseras_dht=debug tesseras-daemon
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
