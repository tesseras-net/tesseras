+++
title = "Fase 4: Criptografia e Tesseras Seladas"
date = 2026-02-14T16:00:00+00:00
description = "Tesseras agora suporta memórias privadas e seladas com criptografia híbrida pós-quântica — AES-256-GCM, X25519 + ML-KEM-768 e publicação de chaves com bloqueio temporal."
+++

Algumas memórias não são para todos. Um diário privado, uma carta para ser aberta em 2050, um segredo de família selado até que os netos tenham idade suficiente. Até agora, toda tessera na rede era aberta. A Fase 4 muda isso: Tesseras agora criptografa conteúdo privado e selado com um esquema criptográfico híbrido projetado para resistir tanto a ataques clássicos quanto quânticos.

O princípio continua o mesmo — criptografar o mínimo possível. Memórias públicas precisam de disponibilidade, não de sigilo. Mas quando alguém cria uma tessera privada ou selada, o conteúdo agora é trancado por criptografia AES-256-GCM com chaves protegidas por um mecanismo híbrido de encapsulamento de chaves combinando X25519 e ML-KEM-768. Ambos os algoritmos precisam ser quebrados para acessar o conteúdo.

## O que foi construído

**Encriptador AES-256-GCM** (`tesseras-crypto/src/encryption.rs`) — Criptografia simétrica de conteúdo com nonces aleatórios de 12 bytes e dados autenticados associados (AAD). O AAD vincula o texto cifrado ao seu contexto: para tesseras privadas, o hash do conteúdo é incluído; para tesseras seladas, tanto o hash do conteúdo quanto o timestamp `open_after` são vinculados no AAD. Isso significa que mover texto cifrado entre tesseras com datas de abertura diferentes causa falha na decriptação — você não consegue enganar o sistema para abrir uma memória selada antecipadamente trocando o texto cifrado para uma tessera com uma data de selo anterior.

**Mecanismo Híbrido de Encapsulamento de Chaves** (`tesseras-crypto/src/kem.rs`) — Troca de chaves usando X25519 (Diffie-Hellman clássico em curva elíptica) combinado com ML-KEM-768 (o KEM pós-quântico baseado em reticulados padronizado pelo NIST, anteriormente Kyber). Ambos os segredos compartilhados são combinados via `blake3::derive_key` com uma string de contexto fixa ("tesseras hybrid kem v1") para produzir uma única chave de criptografia de conteúdo de 256 bits. Isso segue a mesma filosofia "dual desde o início" das assinaturas duplas do projeto (Ed25519 + ML-DSA): se qualquer algoritmo for quebrado no futuro, o outro ainda protege o conteúdo.

**Envelope de Chave Selada** (`tesseras-crypto/src/sealed.rs`) — Encapsula uma chave de criptografia de conteúdo usando o KEM híbrido, para que apenas o dono da tessera possa recuperá-la. O KEM produz uma chave de transporte, que é XORed com a chave de conteúdo para produzir uma chave encapsulada armazenada junto ao texto cifrado do KEM. Ao desselar, o dono decapsula o texto cifrado do KEM para recuperar a chave de transporte, depois faz XOR novamente para recuperar a chave de conteúdo.

**Publicação de Chave** (`tesseras-crypto/src/sealed.rs`) — Um artefato assinado independente para publicar a chave de conteúdo de uma tessera selada após a data `open_after` ter passado. O dono assina a chave de conteúdo, o hash da tessera e o timestamp de publicação com suas chaves duais (Ed25519, com placeholder ML-DSA). O manifesto permanece imutável — a publicação da chave é um documento separado. Outros nós verificam a assinatura contra a chave pública do dono antes de usar a chave publicada para decriptar o conteúdo.

**EncryptionContext** (`tesseras-core/src/enums.rs`) — Um tipo de domínio que representa o contexto AAD para criptografia. Ele vive em tesseras-core e não em tesseras-crypto porque é um conceito de domínio (não um detalhe de implementação criptográfica). O método `to_aad_bytes()` produz serialização determinística: um byte de tag (0x00 para Private, 0x01 para Sealed), seguido do hash de conteúdo e, para Sealed, o timestamp `open_after` como i64 little-endian.

