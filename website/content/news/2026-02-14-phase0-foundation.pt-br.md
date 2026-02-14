+++
title = "Fase 0: Fundação Construída"
date = 2026-02-14
description = "Os crates fundamentais do Tesseras estão prontos — tipos de domínio, primitivas criptográficas, armazenamento SQLite e uma CLI funcional."
+++

O primeiro marco do projeto Tesseras está completo. A Fase 0 estabelece a fundação sobre a qual cada componente futuro será construído: tipos de domínio, criptografia, armazenamento e uma interface de linha de comando funcional.

## O que foi construído

**tesseras-core** — A camada de domínio define o formato tessera: `ContentHash` (BLAKE3, 32 bytes), `NodeId` (Kademlia, 20 bytes), tipos de memória (Moment, Reflection, Daily, Relation, Object), modos de visibilidade (Private, Circle, Public, PublicAfterDeath, Sealed) e um formato de manifesto em texto plano que pode ser interpretado por qualquer linguagem de programação pelos próximos mil anos. A camada de serviço (`TesseraService`) gerencia operações de criação, verificação, exportação e listagem através de port traits, seguindo arquitetura hexagonal.

**tesseras-crypto** — Geração de chaves Ed25519, assinatura e verificação. Um framework de assinatura dual (Ed25519 + placeholder ML-DSA) pronto para migração pós-quântica. Hashing de conteúdo com BLAKE3. Codificação de apagamento Reed-Solomon atrás de uma feature flag para futura replicação.

**tesseras-storage** — Índice SQLite via sqlx com migrações verificadas em tempo de compilação. Blob store no sistema de arquivos com layout endereçável por conteúdo (`blobs/<tessera_hash>/<memory_hash>/<filename>`). Persistência de chaves de identidade em disco.

**tesseras-cli** — Um binário `tesseras` funcional com cinco comandos:
- `init` — gera identidade Ed25519, cria banco de dados SQLite
- `create <dir>` — varre um diretório por arquivos de mídia, cria uma tessera assinada
- `verify <hash>` — verifica assinatura e integridade dos arquivos
- `export <hash> <dest>` — escreve um diretório tessera autocontido
- `list` — mostra uma tabela das tesseras armazenadas

**Testes** — 67+ testes em todo o workspace: testes unitários em cada módulo, testes baseados em propriedades (proptest) para roundtrips hex e serialização de manifesto, testes de integração cobrindo o ciclo completo de criação-verificação-exportação incluindo detecção de arquivos adulterados e assinaturas inválidas. Zero avisos do clippy.

## Decisões de arquitetura

- **Arquitetura hexagonal**: operações criptográficas são injetadas via trait objects (`Box<dyn Hasher>`, `Box<dyn ManifestSigner>`, `Box<dyn ManifestVerifier>`), mantendo o crate core livre de dependências criptográficas concretas.
- **Feature flags**: a feature `service` no tesseras-core controla a camada de aplicação assíncrona. As features `classical` e `erasure` no tesseras-crypto controlam quais algoritmos são compilados.
- **Manifesto em texto plano**: interpretável sem qualquer biblioteca de formato binário, com prefixos de hash explícitos `blake3:` e layout legível por humanos.

## O que vem a seguir

A Fase 0 é a fundação local. O caminho adiante:

- **Fase 1: Rede** — Transporte QUIC (quinn), DHT Kademlia para descoberta de pares, travessia de NAT
- **Fase 2: Replicação** — Codificação de apagamento Reed-Solomon pela rede, loops de reparo, reciprocidade bilateral (sem blockchain, sem tokens)
- **Fase 3: Clientes** — App Flutter mobile/desktop via flutter_rust_bridge, API GraphQL, nó WASM no navegador
- **Fase 4: Endurecimento** — Assinaturas pós-quânticas ML-DSA, empacotamento para Alpine/Arch/Debian/FreeBSD/OpenBSD, CI no SourceHut

O formato tessera é estável. Tudo construído a partir daqui se conecta e estende o que existe hoje.
