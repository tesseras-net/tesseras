# NAT Traversal

Most devices on the internet sit behind a **NAT** (Network Address Translator). Your router assigns your device a private address (like `192.168.1.100`) and translates it to a public address when you connect outward. This works fine for browsing the web, but it creates a problem for P2P networks: two devices behind different NATs cannot directly connect to each other without help.

Tesseras solves this with a three-tier approach, trying the cheapest option first:

1. **Direct connection** — if both nodes have public IPs, they connect directly
2. **UDP hole punching** — a third node introduces the two peers so they can punch through their NATs
3. **Relay** — a public-IP node forwards packets between the two peers

## NAT type discovery

When a node starts, it sends STUN (Session Traversal Utilities for NAT) requests to multiple public servers. By comparing the external addresses these servers report back, the node classifies its NAT:

| NAT Type | What it means | Hole punching? |
|----------|---------------|----------------|
| **Public** | No NAT — your device has a public IP | Not needed |
| **Cone** | NAT maps the same internal port to the same external port regardless of destination | Works well (~80%) |
| **Symmetric** | NAT assigns a different external port for each destination | Unreliable |
| **Unknown** | Could not reach STUN servers | Relay needed |

Your node advertises its NAT type in DHT Pong messages, so other nodes know whether hole punching is worth attempting.

## Hole punching

When node A (behind a Cone NAT) wants to connect to node B (also behind a Cone NAT), neither can directly reach the other. The solution:

1. A sends a **PunchIntro** message to node I (an introducer — any public-IP node they both know). The message includes A's external address (from STUN) and an Ed25519 signature proving A's identity.

2. I verifies the signature and forwards a **PunchRequest** to B, including A's address and the original signature.

3. B verifies the signature (proving the request really came from A, not a spoofed source). B then sends a UDP packet to A's external address — this opens a pinhole in B's NAT. B also sends a **PunchReady** message back to A with B's external address.

4. A sends a UDP packet to B's external address. Both NATs now have pinholes, and the two nodes can communicate directly.

The entire process takes 2-5 seconds. The Ed25519 signatures prevent **reflection attacks**, where an attacker replays an old introduction to redirect traffic.

## Relay fallback

When hole punching fails (Symmetric NAT, strict firewalls, or corporate networks), nodes fall back to relaying through a public-IP node:

1. A sends a **RelayRequest** to node R (a public-IP node with relay enabled).
2. R creates a session and sends a **RelayOffer** to both A and B, containing the relay address and a session token.
3. A and B send their packets to R, prefixed with the session token. R strips the token and forwards the payload to the other peer.

Relay sessions have bandwidth limits:
- **256 KB/s** for peers with good reciprocity (they store fragments for others)
- **64 KB/s** for peers without reciprocity
- Non-reciprocal sessions are limited to 10 minutes

This encourages nodes to contribute storage — good network citizens get better relay service.

## Address migration

When a mobile device switches networks (Wi-Fi to cellular), its IP address changes. Rather than tearing down and rebuilding relay sessions, the node sends a signed **RelayMigrate** message to update its address in the existing session. This avoids re-establishing connections from scratch.

## Configuration

The `[nat]` section in the daemon config controls NAT traversal:

```toml
[nat]
# STUN servers for NAT type detection
stun_servers = ["stun.l.google.com:19302", "stun.cloudflare.com:3478"]

# Enable relay (forward traffic for other NATed peers)
relay_enabled = false

# Maximum simultaneous relay sessions
relay_max_sessions = 50

# Bandwidth limit for reciprocal peers (KB/s)
relay_reciprocal_kbps = 256

# Bandwidth limit for non-reciprocal peers (KB/s)
relay_bootstrap_kbps = 64

# Relay session idle timeout (seconds)
relay_idle_timeout_secs = 60
```

To run a relay node, set `relay_enabled = true`. Your node must have a public IP (or a port-forwarded router) to serve as a relay.

## Mobile reconnection

When the Tesseras app detects a network change on a mobile device, it runs a three-phase reconnection sequence:

1. **QUIC migration** (0-2s) — QUIC supports connection migration natively. The app tries to migrate all active connections to the new address.
2. **Re-STUN** (2-5s) — discover the new external address and re-announce to the DHT.
3. **Re-establish** (5-10s) — reconnect peers that migration couldn't save, in priority order: bootstrap nodes first, then nodes holding your fragments, then nodes whose fragments you hold.

The app shows reconnection progress through the `NetworkChanged` event stream.

## Monitoring

NAT traversal exposes Prometheus metrics at `/metrics`:

- `tesseras_nat_type` — current detected NAT type
- `tesseras_stun_requests_total` / `tesseras_stun_failures_total` — STUN reliability
- `tesseras_punch_attempts_total{initiator_nat, target_nat}` — punch success rate by NAT pair
- `tesseras_relay_sessions_active` — current relay load
- `tesseras_relay_bytes_forwarded` — total relay bandwidth
- `tesseras_network_change_total` — network change frequency on mobile