**Validação de domínio** (`tesseras-core/src/service.rs`) — `TesseraService::create()` agora rejeita tesseras Sealed e Private que não fornecem chaves de criptografia. Esta é uma validação no nível de domínio: a camada de serviço garante que você não pode criar uma memória selada sem a maquinaria criptográfica para protegê-la. A mensagem de erro é clara: "missing encryption keys for visibility sealed until 2050-01-01."

**Atualizações de tipos do core** — `TesseraIdentity` agora inclui um campo opcional `encryption_public: Option<HybridEncryptionPublic>` contendo tanto as chaves públicas X25519 quanto ML-KEM-768. `KeyAlgorithm` ganhou as variantes `X25519` e `MlKem768`. O layout do sistema de arquivos de identidade agora suporta `node.x25519.key`/`.pub` e `node.mlkem768.key`/`.pub`.

**Testes** — 8 testes unitários para AES-256-GCM (roundtrip, chave errada, texto cifrado adulterado, AAD errado, falha de decriptação cross-context, nonces únicos, mais 2 testes baseados em propriedades para payloads arbitrários e unicidade de nonces). 5 testes unitários para HybridKem (roundtrip, par de chaves errado, X25519 adulterado, determinismo do KDF, mais 1 teste baseado em propriedades). 4 testes unitários para SealedKeyEnvelope e KeyPublication. 2 testes de integração cobrindo o ciclo de vida completo de tesseras seladas e privadas: gerar chaves, criar chave de conteúdo, criptografar, selar, desselar, decriptar, publicar chave e verificar — o ciclo completo.

## Decisões de arquitetura

- **KEM híbrido desde o início**: X25519 + ML-KEM-768 segue a mesma filosofia das assinaturas duplas. Não sabemos quais suposições criptográficas se manterão ao longo dos milênios, então combinamos algoritmos clássicos e pós-quânticos. O custo é ~1,2 KB de material de chave adicional por identidade — trivial comparado às fotos e vídeos em uma tessera.
- **BLAKE3 para KDF**: ao invés de adicionar `hkdf` + `sha2` como novas dependências, usamos `blake3::derive_key` com uma string de contexto fixa. O modo de derivação de chaves do BLAKE3 é especificamente projetado para este caso de uso, e o projeto já depende do BLAKE3 para hashing de conteúdo.
- **Manifestos imutáveis**: quando a data `open_after` de uma tessera selada passa, a chave de conteúdo é publicada como um artefato assinado separado (`KeyPublication`), não modificando o manifesto. Isso preserva a natureza append-only e endereçada por conteúdo das tesseras. O manifesto foi assinado no momento da criação e nunca muda.
- **Vinculação AAD previne troca de texto cifrado**: o `EncryptionContext` vincula tanto o hash de conteúdo quanto (para tesseras seladas) o timestamp `open_after` nos dados autenticados do AES-GCM. Um atacante que copie conteúdo criptografado de uma tessera "selada até 2050" para uma tessera "selada até 2025" vai descobrir que a decriptação falha — o AAD não corresponde mais.
- **Encapsulamento de chave por XOR**: o envelope de chave selada usa um XOR simples da chave de conteúdo com a chave de transporte derivada do KEM, ao invés de uma camada adicional de AES-GCM. Como a chave de transporte é um valor aleatório fresco do KEM e é usada exatamente uma vez, o XOR é informação-teoricamente seguro para este caso de uso específico e evita complexidade desnecessária.
- **Validação de domínio, não validação de storage**: a verificação de "chaves de criptografia ausentes" vive em `TesseraService::create()`, não na camada de storage. Isso segue o padrão de arquitetura hexagonal: regras de domínio são aplicadas na fronteira de serviço, não espalhadas pelos adaptadores.

## O que vem a seguir

- **Fase 4 continuada: Resiliência e Escala** — Shamir's Secret Sharing para distribuição de chaves de herdeiros, NAT traversal avançado (STUN/TURN), ajuste de performance, auditorias de segurança, empacotamento para sistemas operacionais
- **Fase 5: Exploração e Cultura** — Navegador público de tesseras por era/localização/tema/idioma, curadoria institucional, integração com genealogia, exportação para mídia física (M-DISC, microfilme, papel livre de ácido com QR)

Tesseras seladas fazem do Tesseras uma verdadeira cápsula do tempo. Um pai agora pode gravar uma mensagem para o neto que ainda não nasceu, selá-la até 2060 e saber que o envelope criptográfico vai resistir — mesmo que os computadores quânticos do futuro tentem abri-lo antes da hora.
