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

## Current status: Phase 0

Tesseras is in **Phase 0** — the local-only foundation. You can create tesseras, verify their integrity, and export them as self-contained directories. Networking, replication, and peer-to-peer features are coming in later phases.

Phase 0 gives you:

- Identity generation (Ed25519 keypair)
- Tessera creation from local files
- Content-addressed storage (BLAKE3 hashing)
- Integrity verification
- Self-contained export

## Key concepts

| Concept | Description |
|---------|-------------|
| **Tessera** | A self-contained time capsule of memories |
| **Memory** | A single item (photo, recording, video, or text) within a tessera |
| **Content hash** | A BLAKE3 hash that uniquely identifies a tessera by its contents |
| **Visibility** | Controls who can access a tessera: public, private, or circle |
| **MANIFEST** | A plain-text index listing every file in the tessera with its checksum |
| **Memory type** | Categorizes a memory: moment, reflection, daily, relation, or object |
