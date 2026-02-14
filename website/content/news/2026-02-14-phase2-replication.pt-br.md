+++
title = "Fase 2: Memórias Sobrevivem"
date = 2026-02-14T12:00:00+00:00
description = "Tesseras agora fragmenta, distribui e repara dados automaticamente pela rede usando codificação de apagamento Reed-Solomon e um livro-razão de reciprocidade bilateral."
+++

Uma tessera não está mais presa a uma única máquina. A Fase 2 entrega a camada de replicação: os dados são divididos em fragmentos com codificação de apagamento, distribuídos entre múltiplos pares e reparados automaticamente quando nós ficam offline. Um livro-razão de reciprocidade bilateral garante troca justa de armazenamento — sem blockchain, sem tokens.

## O que foi construído

**tesseras-core** (atualizado) — Novos tipos de domínio de replicação: `FragmentPlan` (seleciona a camada de fragmentação baseada no tamanho da tessera), `FragmentId` (hash da tessera + índice + contagem de shards + checksum), `FragmentEnvelope` (fragmento com seus metadados para transporte na rede), `FragmentationTier` (Small/Medium/Large), `Attestation` (prova de que um nó possui um fragmento em um dado momento) e `ReplicateAck` (confirmação de recebimento de fragmento). Três novas traits de porta definem os limites hexagonais: `DhtPort` (encontrar pares, replicar fragmentos, solicitar atestações, ping), `FragmentStore` (armazenar/ler/deletar/listar/verificar fragmentos) e `ReciprocityLedger` (registrar trocas de armazenamento, consultar saldos, encontrar melhores pares). O tamanho máximo de uma tessera é 1 GB.

**tesseras-crypto** (atualizado) — O `ReedSolomonCoder` existente agora alimenta a codificação de fragmentos. Os dados são divididos em shards, shards de paridade são computados, e qualquer combinação de shards de dados pode reconstruir o original — desde que o número de shards ausentes não exceda a contagem de paridade.

**tesseras-storage** (atualizado) — Dois novos adaptadores:

- `FsFragmentStore` — armazena dados de fragmentos como arquivos em disco (`{raiz}/{hash_tessera}/{indice:03}.shard`) com um índice de metadados SQLite rastreando hash da tessera, índice do shard, contagem de shards, checksum e tamanho em bytes. A verificação recalcula o hash BLAKE3 e compara com o checksum armazenado.
- `SqliteReciprocityLedger` — contabilidade bilateral de armazenamento em SQLite. Cada par tem uma linha rastreando bytes armazenados para eles e bytes que eles armazenam para nós. A coluna `balance` é uma coluna gerada (`bytes_they_store_for_us - bytes_stored_for_them`). UPSERT garante incremento atômico dos contadores.

Nova migração (`002_replication.sql`) adiciona tabelas para fragmentos, planos de fragmentação, detentores, mapeamentos detentor-fragmento e saldos de reciprocidade.

**tesseras-dht** (atualizado) — Quatro novas variantes de mensagem: `Replicate` (enviar um envelope de fragmento), `ReplicateAck` (confirmar recebimento), `AttestRequest` (pedir a um nó que prove que possui os fragmentos de uma tessera) e `AttestResponse` (retornar atestação com checksums e timestamp). O engine trata essas mensagens em seu loop de despacho.

**tesseras-replication** — O novo crate, com cinco módulos:

- *Codificação de fragmentos* (`fragment.rs`): `encode_tessera()` seleciona a camada de fragmentação baseada no tamanho e então chama a codificação Reed-Solomon para as camadas Medium e Large. Três camadas:
  - **Small** (< 4 MB): replicação do arquivo inteiro para r=7 pares, sem codificação de apagamento
  - **Medium** (4–256 MB): 16 shards de dados + 8 de paridade, distribuídos entre r=7 pares
  - **Large** (≥ 256 MB): 48 shards de dados + 24 de paridade, distribuídos entre r=7 pares

- *Distribuição* (`distributor.rs`): filtragem de diversidade de sub-rede limita pares por sub-rede /24 IPv4 (ou prefixo /48 IPv6) para evitar falhas correlacionadas. Se todos os seus fragmentos caírem no mesmo rack, uma única queda de energia os elimina.

