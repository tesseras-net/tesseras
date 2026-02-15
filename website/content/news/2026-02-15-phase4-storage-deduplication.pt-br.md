+++
title = "Fase 4: Deduplicacao de Armazenamento"
date = 2026-02-15T23:00:00+00:00
description = "Uma nova camada de armazenamento enderecavel por conteudo elimina dados duplicados entre tesseras, reduzindo uso de disco e habilitando coleta de lixo automatica."
+++

Quando multiplas tesseras compartilham a mesma foto, o mesmo clipe de audio ou os mesmos dados de fragmento, a camada de armazenamento antiga mantinha copias separadas de cada. Em um no armazenando milhares de tesseras para a rede, essa duplicacao se acumula rapidamente. A Fase 4 continua com deduplicacao de armazenamento: um armazenamento enderecavel por conteudo (CAS) que garante que cada dado unico seja armazenado exatamente uma vez em disco, independentemente de quantas tesseras o referenciam.

O design e simples e comprovado: hash do conteudo com BLAKE3, usar o hash como nome do arquivo e manter uma contagem de referencias no SQLite. Quando duas tesseras incluem a mesma foto de 5 MB, um arquivo existe em disco com refcount 2. Quando uma tessera e deletada, o refcount cai para 1 e o arquivo permanece. Quando a ultima referencia e liberada, uma varredura periodica limpa o orfao.

## O que foi construido

**Migracao do esquema CAS** (`tesseras-storage/migrations/004_dedup.sql`) — Tres novas tabelas:

- `cas_objects` — rastreia cada objeto no armazenamento: hash BLAKE3 (chave primaria), tamanho em bytes, contagem de referencias e timestamp de criacao
- `blob_refs` — mapeia identificadores logicos de blobs (hash da tessera + hash da memoria + nome do arquivo) para hashes CAS, substituindo a convencao antiga de caminhos no sistema de arquivos
- `fragment_refs` — mapeia identificadores logicos de fragmentos (hash da tessera + indice do fragmento) para hashes CAS, substituindo o antigo layout do diretorio `fragments/`

Indices nas colunas de hash garantem lookups O(1) durante leituras e contagem de referencias.

**CasStore** (`tesseras-storage/src/cas.rs`) — O motor central de armazenamento enderecavel por conteudo. Arquivos sao armazenados sob um diretorio de prefixo de dois niveis: `<raiz>/<prefixo-hex-2-chars>/<hash-completo>.blob`. O armazenamento fornece cinco operacoes:

- `put(hash, data)` — escreve dados em disco se ainda nao presente, incrementa o refcount. Retorna se ocorreu um hit de deduplicacao.
- `get(hash)` — le dados do disco pelo hash
- `release(hash)` — decrementa o refcount. Se chegar a zero, o arquivo em disco e deletado imediatamente.
- `contains(hash)` — verifica existencia sem ler
- `ref_count(hash)` — retorna a contagem de referencias atual

Todas as operacoes sao atomicas dentro de uma unica transacao SQLite. O refcount e a fonte de verdade — se o refcount diz que o objeto existe, o arquivo deve estar em disco.

**FsBlobStore com CAS** (`tesseras-storage/src/blob.rs`) — Reescrito para delegar todo armazenamento ao CAS. Quando um blob e escrito, seu hash BLAKE3 e computado e passado para `cas.put()`. Uma linha em `blob_refs` mapeia o caminho logico (tessera + memoria + arquivo) para o hash CAS. Leituras buscam o hash CAS via `blob_refs` e leem de `cas.get()`. Deletar uma tessera libera todas as suas referencias de blob em uma unica transacao.

**FsFragmentStore com CAS** (`tesseras-storage/src/fragment.rs`) — Mesmo padrao para fragmentos codificados com erasure coding. O checksum BLAKE3 de cada fragmento ja e computado durante a codificacao Reed-Solomon, entao e usado diretamente como chave CAS. A verificacao de fragmentos agora checa o hash CAS ao inves de recomputar do zero — se o CAS diz que os dados estao intactos, estao.

**Coletor de lixo sweep** (`cas.rs:sweep()`) — Uma passagem periodica de GC que trata tres casos limite que o caminho normal de refcount nao consegue:

1. **Arquivos orfaos** — arquivos em disco sem linha correspondente em `cas_objects`. Pode acontecer apos um crash durante escrita. Arquivos com menos de 1 hora sao pulados (periodo de graca para escritas em andamento); orfaos mais antigos sao deletados.
2. **Refcounts vazados** — linhas em `cas_objects` com refcount zero que nao foram limpas (ex: se o processo morreu entre decrementar e deletar). Essas linhas sao removidas.
3. **Idempotente** — executar sweep duas vezes produz o mesmo resultado.

O sweep e conectado ao loop de reparo existente em `tesseras-replication`, entao roda automaticamente a cada 24 horas junto com as verificacoes de saude dos fragmentos.

**Migracao do layout antigo** (`tesseras-storage/src/migration.rs`) — Uma estrategia de migracao copy-first que move dados do layout antigo baseado em diretorios (`blobs/<tessera>/<memoria>/<arquivo>` e `fragments/<tessera>/<indice>.shard`) para o CAS. A migracao:

