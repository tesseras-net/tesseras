# Open Questions

Unresolved design decisions that need resolution before or during implementation.

## 1. Identity Recovery

**Problem**: identity is a keypair on a device. Losing the device means losing ownership.

**Tensions**: recovery requires either centralization (recovery server) or UX complexity (seed phrases, social recovery).

**Possible directions**:

- Shamir's Secret Sharing of private key across N devices/heirs (M-of-N recovery)
- Social recovery: trusted contacts each hold a key shard (similar to Argent wallet)
- Device-based: phone + daemon/RPi replicate keys, losing one is recoverable
- Accept the trade-off: lose all devices without recovery setup → tessera becomes read-only (public content accessible, no new memories)

## 2. Content Moderation

**Problem**: P2P with no central authority cannot moderate content. Someone could store illegal material.

**Tensions**: moderation requires seeing content, contradicts privacy. Refusing to moderate contradicts legal obligations.

**Possible directions**:

- Fragments are erasure-coded and unintelligible individually (legal defense)
- Public tesseras flagged by N independent nodes → delisted from public indexes (not deleted)
- Institutional nodes apply their own moderation policies
- Client-side: app refuses to display content flagged by threshold of peers
- Transparency reports

## 3. Tessera Size Limits and Storage Policy

**Problem**: assumes ~3GB but no hard limit. A 200GB tessera breaks reciprocity on mobile.

**Questions**:

- Maximum size? Or tiered reciprocity?
- Minimum storage contribution to participate?
- Handling tesseras larger than any single IoT/mobile node?
- "Lightweight tessera" mode (text + photos, no video)?

## 4. Protocol Versioning and Migration

**Problem**: tessera format has versioning, wire protocol does not.

**Questions**:

- Protocol version in QUIC handshake? Negotiate highest common?
- How long to support old versions? (Network-wide upgrade impossible in P2P)
- Policy: additive-only (new message types ignored by old nodes)?
- Hard fork mechanism if necessary?

## 5. Discovery and Search of Public Tesseras

**Problem**: DHT finds by hash (need exact hash). Killer use case is exploration ("memories from São Paulo in 2020s").

**Possible directions**:

- Institutional nodes maintain searchable indexes
- Federated index protocol: opt-in nodes publish metadata, others query
- tesseras.net hosts centralized convenience index (not required)
- Tags and location in meta.json enable structured queries
- Full-text search over context.txt of public memories

## 6. Project Governance

**Problem**: who decides protocol changes, holds commit access, maintains bootstrap nodes, holds signify key?

**Possible directions**:

- Start as BDFL (pragmatic for early stage)
- Transition to small council of maintainers
- Long-term: foundation (Apache/Linux Foundation style) for domain, keys, bootstrap
- RFC process for protocol changes
- Signify key held by multiple parties or foundation

## 7. Threat Model

Beyond network-level attacks (Eclipse, Sybil, poisoning):

- **State censorship**: traffic looks like QUIC (HTTP/3), hard to distinguish. Domain fronting as last resort.
- **Targeted deletion**: r=7 + erasure coding requires compromising many independent nodes.
- **Spam/flooding**: millions of garbage tesseras. PoW on creation? Reciprocity makes it expensive.
- **Metadata analysis**: traffic patterns reveal who/when. Onion routing probably overkill.
- **Compromised bootstrap**: multiple independent sources (hardcoded, DNS, local discovery).
- **Long-term key compromise**: in 100 years Ed25519 may be broken. ML-DSA dual signing mitigates. Pre-ML-DSA tesseras need re-signing mechanism.

## 8. Offline-First and Conflict Resolution

**Problem**: offline edits on phone + desktop create divergent versions.

**Recommendation**: append-only by default. Memories immutable once created. Metadata corrections are new entries that supersede old ones (like Git commits). MANIFEST regenerated from union of all memories. Eliminates conflict resolution entirely.

## 9. Internationalization (i18n)

- Minimum launch languages: English, Portuguese, Spanish, Mandarin, Arabic, Hindi (~60% of world)
- Flutter built-in i18n (ARB + `intl` package) from start
- Memory prompts curated per culture, not just translated
- README.decode already multilingual by design

## 10. Long-Term Storage Economics

**Problem**: in 200 years, most tesseras belong to dead people who don't run nodes.

**Possible directions**:

- **Institutional anchors**: libraries/museums absorb cost as institutional mandate (precedent: monasteries preserving manuscripts)
- **Endowment model**: store 2x during lifetime, surplus sustains after death
- **Cultural value increases**: a 2026 tessera is priceless in 2226
- **Network contraction acceptable**: remaining nodes hold curated subset
- **Accept uncertainty**: make format so simple it survives whatever future mechanism emerges
