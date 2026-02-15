# Introduction

Tesseras is a peer-to-peer network for preserving human memories across millennia. Each person creates a **tessera** — a self-contained time capsule of memories (photos, audio, video, text) that survives independently of any software, company, or infrastructure.

## What is a tessera?

The word *tessera* comes from the small tiles used to make mosaics in the ancient world. In Tesseras, each tessera is a collection of memories packaged into a format designed to be understood even thousands of years from now, without any special software.

A tessera contains:

- **Memories** — photos (JPEG), audio recordings (WAV), video (WebM), and text (plain UTF-8)
- **Metadata** — when and where each memory was created, who it involves, and what it means
- **Identity** — cryptographic signatures proving who created it
- **Decoding instructions** — plain-text explanations of every format used, so future humans can read the contents

## Core philosophy

- **No company dependency** — your memories are yours, stored locally and replicated across a peer-to-peer network
- **No format lock-in** — every tessera includes instructions for decoding its contents
- **Availability over secrecy** — public memories are not encrypted, because long-term accessibility matters more than hiding things
- **Minimal encryption** — only private and sealed content is encrypted; everything else is open
- **Quantum-resistant** — dual signatures (Ed25519 + ML-DSA) protect integrity even against future quantum computers

## Current status: Phase 4

Tesseras has completed through **Phase 4** — encryption and sealed tesseras. The project now covers local tessera management, networking, replication, a mobile app, and cryptographic privacy.

What's available today:

- Identity generation (Ed25519 keypair with proof-of-work)
- Tessera creation from local files
- Content-addressed storage (BLAKE3 hashing)
- Integrity verification and self-contained export
- Full node daemon with QUIC transport
- Peer discovery via Kademlia DHT
- Tessera pointer publishing and lookup across the network
- Reed-Solomon erasure coding with automatic fragment repair
- Flutter mobile app with embedded Rust P2P node
- **Private tesseras** — encrypted content only the owner can access
- **Sealed tesseras** — time-locked content that opens after a specific date
- **Hybrid post-quantum encryption** — X25519 + ML-KEM-768 key encapsulation
- **AES-256-GCM** content encryption with AAD binding

## Key concepts

| Concept | Description |
|---------|-------------|
| **Tessera** | A self-contained time capsule of memories |
| **Memory** | A single item (photo, recording, video, or text) within a tessera |
| **Content hash** | A BLAKE3 hash that uniquely identifies a tessera by its contents |
| **Visibility** | Controls who can access a tessera: public, private, sealed, or circle |
| **Sealed tessera** | A time capsule that can only be opened after a specific date |
| **MANIFEST** | A plain-text index listing every file in the tessera with its checksum |
| **Memory type** | Categorizes a memory: moment, reflection, daily, relation, or object |
| **Node** | A device running the Tesseras daemon, participating in the P2P network |
| **DHT** | Distributed hash table — how nodes find tessera pointers without a central server |
| **Bootstrap** | The process of joining the network by contacting known seed nodes |
