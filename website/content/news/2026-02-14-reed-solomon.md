+++
title = "Reed-Solomon: How Tesseras Survives Data Loss"
date = 2026-02-14T14:00:00+00:00
description = "A deep dive into Reed-Solomon erasure coding — what it is, why Tesseras uses it, and the challenges of keeping memories alive across centuries."
+++

Your hard drive will die. Your cloud provider will pivot. The RAID array in your closet will outlive its controller but not its owner. If a memory is stored in exactly one place, it has exactly one way to be lost forever.

Tesseras is a network that keeps human memories alive through mutual aid. The core survival mechanism is **Reed-Solomon erasure coding** — a technique borrowed from deep-space communication that lets us reconstruct data even when pieces go missing.

## What is Reed-Solomon?

Reed-Solomon is a family of error-correcting codes invented by Irving Reed and Gustave Solomon in 1960. The original use case was correcting errors in data transmitted over noisy channels — think Voyager sending photos from Jupiter, or a CD playing despite scratches.

The key insight: if you add carefully computed redundancy to your data *before* something goes wrong, you can recover the original even after losing some pieces.

Here's the intuition. Suppose you have a polynomial of degree 2 — a parabola. You need 3 points to define it uniquely. But if you evaluate it at 5 points, you can lose any 2 of those 5 and still reconstruct the polynomial from the remaining 3. Reed-Solomon generalizes this idea to work over finite fields (Galois fields), where the "polynomial" is your data and the "evaluation points" are your fragments.

In concrete terms:

1. **Split** your data into *k* data shards
2. **Compute** *m* parity shards from the data shards
3. **Distribute** all *k + m* shards across different locations
4. **Reconstruct** the original data from any *k* of the *k + m* shards

You can lose up to *m* shards — any *m*, data or parity, in any combination — and still recover everything.

## Why not just make copies?

The naive approach to redundancy is replication: make 3 copies, store them in 3 places. This gives you tolerance for 2 failures at the cost of 3x your storage.

Reed-Solomon is dramatically more efficient:

| Strategy | Storage overhead | Failures tolerated |
|----------|----------------:|-------------------:|
| 3x replication | 200% | 2 out of 3 |
| Reed-Solomon (16,8) | 50% | 8 out of 24 |
| Reed-Solomon (48,24) | 50% | 24 out of 72 |

With 16 data shards and 8 parity shards, you use 50% extra storage but can survive losing a third of all fragments. To achieve the same fault tolerance with replication alone, you'd need 3x the storage.

For a network that aims to preserve memories across decades and centuries, this efficiency isn't a nice-to-have — it's the difference between a viable system and one that drowns in its own overhead.

## How Tesseras uses Reed-Solomon

Not all data deserves the same treatment. A 500-byte text memory and a 100 MB video have very different redundancy needs. Tesseras uses a three-tier fragmentation strategy:

**Small (< 4 MB)** — Whole-file replication to 7 peers. For small tesseras, the overhead of erasure coding (encoding time, fragment management, reconstruction logic) outweighs its benefits. Simple copies are faster and simpler.

**Medium (4–256 MB)** — 16 data shards + 8 parity shards = 24 total fragments. Each fragment is roughly 1/16th of the original size. Any 16 of the 24 fragments reconstruct the original. Distributed across 7 peers.

**Large (≥ 256 MB)** — 48 data shards + 24 parity shards = 72 total fragments. Higher shard count means smaller individual fragments (easier to transfer and store) and higher absolute fault tolerance. Also distributed across 7 peers.

The implementation uses the `reed-solomon-erasure` crate operating over GF(2⁸) — the same Galois field used in QR codes and CDs. Each fragment carries a BLAKE3 checksum so corruption is detected immediately, not silently propagated.

```
Tessera (120 MB photo album)
    ↓ encode
16 data shards (7.5 MB each) + 8 parity shards (7.5 MB each)
    ↓ distribute
24 fragments across 7 peers (subnet-diverse)
    ↓ any 16 fragments
Original tessera recovered
```

## The challenges

Reed-Solomon solves the mathematical problem of redundancy. The engineering challenges are everything around it.

### Fragment tracking

Every fragment needs to be findable. Tesseras uses a Kademlia DHT for peer discovery and fragment-to-peer mapping. When a node goes offline, its fragments need to be re-created and distributed to new peers. This means tracking which fragments exist, where they are, and whether they're still intact — across a network with no central authority.

### Silent corruption

A fragment that returns wrong data is worse than one that's missing — at least a missing fragment is honestly absent. Tesseras addresses this with attestation-based health checks: the repair loop periodically asks fragment holders to prove possession by returning BLAKE3 checksums. If a checksum doesn't match, the fragment is treated as lost.

### Correlated failures

If all 24 fragments of a tessera land on machines in the same datacenter, a single power outage kills them all. Reed-Solomon's math assumes independent failures. Tesseras enforces **subnet diversity** during distribution: no more than 2 fragments per /24 IPv4 subnet (or /48 IPv6 prefix). This spreads fragments across different physical infrastructure.

### Repair speed vs. network load

When a peer goes offline, the clock starts ticking. Lost fragments need to be re-created before more failures accumulate. But aggressive repair floods the network. Tesseras balances this with a configurable repair loop (default: every 24 hours with 2-hour jitter) and concurrent transfer limits (default: 4 simultaneous transfers). The jitter prevents repair storms where every node checks its fragments at the same moment.

### Long-term key management

Reed-Solomon protects against data loss, not against losing access. If a tessera is encrypted (private or sealed visibility), you need the decryption key to make the recovered data useful. Tesseras separates these concerns: erasure coding handles availability, while Shamir's Secret Sharing (a future phase) will handle key distribution among heirs. The project's design philosophy — encrypt as little as possible — keeps the key management problem small.

### Galois field limitations

The GF(2⁸) field limits the total number of shards to 255 (data + parity combined). For Tesseras, this is not a practical constraint — even the Large tier uses only 72 shards. But it does mean that extremely large files with thousands of fragments would require either a different field or a layered encoding scheme.

### Evolving codec compatibility

A tessera encoded today must be decodable in 50 years. Reed-Solomon over GF(2⁸) is one of the most widely implemented algorithms in computing — it's in every CD player, every QR code scanner, every deep-space probe. This ubiquity is itself a survival strategy. The algorithm won't be forgotten because half the world's infrastructure depends on it.

## The bigger picture

Reed-Solomon is a piece of a larger puzzle. It works in concert with:

- **Kademlia DHT** for finding peers and routing fragments
- **BLAKE3 checksums** for integrity verification
- **Bilateral reciprocity** for fair storage exchange (no blockchain needed)
- **Subnet diversity** for failure independence
- **Automatic repair** for maintaining redundancy over time

No single technique makes memories survive. Reed-Solomon ensures that data *can* be recovered. The DHT ensures fragments *can be found*. Reciprocity ensures peers *want to help*. Repair ensures none of this degrades over time.

A tessera is a bet that the sum of these mechanisms, running across many independent machines operated by many independent people, is more durable than any single institution. Reed-Solomon is the mathematical foundation of that bet.
