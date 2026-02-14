# tesseras-dht

Kademlia DHT implementation: routing table, RPCs, peer management.

## Kademlia Parameters

- **Node ID**: 160-bit, derived from Ed25519 public key: `BLAKE3(pubkey)[..20]`
- **Routing table**: k-buckets with k=20, 160 buckets
- **Anti-Sybil**: lightweight proof-of-work on Node ID generation

## Protocol Messages

```rust
pub enum Message {
    // Standard Kademlia
    Ping { sender: NodeId },
    Pong { sender: NodeId },
    FindNode { target: NodeId },
    FindNodeResponse { nodes: Vec<NodeInfo> },
    FindValue { key: ContentHash },
    FindValueResponse(FindValueResult),
    Store { key: ContentHash, value: TesseraPointer },

    // Tesseras additions
    Replicate {
        tessera_hash: ContentHash,
        fragment_id: u32,
        data: Vec<u8>,
    },
    ReplicateAck {
        accepted: bool,
        credit: i64,
    },
    Attest {
        tessera_hash: ContentHash,
        timestamp: i64,
        signature: Vec<u8>,
    },
}
```

## DHT Stores Pointers, Not Data

The DHT stores lightweight pointers to tessera holders:

```json
{
  "tessera_hash": "blake3:...",
  "size_bytes": 3221225472,
  "fragment_count": 72,
  "holders": [
    {
      "node_id": "...",
      "last_seen": "2026-02-13T...",
      "fragments": [0, 1, 2, "...", 71],
      "attestation": "..."
    }
  ],
  "metadata_preview": {
    "creator_name": "João",
    "created_at": "2026-02-13",
    "visibility": "public",
    "language": "pt-BR"
  }
}
```

Actual data transfer happens directly between nodes via QUIC streams, not through the DHT.

## Network Security

| Attack | Mitigation |
|--------|-----------|
| Eclipse (surround a node) | Diversify k-buckets, prefer long-lived nodes, cross-IP connections |
| Sybil (mass fake nodes) | PoW on Node ID + reputation by uptime |
| Data poisoning | BLAKE3 checksums per fragment, discard and re-fetch |
| Censorship | r=7 replication + erasure coding (need >33% compromised) |

## Wire Protocol

MessagePack over QUIC streams. Serialization via `rmp-serde`.

```
With daemon:    App ──GraphQL──> Daemon ──QUIC/MsgPack──> P2P Network
Mobile direct:  App ──FFI──> Embedded Node ──QUIC/MsgPack──> P2P Network
```
