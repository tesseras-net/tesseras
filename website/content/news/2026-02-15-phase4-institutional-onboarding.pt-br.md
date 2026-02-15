+++
title = "Fase 4: Onboarding de Nos Institucionais"
date = 2026-02-15T22:00:00+00:00
description = "Bibliotecas, arquivos e museus agora podem ingressar na rede Tesseras como nos institucionais verificados com identidade baseada em DNS, indices de busca full-text e compromissos configuraveis de armazenamento."
+++

Uma rede P2P composta apenas por individuos e fragil. Discos rigidos morrem, celulares sao perdidos, pessoas perdem interesse. A sobrevivencia a longo prazo das memorias da humanidade depende de instituicoes — bibliotecas, arquivos, museus, universidades — que medem seus tempos de vida em seculos. A Fase 4 continua com o onboarding de nos institucionais: organizacoes verificadas agora podem prometer armazenamento, manter indices de busca e participar da rede com uma identidade distinta.

O design segue um principio de confiar mas verificar: instituicoes se identificam via registros DNS TXT (o mesmo mecanismo usado por SPF, DKIM e DMARC para email), prometem um orcamento de armazenamento e recebem isencoes de reciprocidade para que possam armazenar fragmentos para outros sem esperar nada em troca. Em contrapartida, a rede trata seus fragmentos como replicas de maior qualidade e limita a dependencia excessiva de qualquer instituicao individual atraves de restricoes de diversidade.

## O que foi construido

**Bits de capacidade** (`tesseras-core/src/network.rs`) — Dois novos flags adicionados ao bitfield `Capabilities`: `INSTITUTIONAL` (bit 7) e `SEARCH_INDEX` (bit 8). Um novo construtor `institutional_default()` retorna o conjunto completo de capacidades da Fase 2 mais esses dois bits e `RELAY`. Nos normais anunciam `phase2_default()` que nao inclui flags institucionais. Testes de roundtrip de serializacao verificam que os novos bits sobrevivem a codificacao MessagePack.

**Tipos de busca** (`tesseras-core/src/search.rs`) — Tres novos tipos de dominio para o subsistema de busca:

- `SearchFilters` — parametros de consulta: `memory_type`, `visibility`, `language`, `date_range`, `geo` (bounding box), `page`, `page_size`
- `SearchHit` — um resultado individual: hash do conteudo mais um `MetadataExcerpt` (titulo, descricao, tipo de memoria, data de criacao, visibilidade, idioma, tags)
- `GeoFilter` — bounding box com `min_lat`, `max_lat`, `min_lon`, `max_lon` para consultas espaciais

Todos os tipos derivam `Serialize`/`Deserialize` para transporte e `Clone`/`Debug` para diagnostico.

**Configuracao institucional do daemon** (`tesseras-daemon/src/config.rs`) — Uma nova secao `[institutional]` no TOML com `domain` (o dominio DNS a verificar), `pledge_bytes` (compromisso de armazenamento em bytes) e `search_enabled` (toggle para o indice FTS5). O metodo `to_dht_config()` agora define `Capabilities::institutional_default()` quando a configuracao institucional esta presente, para que nos institucionais anunciem os bits de capacidade corretos em respostas Pong.

**Verificacao DNS TXT** (`tesseras-daemon/src/institutional.rs`) — Resolucao DNS assincrona usando `hickory-resolver` para verificar identidade institucional. O daemon consulta registros TXT em `_tesseras.<dominio>` e analisa campos chave-valor: `v` (versao), `node` (node ID em hexadecimal) e `pledge` (compromisso de armazenamento em bytes). A verificacao checa:

1. Um registro TXT existe em `_tesseras.<dominio>`
2. O campo `node` corresponde ao node ID do proprio daemon
3. O campo `pledge` esta presente e e valido

Na inicializacao, o daemon tenta a verificacao DNS. Se bem-sucedida, o no roda com capacidades institucionais. Se falhar, o no registra um aviso e faz downgrade para um no completo normal — sem crash, sem intervencao manual.

