# tesseras-net

QUIC transport (quinn), NAT traversal, relay protocol, local discovery.

## Transport Layer

QUIC via `quinn`:

- Built-in congestion control
- Stream multiplexing (DHT ops + bulk transfer simultaneously)
- TLS 1.3 integrated
- Better NAT traversal than TCP
- Connection migration (Wi-Fi → 4G without drop)

## NAT Traversal

In order of preference:

1. UDP hole punching via QUIC (~80% of NATs)
2. UPnP / NAT-PMP for automatic port forwarding
3. Relay nodes as fallback (any public-IP node can volunteer)

## Bootstrap and Discovery

1. **Hardcoded bootstrap nodes**: list in client binary
2. **DNS discovery**: TXT records on `_tesseras-bootstrap.tesseras.net`
3. **Local discovery**: mDNS/DNS-SD on LAN
4. **Peer exchange**: receive peer lists from neighbors once connected

## Feature Flags

```toml
[features]
default = ["quic", "mdns"]
quic = ["quinn"]
mdns = []       # Local network discovery
relay = []      # Act as relay node
```
