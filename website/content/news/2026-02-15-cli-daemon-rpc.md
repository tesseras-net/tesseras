+++
title = "CLI Meets Network: Publish, Fetch, and Status Commands"
date = 2026-02-15
description = "The tesseras CLI can now publish tesseras to the network, fetch them from peers, and monitor replication status — all through a new Unix socket RPC bridge to the daemon."
+++

Until now the CLI operated in isolation: create a tessera, verify it, export it, list what you have. Everything stayed on your machine. With this release, `tes` gains three commands that bridge the gap between local storage and the P2P network — `publish`, `fetch`, and `status` — by talking to a running `tesd` over a Unix socket.

## What was built

**`tesseras-rpc` crate** — A new shared crate that both the CLI and daemon depend on. It defines the RPC protocol using MessagePack serialization with length-prefixed framing (4-byte big-endian size header, 64 MiB max). Three request types (`Publish`, `Fetch`, `Status`) and their corresponding responses. A sync `DaemonClient` handles the Unix socket connection with configurable timeouts. The protocol is deliberately simple — one request, one response, connection closed — to keep the implementation auditable.

**`tes publish <hash>`** — Publishes a tessera to the network. Accepts full hashes or short prefixes (e.g., `tes publish a1b2`), which are resolved against the local database. The daemon reads all tessera files from storage, packs them into a single MessagePack buffer, and hands them to the replication engine. Small tesseras (< 4 MB) are replicated as a single fragment; larger ones go through Reed-Solomon erasure coding. Output shows the short hash and fragment count:

```
Published tessera 9f2c4a1b (24 fragments created)
Distribution in progress — use `tes status 9f2c4a1b` to track.
```

**`tes fetch <hash>`** — Retrieves a tessera from the network using its full content hash. The daemon collects locally available fragments, reconstructs the original data via erasure decoding if needed, unpacks the files, and stores them in the content-addressable store. Returns the number of memories and total size fetched.

**`tes status <hash>`** — Displays the replication health of a tessera. The output maps directly to the replication engine's internal health model:

| State | Meaning |
|-------|---------|
| Local | Not yet published — exists only on your machine |
| Publishing | Fragments being distributed, critical redundancy |
| Replicated | Distributed but below target redundancy |
| Healthy | Full redundancy achieved |

**Daemon RPC listener** — The daemon now binds a Unix socket (default: `$XDG_RUNTIME_DIR/tesseras/daemon.sock`) with proper directory permissions (0700), stale socket cleanup, and graceful shutdown. Each connection is handled in a Tokio task — the listener converts the async stream to sync I/O for the framing layer, dispatches to the RPC handler, and writes the response back.

**Pack/unpack in `tesseras-core`** — A small module that serializes a list of file entries (path + data) into a single MessagePack buffer and back. This is the bridge between the tessera's directory structure and the replication engine's opaque byte blobs.

## Architecture decisions

- **Unix socket over TCP**: RPC between CLI and daemon happens on the same machine. Unix sockets are faster, don't need port allocation, and filesystem permissions provide access control without TLS.
- **MessagePack over JSON**: the same wire format used everywhere else in Tesseras. Compact, schema-less, and already a workspace dependency. A typical publish request/response round-trip is under 200 bytes.
- **Sync client, async daemon**: the `DaemonClient` uses blocking I/O because the CLI doesn't need concurrency — it sends one request and waits. The daemon listener is async (Tokio) to handle multiple connections. The framing layer works with any `Read`/`Write` impl, bridging both worlds.
- **Hash prefix resolution on the client side**: `publish` and `status` resolve short prefixes locally before sending the full hash to the daemon. This keeps the daemon stateless — it doesn't need access to the CLI's database.
- **Default data directory alignment**: the CLI default changed from `~/.tesseras` to `~/.local/share/tesseras` (via `dirs::data_dir()`) to match the daemon. A migration hint is printed when legacy data is detected.

## What comes next

- **DHT peer count**: the `status` command currently reports 0 peers — wiring the actual peer count from the DHT is the next step
- **`tes show`**: display the contents of a tessera (memories, metadata) without exporting
- **Streaming fetch**: for large tesseras, stream fragments as they arrive rather than waiting for all of them