**Comando CLI de setup** (`tesseras-cli/src/institutional.rs`) — Um novo subcomando `institutional setup` que guia operadores pelo onboarding:

1. Le a identidade do no a partir do diretorio de dados
2. Solicita nome de dominio e tamanho do pledge
3. Gera o registro DNS TXT exato a adicionar: `v=tesseras1 node=<hex> pledge=<bytes>`
4. Escreve a secao institucional no arquivo de configuracao do daemon
5. Imprime os proximos passos: adicionar o registro TXT, reiniciar o daemon

**Indice de busca SQLite** (`tesseras-storage`) — Uma migracao (`003_institutional.sql`) que cria tres estruturas:

- `search_content` — uma tabela virtual FTS5 para busca full-text sobre metadados de tesseras (titulo, descricao, criador, tags, idioma)
- `geo_index` — uma tabela virtual R-tree para consultas espaciais de bounding box sobre latitude/longitude
- `geo_map` — uma tabela de mapeamento ligando IDs de linhas do R-tree a hashes de conteudo

O adaptador `SqliteSearchIndex` implementa o port trait `SearchIndex` com `index_tessera()` (inserir/atualizar) e `search()` (consultar com filtros). Consultas FTS5 suportam busca em linguagem natural; consultas geo usam `INTERSECT` do R-tree para lookups de bounding box. Resultados sao ranqueados por score de relevancia do FTS5.

A migracao tambem adiciona uma coluna `is_institutional` a tabela `reciprocity`, tratada de forma idempotente via checagens `pragma_table_info` (o `ALTER TABLE ADD COLUMN` do SQLite nao tem `IF NOT EXISTS`).

**Bypass de reciprocidade** (`tesseras-replication/src/service.rs`) — Nos institucionais sao isentos de checagens de reciprocidade. Quando `receive_fragment()` e chamado, se o node ID do remetente esta marcado como institucional no ledger de reciprocidade, a checagem de saldo e ignorada completamente. Isso significa que instituicoes podem armazenar fragmentos para toda a rede sem precisar "ganhar" creditos primeiro — sua identidade verificada por DNS e compromisso de armazenamento servem como credencial.

**Restricao de diversidade por tipo de no** (`tesseras-replication/src/distributor.rs`) — Uma nova funcao `apply_institutional_diversity()` limita quantas replicas de uma unica tessera podem ir para nos institucionais. O limite e `ceil(fator_replicacao / 3.5)` — com o padrao `r=7`, no maximo 2 de 7 replicas vao para instituicoes. Isso impede que a rede se torne dependente de um pequeno numero de grandes instituicoes: se os servidores de uma universidade cairem, pelo menos 5 replicas permanecem em nos independentes.

**Extensoes de mensagens DHT** (`tesseras-dht/src/message.rs`) — Duas novas variantes de mensagem:

| Mensagem | Proposito |
|----------|-----------|
| `Search` | Cliente envia string de consulta, filtros e numero da pagina |
| `SearchResult` | No institucional responde com resultados e contagem total |

A funcao `encode()` foi trocada de serializacao MessagePack posicional para nomeada (`rmp_serde::to_vec_named`) para lidar corretamente com campos opcionais de `SearchFilters` — a codificacao posicional quebra quando `skip_serializing_if` omite campos.

**Metricas Prometheus** (`tesseras-daemon/src/metrics.rs`) — Oito metricas especificas institucionais:

- `tesseras_institutional_pledge_bytes` — compromisso de armazenamento configurado
- `tesseras_institutional_stored_bytes` — bytes realmente armazenados
- `tesseras_institutional_pledge_utilization_ratio` — razao armazenado/prometido
- `tesseras_institutional_peers_served` — peers unicos que receberam fragmentos
- `tesseras_institutional_search_index_total` — tesseras no indice de busca
- `tesseras_institutional_search_queries_total` — consultas de busca recebidas
- `tesseras_institutional_dns_verification_status` — 1 se verificado por DNS, 0 caso contrario
- `tesseras_institutional_dns_verification_last` — timestamp Unix da ultima verificacao

