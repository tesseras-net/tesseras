# Network Concepts

This chapter explains how Tesseras nodes find each other and locate tessera pointers on the network. You don't need to understand these details to use Tesseras, but they help explain what the daemon is doing in the background.

## How nodes find each other

Tesseras uses a **Kademlia distributed hash table (DHT)** — a proven algorithm used by BitTorrent and other P2P systems for over 20 years. There is no central server. Each node maintains a routing table of peers it knows about, and nodes cooperate to route queries to the right place.

When your node starts, it contacts one or more **bootstrap nodes** (seed nodes with known addresses). Through these initial connections, your node discovers other peers and builds up its routing table. Over time, your node naturally learns about more peers as it participates in the network.

## What the DHT stores

The DHT stores **pointers**, not data. A pointer is a lightweight record that says "tessera X is held by nodes Y and Z." When someone wants to retrieve a tessera, they first look up its pointer in the DHT to find out which nodes have it, then connect directly to those nodes to download the actual data.

This means the DHT stays small and fast — it only tracks who has what, not the content itself.

## Node identity and proof-of-work

Every node has a 160-bit **node ID** derived from its public key. To prevent an attacker from cheaply creating thousands of fake nodes (a **Sybil attack**), generating a node ID requires a small proof-of-work: the node must find a nonce such that `BLAKE3(public_key || nonce)` starts with 8 zero bits.

This takes about 256 hash attempts — under a second on any device, including a Raspberry Pi. But an attacker trying to create 10,000 fake identities would need millions of attempts, making the attack impractical.

## XOR distance

Kademlia defines "closeness" between nodes using the **XOR metric**: the distance between two node IDs is their bitwise XOR. Nodes are responsible for storing pointers whose keys are close to their own ID (in XOR distance). This distributes data evenly across the network without any coordination.

When looking up a tessera pointer, your node asks the peers it knows that are closest to the target key. Those peers point to even closer ones, and so on, until the pointer is found. This **iterative lookup** typically reaches any node in the network within a few hops.

## Transport: QUIC

All communication between nodes uses **QUIC**, a modern transport protocol built on UDP. QUIC provides:

- **Built-in encryption** — every connection uses TLS 1.3
- **NAT-friendly** — works through most network address translators since it's UDP-based
- **Multiplexing** — multiple independent operations over one connection without head-of-line blocking
- **Connection migration** — survives network changes (e.g., switching from Wi-Fi to mobile data)

The daemon listens on UDP port **4433** by default.

## Bootstrap process

When a node starts, it follows this sequence:

1. **Contact seed nodes** — connect to one or more known bootstrap addresses
2. **Exchange pings** — verify the seed is alive and exchange node identities
3. **Self-lookup** — ask the seed for nodes close to your own ID, to populate your routing table
4. **Iterative discovery** — contact the newly discovered nodes, which point you to even more peers

After bootstrap, the node maintains its routing table automatically: it refreshes buckets periodically and replaces unresponsive peers with new ones.

## Node types

Not every device participates in the network the same way:

| Type | Description | Always on? |
|------|-------------|-----------|
| **Full node** | Desktop, server, or Raspberry Pi running `tesd`. Participates fully in the DHT and stores data for other nodes. | Yes |
| **Mobile node** | Phone or tablet running the Tesseras app. Participates in the DHT when the app is active. | No |
| **Browser node** | Web browser running the WASM client. Connects via a relay node. Read-only. | No |
| **IoT node** | ESP32 or similar device on the local network. Stores fragments passively, does not participate in the DHT. | Yes |

The full node daemon is the backbone of the network. The more full nodes running, the more resilient the network becomes.
