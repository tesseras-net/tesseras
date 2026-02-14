+++
title = "FAQ"
description = "Frequently asked questions about Tesseras"
+++

### What is a tessera?

A tessera is a self-contained time capsule of memories — photos, audio recordings, video, and text — packaged in a format designed to survive independently of any software, company, or infrastructure. The name comes from the small tiles used in Roman mosaics: each piece is simple, but together they form something that endures.

### How does my data survive if my computer dies?

Your tessera is replicated across multiple nodes in the Tesseras peer-to-peer network. It uses erasure coding (Reed-Solomon) to split your data into redundant fragments. Even if several nodes go offline permanently, your tessera can be reconstructed from the remaining fragments.

### Is my data encrypted?

By default, no. Tesseras prioritizes availability over secrecy — the goal is that your memories survive, even if the software to decrypt them doesn't. You can mark individual memories as private (encrypted with AES-256-GCM) or sealed (to be opened after a specific date), but public and circle-visibility memories are stored unencrypted to maximize their chances of long-term survival.

### Do I need to pay anything?

No. The network runs on mutual aid: you store fragments of other people's tesseras, and they store yours. There are no tokens, no blockchain, no subscription fees. The only cost is the storage space you contribute to the network.

### What platforms does it run on?

Tesseras runs on Linux, macOS, FreeBSD, OpenBSD, Windows, Android, and iOS. There's also a browser-based viewer and support for low-power IoT devices (ESP32) as passive storage nodes.

### How is this different from IPFS, Filecoin, or Arweave?

Tesseras is designed specifically for personal memory preservation, not general-purpose file storage. Key differences:

- **No cryptocurrency or tokens** — incentives are based on bilateral reciprocity, not financial markets
- **Self-describing format** — each tessera includes instructions for decoding itself in multiple languages, so it can be understood centuries from now without any special software
- **Availability over secrecy** — most data is stored unencrypted to maximize long-term survival
- **Simplest possible media formats** — JPEG, WAV, WebM, plain text — chosen for durability, not features

### What media formats are supported?

- **Photos:** JPEG
- **Audio:** WAV PCM
- **Video:** WebM
- **Text:** UTF-8 plain text

These formats were chosen for maximum longevity and widespread support.

### Can I export my tessera?

Yes. A tessera is a standard directory of files. You can copy it to a USB drive, burn it to optical media, or print the text portions. The format is designed to be readable without any special software.
