+++
title = "Releases"
description = "Tesseras software releases and downloads"
+++

No releases yet. Tesseras is in early development (Phase 0).

### Release Format

When available, releases will include:

| File | Description |
|------|-------------|
| `tesseras-X.Y.Z.tar.gz` | Source tarball |
| `tesseras-X.Y.Z.tar.gz.sig` | Signify signature |
| `SHA256` | BLAKE3 checksums |
| `CHANGELOG.md` | What changed |

Releases follow [Semantic Versioning](https://semver.org/). Tarballs are signed with [signify](https://man.openbsd.org/signify).

### Verifying a Release

```
signify -Vep tesseras.pub -m tesseras-X.Y.Z.tar.gz -x tesseras-X.Y.Z.tar.gz.sig
b3sum -c SHA256
```
