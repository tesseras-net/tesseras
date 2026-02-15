+++
title = "Fase 3: Memórias nas Suas Mãos"
date = 2026-02-14T14:00:00+00:00
description = "Tesseras agora tem um app Flutter e um nó Rust embarcado — qualquer pessoa pode criar e preservar memórias pelo celular."
+++

As pessoas agora podem segurar suas memórias nas próprias mãos. A Fase 3 entrega o que as fases anteriores construíram: um app mobile onde alguém baixa o Tesseras, cria uma identidade, tira uma foto, e aquela memória entra na rede de preservação. Sem contas na nuvem, sem assinaturas, sem nenhuma empresa entre você e suas memórias.

## O que foi construído

**tesseras-embedded** — Um nó P2P completo que roda dentro de um app mobile. A struct `EmbeddedNode` é dona de um runtime Tokio, banco SQLite, transporte QUIC, engine Kademlia DHT, serviço de replicação e serviço de tessera — a mesma stack do daemon desktop, compilada como biblioteca compartilhada. Um padrão singleton global (`Mutex<Option<EmbeddedNode>>`) garante um único nó por ciclo de vida do app. Ao iniciar, ele abre o banco de dados, executa migrações, carrega ou gera uma identidade Ed25519 com proof-of-work para o node ID, faz bind QUIC numa porta efêmera, conecta DHT e replicação, e inicia o loop de reparo. Ao parar, envia um sinal de shutdown e drena graciosamente.

Onze funções FFI são expostas para Dart via flutter_rust_bridge: ciclo de vida (`node_start`, `node_stop`, `node_is_running`), identidade (`create_identity`, `get_identity`), memórias (`create_memory`, `get_timeline`, `get_memory`) e status da rede (`get_network_stats`, `get_replication_status`). Todos os tipos que cruzam a fronteira FFI são structs planas com apenas `String`, `Option<String>`, `Vec<String>` e primitivos — sem trait objects, sem generics, sem lifetimes.

Quatro módulos adaptadores fazem a ponte entre as ports do core e as implementações concretas: `Blake3HasherAdapter`, `Ed25519SignerAdapter`/`Ed25519VerifierAdapter` para criptografia, `DhtPortAdapter` para operações DHT, e `ReplicationHandlerAdapter` para RPCs de fragmentos e atestação recebidos.

A feature flag `bundled-sqlite` compila o SQLite a partir do código-fonte, necessário para Android e iOS onde a biblioteca do sistema pode não estar disponível. A configuração do Cargokit passa essa flag automaticamente em builds de debug e release.

**App Flutter** — Uma aplicação Material Design 3 com gerenciamento de estado Riverpod, direcionada para Android, iOS, Linux, macOS e Windows a partir de uma única base de código.

O *fluxo de onboarding* são três telas: uma tela de boas-vindas explicando o projeto em uma frase ("Preserve suas memórias através dos milênios. Sem nuvem. Sem empresa."), uma tela de criação de identidade que dispara a geração do par de chaves Ed25519 em Rust, e uma tela de confirmação mostrando o nome do usuário e a identidade criptográfica.

A *tela de timeline* exibe memórias em ordem cronológica reversa com previews de imagem, texto de contexto e chips para tipo de memória e visibilidade. Pull-to-refresh recarrega a partir do nó Rust. Um floating action button abre a *tela de criação de memória*, que suporta seleção de foto da galeria ou câmera via `image_picker`, texto de contexto opcional, dropdowns de tipo de memória e visibilidade, e tags separadas por vírgula. Criar uma memória chama o FFI Rust sincronamente, depois retorna à timeline.

A *tela de rede* mostra dois cards: status do nó (contagem de peers, tamanho da DHT, estado de bootstrap, uptime) e saúde da replicação (total de fragmentos, fragmentos saudáveis, fragmentos em reparo, fator de replicação). A *tela de configurações* exibe a identidade do usuário — nome, node ID truncado, chave pública truncada e data de criação.

Três providers Riverpod gerenciam o estado: `nodeProvider` inicia o nó embarcado ao abrir o app usando o diretório de documentos e para ao fazer dispose; `identityProvider` carrega o perfil existente ou cria um novo; `timelineProvider` busca a lista de memórias com paginação.

**Testes** — 9 testes unitários Rust em tesseras-embedded cobrindo ciclo de vida do nó (start/stop sem panic), persistência de identidade entre reinícios, ciclos de reinício sem corrupção do SQLite, streaming de eventos de rede, recuperação de estatísticas, criação de memória e recuperação da timeline, e busca de memória individual por hash. 2 testes Flutter: um teste de integração verificando inicialização do Rust e startup do app, e um smoke test de widget.

## Decisões de arquitetura

- **Nó embarcado, não cliente-servidor**: o celular roda a stack P2P completa, não um thin client conversando com um daemon remoto. Isso significa que memórias são preservadas mesmo sem internet. Usuários com um Raspberry Pi ou VPS podem opcionalmente conectar o app ao seu daemon via GraphQL para maior disponibilidade, mas não é obrigatório.
- **FFI síncrono**: todas as funções flutter_rust_bridge são marcadas como `#[frb(sync)]` e bloqueiam no runtime Tokio interno. Isso simplifica o lado Dart (sem complexidade de bridge assíncrono) enquanto o lado Rust lida com concorrência internamente. A UI thread do Flutter permanece responsiva porque o Riverpod envolve as chamadas em providers assíncronos.
- **Singleton global**: um global `Mutex<Option<EmbeddedNode>>` garante que o ciclo de vida do nó seja previsível — um start, um stop, sem race conditions. Plataformas mobile matam processos agressivamente, então simplicidade no gerenciamento de ciclo de vida é uma feature.
- **Tipos FFI planos**: nenhuma abstração Rust vaza pela fronteira FFI. Todo tipo é uma struct plana com strings e números. Isso torna os bindings Dart auto-gerados confiáveis e fáceis de debugar.
- **Onboarding de três telas**: a criação de identidade é o único passo obrigatório. Sem email, sem senha, sem registro em servidor. O app gera uma identidade criptográfica localmente e está pronto para uso.

## O que vem a seguir

- **Fase 4: Resiliência e Escala** — NAT traversal avançado (STUN/TURN), Shamir's Secret Sharing para herdeiros, tesseras seladas com criptografia temporal, ajuste de performance, auditorias de segurança, empacotamento para Alpine/Arch/Debian/FreeBSD/OpenBSD
- **Fase 5: Exploração e Cultura** — Navegador público de tesseras por era/localização/tema/idioma, curadoria institucional, integração com genealogia, exportação para mídia física (M-DISC, microfilme, papel livre de ácido com QR)

A infraestrutura está completa. A rede existe, a replicação funciona, e agora qualquer pessoa com um celular pode participar. O que resta é fortalecer o que temos e abrir para o mundo.