- *Serviço* (`service.rs`): `ReplicationService` é o orquestrador. `replicate_tessera()` codifica os dados, encontra os pares mais próximos via DHT, aplica diversidade de sub-rede e distribui fragmentos em round-robin. `receive_fragment()` valida o checksum BLAKE3, verifica o saldo de reciprocidade (rejeita se o déficit do remetente exceder o limite configurado), armazena o fragmento e atualiza o livro-razão. `handle_attestation_request()` lista os fragmentos locais e calcula seus checksums como prova de posse.

- *Reparo* (`repair.rs`): `check_tessera_health()` solicita atestações dos detentores conhecidos, recorre ao ping para nós não responsivos, verifica a integridade local dos fragmentos e retorna uma de três ações: `Healthy`, `NeedsReplication { deficit }` ou `CorruptLocal { fragment_index }`. O loop de reparo roda a cada 24 horas (com 2 horas de jitter) via `tokio::select!` com integração de desligamento.

- *Configuração* (`config.rs`): `ReplicationConfig` com padrões para intervalo de reparo (24h), jitter (2h), transferências simultâneas (4), espaço livre mínimo (1 GB), tolerância de déficit (256 MB) e limite de armazenamento por par (1 GB).

**tesseras-daemon** (atualizado) — O daemon agora abre um banco de dados SQLite (`db/tesseras.db`), executa migrações, cria instâncias de `FsFragmentStore`, `SqliteReciprocityLedger` e `FsBlobStore`, envolve o engine DHT em um `DhtPortAdapter`, constrói um `ReplicationService` e lança o loop de reparo como tarefa em segundo plano com desligamento gracioso.

**Testes** — 193 testes em todo o workspace:

- 15 testes unitários em tesseras-replication (camadas de codificação de fragmentos, validação de checksum, diversidade de sub-rede, verificações de saúde do reparo, fluxos de recebimento/replicação do serviço)
- 3 testes de integração com armazenamento real (ciclo completo codificar→distribuir→receber para tessera média, replicação de arquivo inteiro para tessera pequena, rejeição de fragmento adulterado)
- Testes usam SQLite em memória + diretório temporário para fragmentos com mocks mockall para DHT e BlobStore
- Zero avisos do clippy, formatação limpa

## Decisões de arquitetura

- **Fragmentação em três camadas**: arquivos pequenos não precisam de codificação de apagamento — o overhead não compensa. Arquivos médios e grandes recebem progressivamente mais shards de paridade. Isso evita desperdiçar armazenamento em tesseras pequenas enquanto oferece redundância forte para as grandes.
- **Distribuição por push do dono**: o dono da tessera codifica os fragmentos e os envia aos pares, em vez dos pares puxarem. Isso simplifica o protocolo (sem fase de negociação) e garante que os fragmentos são distribuídos imediatamente.
- **Reciprocidade bilateral sem consenso**: cada nó rastreia seu próprio saldo com cada par localmente. Sem livro-razão global, sem token, sem blockchain. Se o par A armazena 500 MB para o par B, o par B deveria armazenar aproximadamente 500 MB para o par A. Free riders perdem redundância gradualmente — seus fragmentos são despriorizados para reparo, mas nunca deletados.
- **Diversidade de sub-rede**: os fragmentos são espalhados por diferentes sub-redes para sobreviver a falhas correlacionadas. Uma queda de datacenter não deveria eliminar todas as cópias de uma tessera.
- **Verificações de saúde por atestação primeiro**: o loop de reparo pede aos detentores que provem posse (atestação com checksums) antes de declarar uma tessera degradada. Apenas quando a atestação falha é que ele recorre a um simples ping. Isso detecta corrupção silenciosa de dados, não apenas partida de nós.

## O que vem a seguir

- **Fase 3: API e Apps** — App Flutter mobile/desktop via flutter_rust_bridge, API GraphQL (async-graphql), nó WASM no navegador
- **Fase 4: Resiliência e Escala** — Assinaturas pós-quânticas ML-DSA, travessia avançada de NAT, Compartilhamento de Segredo de Shamir para herdeiros, empacotamento para Alpine/Arch/Debian/FreeBSD/OpenBSD, CI no SourceHut
- **Fase 5: Exploração e Cultura** — navegador público de tesseras, curadoria institucional, integração genealógica, exportação para mídia física

Os nós conseguem se encontrar e manter vivas as memórias uns dos outros. Em seguida, damos às pessoas uma forma de segurar suas memórias nas mãos.
