# Gap Analysis — O que falta para uso real

Data: 2026-02-15

Cenário: 3 usuários reais querem compartilhar memórias na rede.

- **Ivan** — Linux, CLI, técnico
- **Esposa** — Windows, leiga em tecnologia
- **Amigo** — macOS, frontend developer, sensível a UX

---

## O que JÁ funciona hoje

- **CLI (`tes`)**: criar, verificar, exportar, listar tesseras offline
- **Daemon**: roda, conecta via QUIC, faz DHT, replica fragmentos
- **Crypto completa**: assinaturas duais, encryption, Shamir para herdeiros
- **Storage**: SQLite + blobs, deduplicação, busca full-text
- **Embedded node**: FFI bridge para Flutter funcional
- **259 testes passando**, binários compilam

---

## O que FALTA

### Bloco 1 — Rede (sem isto nada funciona em P2P)

1. **Bootstrap nodes rodando** — Pelo menos 2-3 nós acessíveis para o DHT funcionar. VPS (Hetzner/DO) ou máquinas em casa com port-forwarding na porta 4433/UDP. VPS é mais confiável. Na rede de casa funciona para testes na mesma LAN, mas para uso real entre casas precisa de pelo menos 1 VPS.

2. **Daemon empacotado para instalação fácil** — Hoje precisa compilar do source. Falta: pacote `.deb`, `.pkg`, ou binário pré-compilado por OS.

3. **CLI ↔ Daemon integração** — O CLI cria tesseras localmente mas não há comando `tes publish` ou `tes sync` que empurre a tessera para a rede via daemon. O daemon replica automaticamente o que tem no storage, mas o fluxo CLI→daemon não está fechado.

### Bloco 2 — Para Ivan (Linux, CLI)

4. **Comando `tes publish`/`tes sync`** — Conectar o CLI ao daemon local para publicar tesseras na rede.

5. **Comando `tes pull`/`tes fetch`** — Buscar tesseras de outros na rede pelo hash ou identidade do criador.

6. **Discovery por identidade** — Encontrar tesseras da esposa/amigo pelo nome ou chave pública, não apenas por hash.

### Bloco 3 — Para esposa (Windows, leiga)

7. **Flutter app com UI funcional** — O scaffold existe mas as telas estão vazias:
   - Onboarding (criar identidade sem jargão técnico)
   - Criação de memória (escolher fotos, gravar áudio, adicionar texto)
   - Timeline (ver memórias próprias e de amigos)
   - Status de rede (simplificado)

8. **Build Windows** — O projeto Flutter tem diretório Windows, nunca foi testado/empacotado. Precisa gerar `.exe` instalável.

9. **Onboarding zero-config** — Sem digitar endereço de bootstrap node. App com bootstrap nodes hardcoded ou DNS discovery.

### Bloco 4 — Para amigo (macOS, frontend dev)

10. **Build macOS** — `.dmg` ou `.app`. Aceita rodar via `cargo` também.

11. **UX review do Flutter app** — Ele é frontend dev, vai querer contribuir. O scaffold existe para começar.

### Bloco 5 — Funcionalidades sociais (compartilhar memórias)

12. **Circles (grupos de compartilhamento)** — `Visibility::Circle` existe no enum mas sem implementação de quem pertence a um circle. Precisa de mecanismo para "compartilho com essas 2 pessoas".

13. **Feed/Timeline de rede** — Ver memórias de amigos, não só as próprias. O GraphQL API (`tesseras-api`) está vazio.

14. **Notificações** — "Sua esposa adicionou uma nova memória" — sem isso o app fica passivo.

---

## Prioridade

| Prioridade | Item | Impacto |
|-----------|------|---------|
| **P0** | Bootstrap nodes (1-2 VPS) | Sem rede, nada funciona |
| **P0** | CLI↔Daemon sync (publish/fetch) | Ivan pode usar primeiro |
| **P1** | Discovery por identidade + Circles | Vocês se encontram na rede |
| **P1** | Flutter UI funcional (telas core) | Esposa e amigo podem usar |
| **P2** | Builds empacotados (Win/Mac/Linux) | Instalação fácil |
| **P2** | Onboarding zero-config | Esposa não precisa configurar |
| **P3** | GraphQL API | Base para features sociais |
| **P3** | Feed de rede + notificações | Experiência social |

---

## Caminho mais rápido para testar juntos

1. Subir 1 VPS barata (Hetzner CX22, ~€4/mês) com o daemon como bootstrap
2. Rodar o daemon na máquina Linux apontando para o bootstrap
3. Criar tesseras via CLI — replicam automaticamente
4. Em paralelo, avançar o Flutter app para esposa e amigo

Na rede de casa funciona na mesma LAN — o daemon aceita conexões diretas sem bootstrap. Entre casas diferentes, precisa do VPS.

---

## Sequência de implementação sugerida

```
Bloco 1 (Rede)          ─────────────────────────────►
  ├─ Bootstrap nodes     [deploy VPS]
  ├─ CLI↔Daemon sync     [tes publish/fetch]
  └─ Discovery           [por identidade]

Bloco 2 (Circles)       ──────────────►
  ├─ Circle membership   [quem está no circle]
  └─ Circle encryption   [chaves compartilhadas]

Bloco 3 (Flutter UI)    ──────────────────────────────►
  ├─ Onboarding          [criar identidade]
  ├─ Criar memória       [fotos/áudio/texto]
  ├─ Timeline            [feed próprio + amigos]
  └─ Builds Win/Mac      [empacotamento]

Bloco 4 (API)           ─────────►
  └─ GraphQL básico      [queries + mutations]
```
