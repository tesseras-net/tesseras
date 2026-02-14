+++
title = "Fase 1: Nós Se Encontram"
date = 2026-02-14
description = "Os nós do Tesseras agora descobrem pares, formam uma DHT Kademlia sobre QUIC e publicam e encontram ponteiros de tesseras pela rede."
+++

Tesseras não é mais uma ferramenta apenas local. A Fase 1 entrega a camada de rede: nós se descobrem através de uma DHT Kademlia, comunicam-se sobre QUIC e publicam ponteiros de tesseras que qualquer par na rede pode encontrar. Uma tessera criada no nó A agora pode ser encontrada a partir do nó C.

## O que foi construído

**tesseras-core** (atualizado) — Novos tipos de domínio de rede: `TesseraPointer` (referência leve aos detentores de uma tessera e localização dos fragmentos), `NodeIdentity` (ID do nó + chave pública + nonce de prova de trabalho), `NodeInfo` (identidade + endereço + capacidades) e `Capabilities` (bitflags do que um nó suporta: DHT, armazenamento, relay, replicação).

**tesseras-net** — A camada de transporte, construída sobre QUIC via quinn. A trait `Transport` define a porta: `send`, `recv`, `disconnect`, `local_addr`. Dois adaptadores a implementam:

- `QuinnTransport` — QUIC real com TLS auto-assinado, negociação ALPN (`tesseras/1`), pool de conexões via DashMap e um loop de aceitação em background que trata streams recebidas.
- `MemTransport` + `SimNetwork` — canais em memória para testes determinísticos sem I/O de rede. Cada teste de integração no crate DHT roda contra este adaptador.

O protocolo de fio usa MessagePack com prefixo de comprimento: um cabeçalho de 4 bytes big-endian seguido de um payload rmp-serde. `WireMessage` carrega um byte de versão, ID de requisição e um corpo que pode ser requisição, resposta ou erro de protocolo. Tamanho máximo de mensagem é 64 KiB.

**tesseras-dht** — Uma implementação completa de Kademlia:

- *Tabela de roteamento*: 160 k-buckets com k=20. Evicção do menos recentemente visto, mover-para-trás ao atualizar, verificação por ping antes de substituir a entrada mais antiga de um bucket cheio.
- *Distância XOR*: métrica XOR de 160 bits com indexação de bucket pelo bit mais significativo diferente.
- *Prova de trabalho*: nós iteram um nonce até que `BLAKE3(pubkey || nonce)[..20]` tenha 8 bits zero iniciais (~256 tentativas de hash em média). Barato o suficiente para qualquer dispositivo, caro o suficiente para tornar ataques Sybil impraticáveis em escala.
- *Mensagens de protocolo*: Ping/Pong, FindNode/FindNodeResponse, FindValue/FindValueResult, Store — todos serializados com MessagePack via serde.
- *Armazenamento de ponteiros*: armazenamento em memória limitado com TTL configurável (24 horas padrão) e máximo de entradas (10.000 padrão). Quando cheio, remove ponteiros mais distantes do ID do nó local, seguindo o modelo de responsabilidade baseado em distância do Kademlia.
- *DhtEngine*: o orquestrador principal. Trata RPCs recebidos, executa buscas iterativas (paralelismo alpha=3), bootstrap, publicação e busca. O método `run()` dirige um loop `tokio::select!` com timers de manutenção: refresh da tabela de roteamento a cada 60 segundos, expiração de ponteiros a cada 5 minutos.

**tesseras-daemon** — Um binário de nó completo. Analisa argumentos de CLI (endereço de bind, pares de bootstrap, diretório de dados), gera uma identidade de nó válida por PoW, abre um endpoint QUIC, faz bootstrap na rede e roda o motor DHT. Desligamento gracioso com Ctrl+C via tratamento de sinais do tokio.

**Infraestrutura** — Configuração OpenTofu para dois nós bootstrap no Hetzner Cloud (instâncias cx22 em Falkenstein, Alemanha e Helsinki, Finlândia). Script de provisionamento cloud-init cria um usuário dedicado `tesseras`, escreve um arquivo de configuração e configura um serviço systemd. Regras de firewall abrem UDP 4433 (QUIC) e restringem métricas a acesso interno.

**Testes** — 139 testes em todo o workspace:

- 47 testes unitários em tesseras-dht (tabela de roteamento, distância, PoW, armazenamento de ponteiros, serialização de mensagens, RPCs do engine)
- 5 testes de integração multi-nó (bootstrap de 3 nós, convergência de lookup com 10 nós, publicar-e-encontrar, detecção de partida de nó, rejeição de PoW)
- 14 testes em tesseras-net (roundtrips de codec, send/recv de transporte, backpressure, disconnect)
- Testes de fumaça com Docker Compose usando 3 nós containerizados comunicando sobre QUIC real
- Zero avisos do clippy, formatação limpa

## Decisões de arquitetura

- **Transport como porta**: a trait `Transport` é a única interface entre o motor DHT e a rede. Trocar QUIC por qualquer outro protocolo significa implementar quatro métodos. Todos os testes de DHT usam o adaptador em memória, tornando-os rápidos e determinísticos.
- **Um stream por RPC**: cada par requisição-resposta DHT usa um stream bidirecional QUIC novo. Sem complexidade de multiplexação, sem bloqueio head-of-line entre operações independentes. O QUIC trata a multiplexação no nível da conexão.
- **MessagePack em vez de Protobuf**: codificação binária compacta sem geração de código ou arquivos de esquema. Integração com serde significa que adicionar um campo a uma mensagem é uma mudança de uma linha. Trade-off: sem garantias de evolução de esquema embutidas, mas neste estágio velocidade importa mais.
- **PoW em vez de stake ou reputação**: uma identidade de nó custa ~256 hashes BLAKE3. Isso roda em menos de um segundo em qualquer hardware, incluindo um Raspberry Pi, mas gerar milhares de identidades para um ataque Sybil se torna caro. Sem tokens, sem blockchain, sem dependências externas.
- **Busca iterativa com atualização da tabela de roteamento**: nós descobertos são adicionados à tabela de roteamento conforme encontrados durante buscas iterativas, seguindo o comportamento padrão do Kademlia. Isso garante que a tabela de roteamento melhore organicamente conforme os nós interagem.

## O que vem a seguir

- **Fase 2: Replicação** — Codificação de apagamento Reed-Solomon pela rede, distribuição de fragmentos, loops de reparo automáticos, livro-razão de reciprocidade bilateral (sem blockchain, sem tokens)
- **Fase 3: API e Apps** — App Flutter mobile/desktop via flutter_rust_bridge, API GraphQL (async-graphql), nó WASM no navegador
- **Fase 4: Resiliência e Escala** — Assinaturas pós-quânticas ML-DSA, travessia avançada de NAT, Compartilhamento de Segredo de Shamir para herdeiros, empacotamento para Alpine/Arch/Debian/FreeBSD/OpenBSD, CI no SourceHut
- **Fase 5: Exploração e Cultura** — navegador público de tesseras, curadoria institucional, integração genealógica, exportação para mídia física

Os nós conseguem se encontrar. Em seguida, aprendem a manter vivas as memórias uns dos outros.
