# Gap Analysis — O que falta para uso real

Data: 2026-02-15 | Atualizado: 2026-02-16

Cenário: 3 usuários reais querem compartilhar memórias na rede.

- **Ivan** — Linux, CLI, técnico
- **Esposa** — Windows, leiga em tecnologia
- **Amigo** — macOS, frontend developer, sensível a UX

---

## O que JÁ funciona hoje

- **CLI (`tes`)**: criar, verificar, exportar, listar, publish, fetch, status, peers, heir management
- **Daemon**: roda, conecta via QUIC, faz DHT, replica fragmentos, RPC via Unix socket
- **CLI ↔ Daemon**: publish e fetch funcionam via RPC
- **Bootstrap nodes**: 2 VPS live (Falkenstein + Helsinki), DNS SRV configurado
- **Crypto completa**: assinaturas duais, encryption, Shamir para herdeiros
- **Storage**: SQLite + blobs, deduplicação, busca full-text
- **Embedded node**: FFI bridge para Flutter funcional
- **Flutter UI**: telas completas (onboarding, timeline, rede, settings) com shadcn_flutter, i18n pt-BR/en — usando mock data
- **Packaging**: .deb (Debian), .pkg.tar.zst (Arch), .exe (Windows via Inno Setup), APKBUILD (Alpine), rc.d (OpenBSD)
- **E2E tests**: suite completa (local + rede)
- **259+ testes passando**, binários compilam

---

## O que FALTA

### ~~Bloco 1 — Rede~~ CONCLUÍDO

1. ~~**Bootstrap nodes rodando**~~ — **FEITO**. 2 VPS Hetzner (bootstrap1/bootstrap2.tesseras.net), DNS SRV `_tesseras._udp.tesseras.net`, fallback hardcoded.

2. ~~**Daemon empacotado**~~ — **FEITO**. .deb, .pkg.tar.zst, .exe, APKBUILD, rc.d. Falta apenas macOS (.dmg).

3. ~~**CLI ↔ Daemon integração**~~ — **FEITO**. `tes publish` e `tes fetch` via Unix socket RPC.

### ~~Bloco 2 — Para Ivan (Linux, CLI)~~ PARCIALMENTE CONCLUÍDO

4. ~~**Comando `tes publish`**~~ — **FEITO**. Publica tessera na rede via daemon.

5. ~~**Comando `tes fetch`**~~ — **FEITO**. Busca tessera por hash (hex ou base32), importa no storage local.

6. **Discovery por identidade** — **PENDENTE**. Sem busca por criador, só por hash. Falta `find_by_creator()` no storage + DHT lookup por chave pública.

### Bloco 3 — Para esposa (Windows, leiga)

7. ~~**Flutter app com UI funcional**~~ — **FEITO** (UI). Telas completas com shadcn_flutter: onboarding, timeline, criar memória, rede, settings. **MAS**: usa mock data. Falta conectar ao backend Rust via `flutter_rust_bridge`.

8. ~~**Build Windows**~~ — **FEITO**. Inno Setup installer (29MB .exe).

9. **Onboarding zero-config** — **PARCIAL**. Bootstrap nodes hardcoded no daemon, mas Flutter app ainda não usa o embedded node real (está em mocks).

### Bloco 4 — Para amigo (macOS, frontend dev)

10. **Build macOS** — **PENDENTE**. Sem .dmg, sem Homebrew formula.

11. ~~**UX review do Flutter app**~~ — **FEITO**. UI implementada com shadcn_flutter, pronta para review.

### Bloco 5 — Funcionalidades sociais (compartilhar memórias)

12. **Circles (grupos de compartilhamento)** — **PENDENTE**. `Visibility::Circle` existe como enum, zero lógica de membership, sem lista de membros no manifest.

13. **Feed/Timeline de rede** — **PENDENTE**. GraphQL API (`tesseras-api`) completamente vazio (só CLAUDE.md com spec).

14. **Notificações** — **PENDENTE**. Sem implementação.

### Bloco 6 — Integração Flutter ↔ Rust (NOVO, P0)

15. **Flutter FFI bridge real** — **PENDENTE**. Providers usam mock data. Precisa conectar `flutter_rust_bridge` ao embedded node para: criar identidade, criar/listar tesseras, status de rede, publicar/buscar.

16. **`tes sync` contínuo** — **PENDENTE**. Fetch é one-shot. Falta modo de sync que fique ouvindo novidades.

---

## Prioridade (atualizada)

| Prioridade | Item | Impacto |
|-----------|------|---------|
| ~~**P0**~~ | ~~Bootstrap nodes~~ | ~~FEITO~~ |
| ~~**P0**~~ | ~~CLI↔Daemon (publish/fetch)~~ | ~~FEITO~~ |
| **P0** | Flutter FFI bridge real (15) | Esposa e amigo podem usar o app de verdade |
| **P1** | Discovery por identidade (6) | Encontrar tesseras sem trocar hashes |
| **P1** | Circles membership (12) | Compartilhar só com família/amigos |
| **P1** | Build macOS (10) | Amigo pode instalar |
| **P2** | `tes sync` contínuo (16) | Sync automático de novidades |
| **P2** | Onboarding zero-config completo (9) | Esposa não configura nada |
| **P3** | GraphQL API (13) | Base para features sociais |
| **P3** | Feed de rede + notificações (14) | Experiência social |

---

## Caminho mais rápido para testar juntos

~~1. Subir 1 VPS barata (Hetzner CX22, ~€4/mês) com o daemon como bootstrap~~ — **FEITO**
~~2. Rodar o daemon na máquina Linux apontando para o bootstrap~~ — **FEITO**
~~3. Criar tesseras via CLI — replicam automaticamente~~ — **FEITO**
4. **PRÓXIMO**: Conectar Flutter app ao backend Rust via FFI bridge
5. Build macOS para o amigo
6. Implementar circles para compartilhamento em grupo

---

## Sequência de implementação sugerida (atualizada)

```
Bloco 1 (Rede)          ████████████████████████████ CONCLUÍDO
  ├─ Bootstrap nodes     [FEITO - 2 VPS live]
  ├─ CLI↔Daemon sync     [FEITO - publish/fetch]
  └─ Discovery           [PENDENTE - por identidade]

Bloco 6 (Flutter FFI)   ─────────────────────────────► PRÓXIMO
  ├─ Providers reais     [substituir mocks por FFI]
  ├─ Onboarding real     [criar identidade via Rust]
  ├─ CRUD tesseras       [criar/listar/buscar via embedded node]
  └─ Status de rede      [peers/replicação real]

Bloco 2 (Circles)       ──────────────►
  ├─ Circle membership   [quem está no circle]
  └─ Circle encryption   [chaves compartilhadas]

Bloco 3 (Builds)        ──────────►
  ├─ macOS .dmg          [para o amigo]
  └─ sync contínuo       [tes sync]

Bloco 4 (API)           ─────────►
  └─ GraphQL básico      [queries + mutations]
```
