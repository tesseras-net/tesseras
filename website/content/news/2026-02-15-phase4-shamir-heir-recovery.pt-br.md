+++
title = "Fase 4: Recuperação de Chaves por Herdeiros com Shamir's Secret Sharing"
date = 2026-02-15
description = "Tesseras agora permite dividir sua identidade criptográfica em fragmentos distribuídos a herdeiros de confiança — qualquer limiar deles pode reconstruir suas chaves, mas menos que isso não revela nada."
+++

O que acontece com suas memórias quando você morre? Até agora, Tesseras conseguia preservar conteúdo ao longo de milênios — mas as chaves privadas e seladas morriam com o dono. A Fase 4 continua com uma solução: Shamir's Secret Sharing, um esquema criptográfico que permite dividir sua identidade em fragmentos e distribuí-los para as pessoas em quem você mais confia.

A matemática é elegante: você escolhe um limiar T e um total N. Qualquer T fragmentos reconstroem o segredo completo; T-1 fragmentos não revelam absolutamente nada. Isso não é "quase nada" — é informação-teoricamente seguro. Um atacante com um fragmento a menos que o limiar tem exatamente zero bits de informação sobre o segredo, independentemente do poder computacional que tenha.

## O que foi construído

**Aritmética de corpo finito GF(256)** (`tesseras-crypto/src/shamir/gf256.rs`) — Shamir's Secret Sharing requer aritmética em um corpo finito. Implementamos GF(256) usando o mesmo polinômio irredutível do AES (x^8 + x^4 + x^3 + x + 1), com tabelas de lookup para logaritmo e exponenciação computadas em tempo de compilação. Todas as operações são em tempo constante via consulta a tabelas — sem ramificações baseadas em dados secretos. O módulo inclui o método de Horner para avaliação de polinômios e interpolação de Lagrange em x=0 para recuperação do segredo. 233 linhas, exaustivamente testado: todos os 256 elementos para propriedades de identidade/inverso, comutatividade e associatividade.

**ShamirSplitter** (`tesseras-crypto/src/shamir/mod.rs`) — A API principal de split/reconstruct. `split()` recebe uma fatia de bytes do segredo, uma configuração (limiar T, total N) e a chave pública Ed25519 do dono. Para cada byte do segredo, constrói um polinômio aleatório de grau T-1 sobre GF(256) com o byte do segredo como termo constante, e então o avalia em N pontos distintos. `reconstruct()` recebe T ou mais fragmentos e recupera o segredo via interpolação de Lagrange. Ambas as operações incluem validação extensiva: limites do limiar, consistência de sessão, correspondência de impressão digital do dono e verificação de checksum BLAKE3.

**Formato HeirShare** — Cada fragmento é um artefato autocontido e serializável com:
- Versão do formato (v1) para compatibilidade futura
- Índice do fragmento (1..N) e metadados de limiar/total
- ID de sessão (8 bytes aleatórios) — impede mistura de fragmentos de sessões diferentes
- Impressão digital do dono (primeiros 8 bytes do hash BLAKE3 da chave pública Ed25519)
- Dados do fragmento (os y-values de Shamir, mesmo comprimento do segredo)
- Checksum BLAKE3 sobre todos os campos anteriores

Os fragmentos são serializados em dois formatos: **MessagePack** (binário compacto, para uso programático) e **texto base64** (legível por humanos, para impressão e armazenamento físico). O formato texto inclui um cabeçalho com metadados e delimitadores:

```
--- TESSERAS HEIR SHARE ---
Format: v1
Owner: a1b2c3d4e5f6a7b8 (fingerprint)
Share: 1 of 3 (threshold: 2)
Session: 9f8e7d6c5b4a3210
Created: 2026-02-15

<dados MessagePack codificados em base64>
--- END HEIR SHARE ---
```

Este formato é projetado para ser impresso em papel, armazenado em um cofre bancário ou gravado em metal. O cabeçalho é informacional — apenas o payload base64 é analisado durante a reconstrução.

**Integração com CLI** (`tesseras-cli/src/commands/heir.rs`) — Três novos subcomandos:

- `tes heir create` — divide sua identidade Ed25519 em fragmentos de herdeiros. Solicita confirmação (sua identidade completa está em jogo), gera arquivos `.bin` e `.txt` para cada fragmento e escreve `heir_meta.json` no diretório de identidade.
- `tes heir reconstruct` — carrega arquivos de fragmentos (detecta automaticamente formato binário vs texto), valida consistência, reconstrói o segredo, deriva o par de chaves Ed25519 e opcionalmente o instala em `~/.tesseras/identity/` (com backup automático da identidade existente).
- `tes heir info` — exibe metadados do fragmento e verifica o checksum sem expor nenhum material secreto.

