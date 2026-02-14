# tesseras-embedded

Embeddable node for mobile/desktop via FFI (flutter_rust_bridge).

## Architecture

Wraps the same core crates as the daemon (`tesseras-dht`, `tesseras-net`, `tesseras-storage`, `tesseras-replication`) and exposes them via `flutter_rust_bridge`. The bridge auto-generates Dart bindings from Rust function signatures.

```rust
// tesseras-embedded/src/api.rs
pub fn node_start(config_path: String) -> Result<(), TesserasError> { ... }
pub fn node_stop() -> Result<(), TesserasError> { ... }

pub async fn create_memory(
    media_path: String,
    context: String,
    memory_type: MemoryType,
    visibility: Visibility,
) -> Result<MemoryId, TesserasError> { ... }

pub async fn get_timeline(offset: u32, limit: u32) -> Result<Vec<Memory>, TesserasError> { ... }
pub async fn get_network_stats() -> Result<NetworkStats, TesserasError> { ... }
```

No manual JNI/Swift glue code. The bridge handles marshalling, async/await, streams, and error propagation across FFI.

## Node Lifecycle on Mobile

| State | Network Participation |
|-------|----------------------|
| App in foreground | Full DHT: lookup, store, respond, transfer |
| Background (Wi-Fi + charging) | Sync, replicate (BGProcessingTask / WorkManager) |
| Background (cellular/battery) | No activity |
| App closed/killed | Offline, peers detect via PING timeout |

Phone-only users have lower availability. Their tessera is still replicated to always-on nodes.

## Connection Modes

```
Phone with daemon (RPi/VPS):
  App ──GraphQL──> Daemon (persistence)
  App ──P2P──> Network (when active, for speed)

Phone only:
  App ──FFI──> Embedded Rust Node ──P2P──> Network
```

## Constraints and Solutions

| Constraint | Solution |
|-----------|---------|
| OS kills background processes | Opportunistic sync (foreground or BGProcessingTask/WorkManager) |
| Limited storage | Own tessera + configurable fragment contributions |
| NAT | UDP hole punching via QUIC, relay fallback |
| Camera formats (HEIC, H.265) | Convert to durable formats in background |
| Battery | No network unless foreground or Wi-Fi+charging |

Internal storage: SQLite + app sandbox blobs. Tessera directory format used for export only.

## Feature Flags

```toml
[features]
default = ["mobile"]
mobile = [
    "tesseras-net/quic",
    "tesseras-crypto/classical",
    "tesseras-crypto/erasure",
    "flutter_rust_bridge",
]
post-quantum = ["tesseras-crypto/post-quantum"]
```
