+++
title = "Fase 4: Furando NATs"
date = 2026-02-15T18:00:00+00:00
description = "Os nos Tesseras agora podem descobrir seu tipo de NAT via STUN, coordenar UDP hole punching atraves de introdutores e usar relay transparente quando a conectividade direta falha."
+++

A maioria dos dispositivos das pessoas ficam atras de um NAT — um tradutor de enderecos de rede que permite acessar a internet mas impede conexoes de entrada. Para uma rede P2P, isso e um problema existencial: se dois nos atras de NATs nao conseguem se comunicar, a rede se fragmenta. A Fase 4 continua com uma pilha completa de travessia de NAT: descoberta via STUN, hole punching coordenado e fallback por relay.

A abordagem segue o mesmo padrao da maioria dos sistemas P2P consolidados (WebRTC, BitTorrent, IPFS): tente a opcao mais barata primeiro, escale apenas quando necessario. Conectividade direta nao custa nada. Hole punching custa alguns pacotes coordenados. Relay custa largura de banda sustentada de um terceiro. Tesseras tenta nessa ordem.

## O que foi construido

**Classificacao NatType** (`tesseras-core/src/network.rs`) — Um novo enum `NatType` (Public, Cone, Symmetric, Unknown) adicionado a camada de dominio core. Esse tipo e compartilhado por toda a pilha: o cliente STUN o escreve, o DHT o divulga em mensagens Pong, e o coordenador de punch o le para decidir se hole punching vale a pena tentar (Cone-para-Cone funciona ~80% das vezes; Symmetric-para-Symmetric quase nunca funciona).

**Cliente STUN** (`tesseras-net/src/stun.rs`) — Uma implementacao STUN minima (RFC 5389 Binding Request/Response) que descobre o endereco externo de um no. O codec codifica requisicoes de 20 bytes com um ID de transacao aleatorio e decodifica respostas XOR-MAPPED-ADDRESS. A funcao `discover_nat()` consulta multiplos servidores STUN em paralelo (Google, Cloudflare por padrao), compara os enderecos mapeados e classifica o tipo de NAT:

- Mesmo IP e porta de todos os servidores → **Public** (sem NAT)
- Mesmo endereco mapeado de todos os servidores → **Cone** (hole punching funciona)
- Enderecos mapeados diferentes → **Symmetric** (hole punching nao confiavel)
- Sem respostas → **Unknown**

Retentativas com backoff exponencial e timeouts configuraveis. 12 testes cobrindo roundtrips de codec, todos os caminhos de classificacao e consultas async em loopback.

**Coordenacao de punch assinada** (`tesseras-net/src/punch.rs`) — Assinatura e verificacao Ed25519 para mensagens `PunchIntro`, `RelayRequest` e `RelayMigrate`. Cada introducao e assinada pelo iniciador com uma janela de timestamp de 30 segundos, prevenindo ataques de reflexao (onde um atacante reproduz uma introducao antiga para redirecionar trafego). O formato do payload e `target || external_addr || timestamp` — alterar qualquer campo invalida a assinatura. 6 testes unitarios mais 3 testes baseados em propriedades com proptest (IDs de no, portas e tokens de sessao arbitrarios).

**Gerenciador de sessoes de relay** (`tesseras-net/src/relay.rs`) — Gerencia sessoes de relay UDP transparente entre nos com NAT. Cada sessao tem um token aleatorio de 16 bytes; os nos prefixam seus pacotes com o token, o relay remove e encaminha. Funcionalidades:

- Encaminhamento bidirecional (A→R→B e B→R→A)
- Limite de taxa: 256 KB/s para nos reciprocos, 64 KB/s para nao reciprocos
- Duracao maxima de 10 minutos para sessoes bootstrap (nao reciprocas)
- Migracao de endereco: quando o IP de um no muda (Wi-Fi para celular), um `RelayMigrate` assinado atualiza a sessao sem derruba-la
- Limpeza por inatividade com timeout configuravel
- 8 testes unitarios mais 2 testes baseados em propriedades

**Extensoes de mensagens DHT** (`tesseras-dht/src/message.rs`) — Sete novas variantes de mensagem adicionadas ao protocolo DHT:

| Mensagem | Proposito |
|----------|-----------|
| `PunchIntro` | "Quero conectar ao no X, aqui esta meu endereco externo assinado" |
| `PunchRequest` | O introdutor encaminha a requisicao ao destino |
| `PunchReady` | O destino confirma prontidao, envia seu endereco externo |
| `RelayRequest` | "Crie uma sessao de relay para o no X" |
| `RelayOffer` | O relay responde com seu endereco e token de sessao |
| `RelayClose` | Encerrar uma sessao de relay |
| `RelayMigrate` | Atualizar sessao apos mudanca de rede |

A mensagem `Pong` foi estendida com metadados NAT: `nat_type`, `relay_slots_available` e `relay_bandwidth_used_kbps`. Todos os novos campos usam `#[serde(default)]` para compatibilidade retroativa — nos antigos ignoram o que nao reconhecem, nos novos usam defaults. 9 novos testes de roundtrip de serializacao.

**Trait NatHandler e dispatch** (`tesseras-dht/src/engine.rs`) — Uma nova trait async `NatHandler` (5 metodos) injetada no engine DHT, seguindo o mesmo padrao de injecao de dependencia do `ReplicationHandler` existente. O loop de dispatch de mensagens do engine agora roteia todas as mensagens punch/relay para o handler. Isso mantem o engine DHT agnóstico ao protocolo enquanto permite que a logica de travessia de NAT viva em `tesseras-net`.