1. Verifica a versao de armazenamento em `storage_meta` (versao 1 = layout antigo, versao 2 = CAS)
2. Percorre os diretorios antigos `blobs/` e `fragments/`
3. Computa hashes BLAKE3 e insere no CAS via `put()` — duplicatas sao automaticamente deduplicadas
4. Cria entradas correspondentes em `blob_refs` / `fragment_refs`
5. Remove diretorios antigos somente apos todos os dados estarem seguros no CAS
6. Atualiza a versao de armazenamento para 2

A migracao roda na inicializacao do daemon, e idempotente (segura para re-executar) e reporta estatisticas: arquivos migrados, duplicatas encontradas, bytes economizados.

**Metricas Prometheus** (`tesseras-storage/src/metrics.rs`) — Dez novas metricas para observabilidade:

| Metrica | Descricao |
|---------|-----------|
| `cas_objects_total` | Total de objetos unicos no CAS |
| `cas_bytes_total` | Total de bytes armazenados |
| `cas_dedup_hits_total` | Numero de escritas que encontraram um objeto existente |
| `cas_bytes_saved_total` | Bytes economizados por deduplicacao |
| `cas_gc_refcount_deletions_total` | Objetos deletados quando refcount chegou a zero |
| `cas_gc_sweep_orphans_cleaned_total` | Arquivos orfaos removidos pelo sweep |
| `cas_gc_sweep_leaked_refs_cleaned_total` | Linhas de refcount vazadas limpas |
| `cas_gc_sweep_skipped_young_total` | Orfaos jovens pulados (periodo de graca) |
| `cas_gc_sweep_duration_seconds` | Tempo gasto no sweep GC |

**Testes baseados em propriedades** — Dois testes proptest verificam invariantes do CAS sob entradas aleatorias:

- `refcount_matches_actual_refs` — apos N operacoes aleatorias de put/release, o refcount sempre corresponde ao numero real de referencias pendentes
- `cas_path_is_deterministic` — o mesmo hash sempre produz o mesmo caminho no sistema de arquivos

**Atualizacao de testes de integracao** — Todos os testes de integracao em `tesseras-core`, `tesseras-replication`, `tesseras-embedded` e `tesseras-cli` atualizados para os novos construtores com CAS. Testes de deteccao de adulteracao atualizados para funcionar com o layout de diretorio CAS.

347 testes passam em todo o workspace. Clippy limpo com `-D warnings`.

## Decisoes de arquitetura

- **BLAKE3 como chave CAS**: o hash de conteudo que ja computamos para verificacao de integridade serve tambem como chave de deduplicacao. Nenhuma etapa adicional de hashing — o hash computado durante `create` ou `replicate` e reutilizado como endereco CAS.
- **Refcount SQLite ao inves de reflinks do sistema de arquivos**: consideramos usar copy-on-write no nivel do sistema de arquivos (reflinks em btrfs/XFS), mas isso amarraria o Tesseras a sistemas de arquivos especificos. Refcounting em SQLite funciona em qualquer sistema de arquivos, incluindo FAT32 em pendrives baratos e ext4 em Raspberry Pis.
- **Diretorios de prefixo hexadecimal de dois niveis**: armazenar todos os objetos CAS em um diretorio plano desaceleraria sistemas de arquivos com milhoes de entradas. A divisao `<prefixo 2 chars>/` limita qualquer diretorio individual a ~65k entradas antes de um segundo nivel ser necessario. Isso segue a abordagem usada pelo object store do Git.
- **Periodo de graca para arquivos orfaos**: o sweep GC pula arquivos com menos de 1 hora para evitar deletar objetos sendo escritos por uma operacao concorrente. Esta e uma escolha pragmatica — troca uma pequena janela de potenciais orfaos por seguranca contra crashes sem exigir fsync ou commit de duas fases.
- **Migracao copy-first**: a migracao copia dados para o CAS antes de remover diretorios antigos. Se o processo for interrompido, os dados antigos permanecem intactos e a migracao pode ser re-executada. Isso e mais lento que mover arquivos mas garante zero perda de dados.
- **Sweep no loop de reparo**: ao inves de adicionar um timer separado de GC, o sweep CAS aproveita o loop de reparo existente de 24 horas. Isso mantem o daemon simples — um unico ciclo de manutencao em segundo plano cuida tanto da saude dos fragmentos quanto da limpeza de armazenamento.

## O que vem a seguir

- **Fase 4 continuacao** — auditorias de seguranca, empacotamento para OS (Alpine, Arch, Debian, OpenBSD, FreeBSD)
- **Fase 5: Exploracao e Cultura** — navegador publico de tesseras por era/localizacao/tema/idioma, curadoria institucional, integracao genealogica (FamilySearch, Ancestry), exportacao para midia fisica (M-DISC, microfilme, papel livre de acido com QR), contexto assistido por IA

A deduplicacao de armazenamento completa a historia de eficiencia de armazenamento do Tesseras. Um no que armazena fragmentos para milhares de usuarios — comum para nos institucionais e nos completos sempre ligados — agora paga o custo de disco apenas por dados unicos. Combinado com codificacao de apagamento Reed-Solomon (que ja minimiza redundancia no nivel da rede), o sistema alcanca armazenamento eficiente tanto nas camadas local quanto distribuida.
