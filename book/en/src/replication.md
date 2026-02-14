# Replication and Repair

This chapter explains how Tesseras keeps your memories safe even when individual nodes go offline or suffer hardware failures. You don't need to understand these details to use Tesseras — the daemon handles everything automatically.

## Why replication matters

A tessera stored on a single machine dies when that machine dies. Tesseras solves this by splitting data into fragments, spreading them across multiple peers, and continuously verifying that enough copies exist. If some fragments disappear, the network repairs itself automatically.

## Erasure coding

Tesseras uses **Reed-Solomon erasure coding** to create redundant fragments. The idea is simple: from N data fragments, generate M extra parity fragments. Any N of the N+M total fragments can reconstruct the original data.

This is far more storage-efficient than simple replication. Storing 3 complete copies of a 100 MB file costs 300 MB. With 16 data + 8 parity fragments, you get stronger protection (can lose up to 8 of 24 fragments — 33%) for only 150 MB total.

## Fragmentation tiers

Not every tessera is treated the same way. Small files don't benefit from erasure coding overhead, so Tesseras uses three tiers:

| Tier | Size | Strategy | Fragments |
|------|------|----------|-----------|
| **Small** | < 4 MB | Whole-file replication | 7 copies of the complete file |
| **Medium** | 4–256 MB | Reed-Solomon 16+8 | 16 data + 8 parity = 24 fragments |
| **Large** | ≥ 256 MB | Reed-Solomon 48+24 | 48 data + 24 parity = 72 fragments |

All tiers target a **replication factor of 7** — meaning fragments are distributed to 7 different peers.

## How distribution works

When you create a tessera and the daemon replicates it, this is what happens:

1. **Encode** — the tessera data is split into fragments according to its size tier
2. **Find peers** — the daemon queries the DHT for the closest nodes to the tessera's hash
3. **Subnet diversity** — peers are filtered so that no more than a few come from the same network subnet (to avoid correlated failures if a datacenter goes down)
4. **Distribute** — fragments are pushed to the selected peers in round-robin order
5. **Acknowledge** — each peer validates the fragment's checksum and confirms receipt

The tessera owner pushes fragments to peers. Peers don't pull — this keeps the protocol simple and ensures immediate distribution.

## Fragment verification

Every fragment carries a BLAKE3 checksum. When a node receives a fragment, it recomputes the hash and compares it to the expected checksum. If they don't match, the fragment is rejected. This catches both transmission errors and deliberate tampering.

Fragments are stored on disk as individual files with a SQLite metadata index tracking their checksums, sizes, and ownership.

## Repair loop

The daemon runs a background repair loop every 24 hours (with random jitter to avoid network-wide storms). For each tessera it's responsible for, the repair loop:

1. **Requests attestations** from known holders — each holder proves it still has the fragments by reporting their checksums
2. **Falls back to ping** if attestation fails — to distinguish between "node is down" and "node lost the data"
3. **Checks local fragments** — verifies integrity of any fragments stored locally by recomputing BLAKE3 checksums
4. **Decides action**:
   - **Healthy** — all holders responded, all checksums valid, nothing to do
   - **Needs replication** — some holders are gone, find new peers and redistribute missing fragments
   - **Corrupt local** — a local fragment has bad data, fetch a replacement from the network

## Reciprocity

Tesseras uses a **bilateral reciprocity ledger** to ensure fair storage exchange. There is no cryptocurrency, no blockchain, no global consensus — each node simply tracks its balance with each peer locally:

```
peer_a: +500 MB  (they store 500 MB of mine)
peer_b: -200 MB  (I store 200 MB more of theirs than they store of mine)
peer_c:    0 MB  (balanced)
```

The rules are simple:

- Store 1 GB on the network → you should store roughly 1 GB for others
- Nodes with a positive balance (they store more for you) get priority when you need to distribute new fragments
- Free riders gradually lose redundancy — their fragments are deprioritized for repair, but never deleted
- When receiving a fragment, a node checks the sender's deficit. If the sender owes too much storage, the fragment is rejected
- Institutional nodes (universities, archives) can operate altruistically with imbalanced ratios

## Maximum tessera size

The maximum tessera size is **1 GB**. This is a practical limit that keeps fragment sizes manageable and replication fast. For larger collections of memories, create multiple tesseras.

## Configuration

The daemon's replication behavior can be tuned through configuration:

| Parameter | Default | Description |
|-----------|---------|-------------|
| Repair interval | 24 hours | How often the repair loop runs |
| Repair jitter | 2 hours | Random delay added to avoid network-wide storms |
| Concurrent transfers | 4 | Maximum parallel fragment transfers |
| Minimum free space | 1 GB | Stop accepting fragments below this threshold |
| Deficit allowance | 256 MB | Maximum storage deficit before rejecting a peer's fragments |
| Per-peer limit | 1 GB | Maximum total storage for any single peer |