**Formato do blob secreto** — As chaves de identidade são serializadas em um blob versionado antes da divisão: um byte de versão (0x01), um byte de flags (0x00 para somente Ed25519), seguido da chave secreta Ed25519 de 32 bytes. Isso deixa espaço para expansão futura quando as chaves privadas X25519 e ML-KEM-768 forem integradas ao sistema de fragmentos de herdeiros.

**Testes** — 20 testes unitários para ShamirSplitter (roundtrip, todas as combinações de fragmentos, fragmentos insuficientes, dono errado, sessão errada, limite threshold-1, segredos grandes até o tamanho de chave ML-KEM-768). 7 testes unitários para aritmética GF(256) (propriedades de campo exaustivas). 3 testes baseados em propriedades com proptest (segredos arbitrários até 5000 bytes, configurações T-de-N arbitrárias, verificação de segurança informação-teórica). Testes de roundtrip de serialização para ambos os formatos MessagePack e texto base64. 2 testes de integração cobrindo o ciclo de vida completo de herdeiros: gerar identidade, dividir em fragmentos, serializar, desserializar, reconstruir, verificar par de chaves e assinar/verificar com chaves reconstruídas.

## Decisões de arquitetura

- **GF(256) ao invés de GF(primo)**: usamos GF(256) ao invés de um corpo primo porque ele mapeia naturalmente para bytes — cada elemento é um único byte, cada fragmento tem o mesmo comprimento do segredo. Sem aritmética de inteiros grandes, sem redução modular, sem padding. Esta é a mesma abordagem usada pela maioria das implementações reais de Shamir, incluindo SSSS e Hashicorp Vault.
- **Tabelas de lookup em tempo de compilação**: as tabelas LOG e EXP para GF(256) são computadas em tempo de compilação usando `const fn`. Isso significa zero custo de inicialização em tempo de execução e operações em tempo constante via consulta a tabelas ao invés de loops.
- **ID de sessão previne mistura entre sessões**: cada chamada a `split()` gera um novo ID de sessão aleatório. Se um herdeiro acidentalmente usar fragmentos de duas sessões diferentes de divisão (por exemplo, antes e depois de uma rotação de chaves), a reconstrução falha de forma limpa com um erro de validação ao invés de produzir dados corrompidos.
- **Checksums BLAKE3 detectam corrupção**: cada fragmento inclui um checksum BLAKE3 sobre seu conteúdo. Isso captura degradação de bits, erros de transmissão e truncamento acidental antes de qualquer tentativa de reconstrução. Um fragmento impresso em papel e escaneado via OCR vai falhar no checksum se um único caractere estiver errado.
- **Impressão digital do dono para identificação**: os fragmentos incluem os primeiros 8 bytes de BLAKE3(chave pública Ed25519) como impressão digital. Isso permite aos herdeiros verificar a qual identidade um fragmento pertence sem revelar a chave pública completa. Durante a reconstrução, a impressão digital é verificada contra a chave recuperada.
- **Formato duplo para resiliência**: ambos os formatos binário (MessagePack) e texto (base64) são gerados porque mídias físicas têm modos de falha diferentes de armazenamento digital. Um pendrive pode falhar; papel sobrevive. Um QR code pode ficar ilegível; texto base64 pode ser digitado manualmente.
- **Versionamento do blob**: o segredo é envolvido em um blob versionado (versão + flags + material de chave) para que versões futuras possam incluir chaves adicionais (X25519, ML-KEM-768) sem quebrar compatibilidade com fragmentos existentes.

## O que vem a seguir

- **Fase 4 continuada: Resiliência e Escala** — NAT traversal avançado (STUN/TURN), ajuste de performance (pool de conexões, cache de fragmentos, SQLite WAL), auditorias de segurança, integração de nós institucionais, empacotamento para sistemas operacionais
- **Fase 5: Exploração e Cultura** — navegador público de tesseras por era/localização/tema/idioma, curadoria institucional, integração com genealogia, exportação para mídia física (M-DISC, microfilme, papel livre de ácido com QR)

Com Shamir's Secret Sharing, Tesseras fecha a última lacuna crítica na preservação a longo prazo. Suas memórias sobrevivem a falhas de infraestrutura através de erasure coding. Sua privacidade sobrevive a computadores quânticos através de criptografia híbrida. E agora, sua identidade sobrevive a você — passada adiante para as pessoas que você escolheu, exigindo a cooperação delas para desbloquear o que você deixou para trás.
