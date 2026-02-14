+++
title = "Lançamentos"
description = "Lançamentos e downloads do software Tesseras"
+++

Nenhum lançamento ainda. Tesseras está em desenvolvimento inicial (Fase 0).

### Formato de Lançamento

Quando disponíveis, os lançamentos incluirão:

| Arquivo | Descrição |
|---------|-----------|
| `tesseras-X.Y.Z.tar.gz` | Tarball com código-fonte |
| `tesseras-X.Y.Z.tar.gz.sig` | Assinatura signify |
| `SHA256` | Checksums BLAKE3 |
| `CHANGELOG.md` | O que mudou |

Os lançamentos seguem [Versionamento Semântico](https://semver.org/). Tarballs são assinados com [signify](https://man.openbsd.org/signify).

### Verificando um Lançamento

```
signify -Vep tesseras.pub -m tesseras-X.Y.Z.tar.gz -x tesseras-X.Y.Z.tar.gz.sig
b3sum -c SHA256
```