**Testes de integracao** — Dois testes em `tesseras-replication/tests/integration.rs`:

- `institutional_peer_bypasses_reciprocity` — verifica que um peer institucional com deficit massivo (-999.999 de saldo) ainda pode armazenar fragmentos, enquanto um peer nao institucional com o mesmo deficit e rejeitado
- `institutional_node_accepts_fragment_despite_deficit` — teste async completo usando `ReplicationService` com DHT, fragment store, reciprocity ledger e blob store mockados: envia um fragmento de um remetente institucional e verifica que e aceito

322 testes passam em todo o workspace. Clippy limpo com `-D warnings`.

## Decisoes de arquitetura

- **DNS TXT ao inves de PKI ou blockchain**: DNS e universalmente implantado, universalmente compreendido e ja usado para verificacao de dominio (SPF, DKIM, Let's Encrypt). Instituicoes ja gerenciam DNS. Nenhuma autoridade certificadora, nenhum token, nenhuma transacao on-chain — apenas um registro TXT. Se uma instituicao perder controle de seu dominio, a verificacao naturalmente falha na proxima checagem.
- **Degradacao graciosa em falha DNS**: se a verificacao DNS falha na inicializacao, o daemon faz downgrade para um no completo normal ao inves de recusar iniciar. Isso previne incidentes operacionais — uma misconfiguracao DNS nao deveria tirar um no do ar.
- **Limite de diversidade em `ceil(r / 3.5)`**: com `r=7`, no maximo 2 replicas vao para instituicoes. Isso e conservador — garante que a rede nunca dependa de instituicoes para quorum majoritario, enquanto ainda se beneficia de sua capacidade de armazenamento e uptime.
- **Codificacao MessagePack nomeada**: trocar de codificacao posicional para nomeada adiciona ~15% de overhead por mensagem mas elimina uma classe de bugs de serializacao quando campos opcionais estao presentes. O DHT nao e limitado por largura de banda no nivel de mensagem, entao o tradeoff vale a pena.
- **Isencao de reciprocidade ao inves de concessao de creditos**: ao inves de dar as instituicoes um saldo inicial grande de creditos (que e arbitrario e precisa de ajuste), isentamos completamente. Sua identidade verificada por DNS e compromisso publico de armazenamento substituem o mecanismo de reciprocidade bilateral.
- **FTS5 + R-tree no SQLite**: busca full-text e indexacao espacial sao embutidas no SQLite como extensoes carregaveis. Nenhum motor de busca externo (Elasticsearch, Meilisearch) necessario. Isso mantem o deploy como um unico binario com um unico arquivo de banco de dados — critico para operadores institucionais que podem nao ter uma equipe de DevOps.

## O que vem a seguir

- **Fase 4 continuacao** — deduplicacao de armazenamento (armazenamento enderecavel por conteudo com BLAKE3), auditorias de seguranca, empacotamento para OS (Alpine, Arch, Debian, OpenBSD, FreeBSD)
- **Fase 5: Exploracao e Cultura** — navegador publico de tesseras por era/localizacao/tema/idioma, curadoria institucional, integracao genealogica (FamilySearch, Ancestry), exportacao para midia fisica (M-DISC, microfilme, papel livre de acido com QR), contexto assistido por IA

O onboarding institucional fecha uma lacuna critica no modelo de preservacao do Tesseras. Nos individuais fornecem resiliencia de base — milhares de dispositivos ao redor do globo, cada um armazenando alguns fragmentos. Nos institucionais fornecem ancoragem — organizacoes com infraestrutura profissional, armazenamento redundante e horizontes operacionais de multiplas decadas. Juntos, formam uma rede onde memorias podem sobreviver tanto a dispositivos individuais quanto a instituicoes individuais.