**Tipos de reconexao mobile** (`tesseras-embedded/src/reconnect.rs`) — Uma maquina de estados de reconexao em tres fases para dispositivos moveis:

1. **QuicMigration** (0-2s) — tenta migracao de conexao QUIC para todos os peers ativos
2. **ReStun** (2-5s) — redescobre endereco externo via STUN
3. **ReEstablish** (5-10s) — reconecta peers que a migracao nao conseguiu salvar

Peers sao reconectados em ordem de prioridade: nos bootstrap primeiro, depois nos que guardam nossos fragmentos, depois nos cujos fragmentos guardamos, depois vizinhos DHT gerais. Uma nova variante de evento `NetworkChanged` foi adicionada ao stream de eventos FFI para que o app Flutter possa mostrar progresso de reconexao.

**Configuracao NAT do daemon** (`tesseras-daemon/src/config.rs`) — Uma nova secao `[nat]` na configuracao TOML com lista de servidores STUN, toggle de relay, maximo de sessoes relay, limites de largura de banda (reciproco vs bootstrap) e timeout de inatividade. Todos os campos tem defaults sensiveis; relay e desabilitado por padrao.

**Metricas Prometheus** (`tesseras-net/src/metrics.rs`) — 16 metricas em quatro subsistemas:

- **STUN**: requisicoes, falhas, histograma de latencia
- **Punch**: tentativas/sucessos/falhas (por par de tipo NAT), histograma de latencia
- **Relay**: sessoes ativas, sessoes totais, bytes encaminhados, timeouts por inatividade, hits de rate limit
- **Reconexao**: mudancas de rede, tentativas/sucessos por fase, histograma de duracao

6 testes verificando registro, incremento, cardinalidade de labels e deteccao de registro duplo.

**Testes de integracao** — Dois testes end-to-end usando `MemTransport` (rede simulada em memoria):

- `punch_integration.rs` — Fluxo completo de hole-punch com 3 nos: A envia `PunchIntro` assinado ao introdutor I, I verifica e encaminha `PunchRequest` a B, B verifica a assinatura original e envia `PunchReady` de volta, A e B trocam mensagens diretamente. Tambem testa que uma assinatura invalida e corretamente rejeitada.
- `relay_integration.rs` — Fluxo completo de relay com 3 nos: A solicita relay de R, R cria sessao e envia `RelayOffer` a ambos os peers, A e B trocam pacotes prefixados com token atraves de R, A migra para um novo endereco no meio da sessao, A fecha a sessao, e o teste verifica que a sessao e encerrada e encaminhamento posterior falha.

**Testes de propriedade** — 7 testes baseados em proptest cobrindo: roundtrips de assinatura para todos os tres tipos de mensagem assinada (IDs de no, portas e tokens arbitrarios), determinismo de classificacao NAT (mesmas entradas sempre produzem mesma saida), validade de binding request STUN, unicidade de tokens de sessao, e rejeicao de pacotes curtos pelo relay.

**Alvos Justfile** — `just test-nat` executa todos os testes de travessia NAT em `tesseras-net` e `tesseras-dht`. `just test-chaos` e um placeholder para futuros testes de caos com Docker Compose e `tc netem`.

## Decisoes de arquitetura

- **STUN ao inves de TURN**: implementamos STUN (descoberta) e relay customizado ao inves de TURN completo. TURN requer alocacao autenticada e foi projetado para relay de midia; nosso relay e mais simples — encaminhamento UDP com prefixo de token e limites de taxa. Isso mantem o protocolo minimo e evita depender de servidores TURN externos.
- **Assinaturas em introducoes**: cada `PunchIntro` e assinado pelo iniciador. Sem isso, um atacante poderia enviar introducoes forjadas para redirecionar as tentativas de hole-punch de um no para um endereco controlado pelo atacante (ataque de reflexao). A janela de timestamp de 30 segundos limita replay.
- **Tiers reciprocos de largura de banda**: nos relay dao 4x mais largura de banda (256 vs 64 KB/s) para peers com boas pontuacoes de reciprocidade. Isso incentiva nos a armazenar fragmentos para outros — se voce contribui, recebe melhor servico de relay quando precisa.
- **Extensao Pong retrocompativel**: novos campos NAT em `Pong` usam `#[serde(default)]` e `Option<T>`. Nos antigos que nao entendem esses campos simplesmente os pulam durante deserializacao. Nenhum bump de versao de protocolo necessario.
- **NatHandler como trait async**: a logica de travessia NAT e injetada no engine DHT via trait, assim como `ReplicationHandler`. Isso mantem o engine DHT focado em roteamento e gerenciamento de peers, e permite que a implementacao NAT seja trocada ou desabilitada sem tocar no codigo core do DHT.

## O que vem a seguir

- **Fase 4 continuacao** — tuning de performance (pooling de conexoes, cache de fragmentos, SQLite WAL), auditorias de seguranca, onboarding de nos institucionais, empacotamento para OS
- **Fase 5: Exploracao e Cultura** — navegador publico de tesseras por era/localizacao/tema/idioma, curadoria institucional, integracao genealogica, exportacao para midia fisica (M-DISC, microfilme, papel livre de acido com QR)

Com travessia de NAT, Tesseras pode conectar nos independentemente de sua topologia de rede. Nos publicos conversam diretamente. Nos com NAT Cone furam com ajuda de um introdutor. Nos com NAT Symmetric ou firewalled usam relay atraves de peers voluntarios. A rede se adapta ao mundo real, onde a maioria dos dispositivos esta atras de um NAT e as condicoes de rede mudam constantemente.
