# Configuration

The daemon can be configured via a TOML file. Pass the path with `--config`:

```bash
tesseras-daemon --config /etc/tesseras/config.toml
```

If no config file is given, the daemon uses sensible defaults. CLI options (`--listen`, `--bootstrap`, `--data-dir`) override the corresponding config values.

## Full example

```toml
[node]
data_dir = "~/.local/share/tesseras"
listen_addr = "0.0.0.0:4433"

[dht]
k = 20
alpha = 3
bucket_refresh_interval_secs = 3600
republish_interval_secs = 3600
pointer_ttl_secs = 86400
max_stored_pointers = 100000
ping_failure_threshold = 3

[bootstrap]
dns_domain = "_tesseras._udp.tesseras.net"
hardcoded = [
    "boot1.tesseras.net:4433",
    "boot2.tesseras.net:4433",
]

[network]
enable_mdns = true

[observability]
metrics_addr = "127.0.0.1:9190"
log_format = "json"
```

## Sections

### `[node]`

Basic node settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `data_dir` | path | Platform-specific | Where to store identity, database, and blobs |
| `listen_addr` | address | `0.0.0.0:4433` | QUIC listener address |

The default `data_dir` is `~/.local/share/tesseras` on Linux and `~/Library/Application Support/tesseras` on macOS.

### `[dht]`

Kademlia DHT tuning parameters. The defaults work well for most deployments.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `k` | integer | `20` | Maximum entries per routing table bucket |
| `alpha` | integer | `3` | Parallelism for iterative lookups |
| `bucket_refresh_interval_secs` | integer | `3600` | How often to refresh routing table buckets (seconds) |
| `republish_interval_secs` | integer | `3600` | How often to republish stored pointers (seconds) |
| `pointer_ttl_secs` | integer | `86400` | How long to keep a pointer before it expires (seconds) |
| `max_stored_pointers` | integer | `100000` | Maximum number of pointers to store locally |
| `ping_failure_threshold` | integer | `3` | How many consecutive ping failures before removing a peer |

### `[bootstrap]`

How the node discovers its first peers when joining the network.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `dns_domain` | string | `_tesseras._udp.tesseras.net` | DNS domain for TXT-record-based peer discovery |
| `hardcoded` | list of strings | `["boot1.tesseras.net:4433", "boot2.tesseras.net:4433"]` | Fallback bootstrap addresses |

### `[network]`

Network-level features.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enable_mdns` | boolean | `true` | Enable local network discovery via mDNS |

### `[observability]`

Monitoring and logging.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `metrics_addr` | address | `127.0.0.1:9190` | Address for the Prometheus metrics endpoint |
| `log_format` | string | `json` | Log output format (`json` or `text`) |

## Minimal config

Most users don't need a config file at all. If you do, a minimal config overriding only what you need is enough:

```toml
[node]
listen_addr = "0.0.0.0:5000"

[bootstrap]
hardcoded = ["192.168.1.10:4433"]
```

All other values use their defaults.
