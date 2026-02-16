+++
title = "Fase 4: Tuning de Performance"
date = 2026-02-15T20:00:00+00:00
description = "SQLite em modo WAL com configuracao centralizada de pragmas, cache LRU de fragmentos, gerenciamento de ciclo de vida do pool de conexoes QUIC e otimizacao do hot path de atestacao."
+++

Uma rede P2P que atravessa NATs mas engasga com seu proprio I/O nao serve de muito. A Fase 4 continua com tuning de performance: centralizacao da configuracao do banco de dados, cache de blobs de fragmentos em memoria, gerenciamento de ciclo de vida de conexoes QUIC e eliminacao de leituras desnecessarias de disco no hot path de atestacao.

O principio orientador foi o mesmo do resto do Tesseras: fazer a coisa mais simples que realmente funciona. Sem alocadores customizados, sem estruturas de dados lock-free, sem complexidade prematura. Um `StorageConfig` centralizado, um cache LRU, um reaper de conexoes e uma correcao pontual para evitar reler blobs que ja tinham checksum calculado.

## O que foi construido

**Configuracao SQLite centralizada** (`tesseras-storage/src/database.rs`) — Um novo struct `StorageConfig` e funcoes `open_database()` / `open_in_memory()` que aplicam todos os pragmas SQLite em um unico lugar: journal mode WAL, foreign keys, modo synchronous (NORMAL por padrao, FULL para hardware instavel como RPi + cartao SD), busy timeout, tamanho do cache de paginas e intervalo de autocheckpoint WAL. Anteriormente, cada ponto de chamada abria uma conexao e aplicava pragmas ad hoc. Agora o daemon, CLI e testes passam todos pelo mesmo caminho. 7 testes cobrindo foreign keys, busy timeout, journal mode, migracoes, modos synchronous e criacao de arquivos WAL em disco.

**Cache LRU de fragmentos** (`tesseras-storage/src/cache.rs`) — Um `CachedFragmentStore` que envolve qualquer `FragmentStore` com um cache LRU ciente de bytes. Blobs de fragmentos sao cacheados na leitura e invalidados na escrita ou exclusao. Quando o cache excede seu limite de bytes configurado, as entradas menos recentemente usadas sao removidas. O cache e transparente: ele proprio implementa `FragmentStore`, entao o resto da pilha nao sabe que esta la. Metricas Prometheus opcionais rastreiam hits, misses e uso atual de bytes. 3 testes: hit no cache evita leitura interna, store invalida cache, remocao quando excede bytes maximos.

**Metricas Prometheus de storage** (`tesseras-storage/src/metrics.rs`) — Um struct `StorageMetrics` com tres contadores/gauges: `fragment_cache_hits`, `fragment_cache_misses` e `fragment_cache_bytes`. Registrado no registry Prometheus e conectado ao cache de fragmentos via `with_metrics()`.

**Correcao do hot path de atestacao** (`tesseras-replication/src/service.rs`) — O fluxo de atestacao anteriormente lia cada blob de fragmento do disco e recalculava seu checksum BLAKE3. Como `list_fragments()` ja retorna `FragmentId` com um checksum armazenado, a correcao e trivial: usar `frag.checksum` ao inves de `blake3::hash(&data)`. Isso elimina uma leitura de disco por fragmento durante atestacao — para uma tessera com 100 fragmentos, sao 100 leituras a menos. Um teste com `expect_read_fragment().never()` verifica que nenhuma leitura de blob acontece durante atestacao.

**Ciclo de vida do pool de conexoes QUIC** (`tesseras-net/src/quinn_transport.rs`) — Um struct `PoolConfig` controlando maximo de conexoes, timeout de inatividade e intervalo do reaper. `PooledConnection` envolve cada `quinn::Connection` com um timestamp `last_used`. Quando o pool atinge capacidade maxima, a conexao inativa mais antiga e removida antes de abrir uma nova. Uma tarefa reaper em background (Tokio spawn) periodicamente fecha conexoes que ficaram inativas alem do timeout. 4 novas metricas de pool: `tesseras_conn_pool_size`, `pool_hits_total`, `pool_misses_total`, `pool_evictions_total`.

**Integracao no daemon** (`tesd/src/config.rs`, `main.rs`) — Uma nova secao `[performance]` na configuracao TOML com campos para tamanho de cache SQLite, modo synchronous, busy timeout, tamanho de cache de fragmentos, maximo de conexoes, timeout de inatividade e intervalo do reaper. O `main()` do daemon agora chama `open_database()` com o `StorageConfig` configurado, envolve `FsFragmentStore` com `CachedFragmentStore` e vincula QUIC com o `PoolConfig` configurado. A dependencia direta de `rusqlite` foi removida do crate do daemon.

**Migracao do CLI** (`tesseras-cli/src/commands/init.rs`, `create.rs`) — Ambos os comandos `init` e `create` agora usam `tesseras_storage::open_database()` com o `StorageConfig` padrao ao inves de abrir conexoes `rusqlite` diretamente. A dependencia de `rusqlite` foi removida do crate do CLI.

## Decisoes de arquitetura

- **Padrao decorator para cache**: `CachedFragmentStore` envolve `Box<dyn FragmentStore>` e implementa `FragmentStore` ele proprio. Isso significa que cache e opt-in, composavel e invisivel para consumidores. O daemon habilita; testes podem pular.
- **Remocao ciente de bytes**: o cache LRU rastreia bytes totais, nao contagem de entradas. Blobs de fragmentos variam muito em tamanho (um fragmento de texto de 4KB vs um shard de foto de 2MB), entao contar entradas daria uma visao enganosa do uso de memoria.
- **Sem crate de pool de conexoes**: ao inves de trazer uma biblioteca generica de pool, o pool de conexoes e um wrapper fino sobre `DashMap<SocketAddr, PooledConnection>` com um reaper Tokio. Conexoes QUIC sao multiplexadas, entao o "pool" e realmente sobre gerenciamento de ciclo de vida (limpeza de inativos, maximo de conexoes) e nao sobre emprestar/devolver.
- **Checksums armazenados ao inves de releituras**: a correcao de atestacao e intencionalmente minima — uma linha alterada, uma leitura de disco removida por fragmento. Os checksums ja estavam armazenados no SQLite por `store_fragment()`, apenas nao estavam sendo usados.
- **Configuracao centralizada de pragmas**: um unico struct `StorageConfig` substitui chamadas `PRAGMA` espalhadas. O flag `sqlite_synchronous_full` existe especificamente para implantacoes em Raspberry Pi onde o kernel pode crashar e perder transacoes WAL nao checkpointadas.

## O que vem a seguir

- **Fase 4 continuacao** — Shamir's Secret Sharing para herdeiros, tesseras seladas (criptografia time-lock), auditorias de seguranca, onboarding de nos institucionais, deduplicacao de storage, empacotamento para OS
- **Fase 5: Exploracao e Cultura** — navegador publico de tesseras por era/localizacao/tema/idioma, curadoria institucional, integracao genealogica, exportacao para midia fisica (M-DISC, microfilme, papel livre de acido com QR)

Com tuning de performance implementado, Tesseras lida com o caso comum de forma eficiente: leituras de fragmentos acertam o cache LRU, atestacao pula I/O de disco, conexoes QUIC inativas sao removidas automaticamente e o SQLite e configurado consistentemente em toda a pilha. Os proximos passos focam em funcionalidades criptograficas (Shamir, time-lock) e hardening para implantacao em producao.
