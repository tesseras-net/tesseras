# Introdução

Tesseras é uma rede peer-to-peer para preservar memórias humanas através dos milênios. Cada pessoa cria uma **tessera** — uma cápsula do tempo autocontida de memórias (fotos, áudio, vídeo, texto) que sobrevive independentemente de qualquer software, empresa ou infraestrutura.

## O que é uma tessera?

A palavra *tessera* vem das pequenas peças usadas para fazer mosaicos no mundo antigo. No Tesseras, cada tessera é uma coleção de memórias empacotada em um formato projetado para ser compreendido mesmo daqui a milhares de anos, sem nenhum software especial.

Uma tessera contém:

- **Memórias** — fotos (JPEG), gravações de áudio (WAV), vídeo (WebM) e texto (UTF-8 puro)
- **Metadados** — quando e onde cada memória foi criada, quem está envolvido e o que significa
- **Identidade** — assinaturas criptográficas provando quem criou
- **Instruções de decodificação** — explicações em texto puro de cada formato utilizado, para que humanos do futuro possam ler o conteúdo

## Filosofia central

- **Sem dependência de empresas** — suas memórias são suas, armazenadas localmente e replicadas em uma rede peer-to-peer
- **Sem aprisionamento de formato** — cada tessera inclui instruções para decodificar seu conteúdo
- **Disponibilidade acima de sigilo** — memórias públicas não são criptografadas, porque acessibilidade a longo prazo importa mais do que esconder coisas
- **Criptografia mínima** — apenas conteúdo privado e selado é criptografado; todo o resto é aberto
- **Resistente a computadores quânticos** — assinaturas duplas (Ed25519 + ML-DSA) protegem a integridade mesmo contra futuros computadores quânticos

## Status atual: Fase 4

Tesseras completou até a **Fase 4** — criptografia e tesseras seladas. O projeto agora cobre gerenciamento local de tesseras, rede, replicação, app mobile e privacidade criptográfica.

O que está disponível hoje:

- Geração de identidade (par de chaves Ed25519 com prova de trabalho)
- Criação de tesseras a partir de arquivos locais
- Armazenamento endereçado por conteúdo (hashing BLAKE3)
- Verificação de integridade e exportação autocontida
- Daemon de nó completo com transporte QUIC
- Descoberta de pares via DHT Kademlia
- Publicação e busca de ponteiros de tesseras pela rede
- Codificação de apagamento Reed-Solomon com reparo automático de fragmentos
- App mobile Flutter com nó Rust P2P embarcado
- **Tesseras privadas** — conteúdo criptografado que apenas o dono pode acessar
- **Tesseras seladas** — conteúdo com bloqueio temporal que abre após uma data específica
- **Criptografia híbrida pós-quântica** — encapsulamento de chaves X25519 + ML-KEM-768
- **AES-256-GCM** para criptografia de conteúdo com vinculação AAD

## Conceitos-chave

| Conceito | Descrição |
|----------|-----------|
| **Tessera** | Uma cápsula do tempo autocontida de memórias |
| **Memória** | Um item individual (foto, gravação, vídeo ou texto) dentro de uma tessera |
| **Hash de conteúdo** | Um hash BLAKE3 que identifica unicamente uma tessera pelo seu conteúdo |
| **Visibilidade** | Controla quem pode acessar uma tessera: pública, privada, selada ou círculo |
| **Tessera selada** | Uma cápsula do tempo que só pode ser aberta após uma data específica |
| **MANIFEST** | Um índice em texto puro listando cada arquivo na tessera com seu checksum |
| **Tipo de memória** | Categoriza uma memória: momento, reflexão, cotidiano, relação ou objeto |
| **Nó** | Um dispositivo executando o daemon Tesseras, participando da rede P2P |
| **DHT** | Tabela hash distribuída — como os nós encontram ponteiros de tesseras sem um servidor central |
| **Bootstrap** | O processo de entrar na rede contactando nós semente conhecidos |
