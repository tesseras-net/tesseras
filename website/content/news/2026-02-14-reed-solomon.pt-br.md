+++
title = "Reed-Solomon: Como o Tesseras Sobrevive à Perda de Dados"
date = 2026-02-14T14:00:00+00:00
description = "Um mergulho profundo na codificação de apagamento Reed-Solomon — o que é, por que o Tesseras a utiliza e os desafios de manter memórias vivas ao longo dos séculos."
+++

Seu disco rígido vai morrer. Seu provedor de nuvem vai pivotar. O array RAID no seu armário vai sobreviver ao controlador, mas não ao dono. Se uma memória está armazenada em exatamente um lugar, ela tem exatamente uma forma de se perder para sempre.

Tesseras é uma rede que mantém memórias humanas vivas através de ajuda mútua. O mecanismo central de sobrevivência é a **codificação de apagamento Reed-Solomon** — uma técnica emprestada da comunicação espacial profunda que nos permite reconstruir dados mesmo quando pedaços desaparecem.

## O que é Reed-Solomon?

Reed-Solomon é uma família de códigos corretores de erros inventada por Irving Reed e Gustave Solomon em 1960. O caso de uso original era corrigir erros em dados transmitidos por canais ruidosos — pense na Voyager enviando fotos de Júpiter, ou num CD tocando apesar de arranhões.

A ideia-chave: se você adicionar redundância cuidadosamente calculada aos seus dados *antes* que algo dê errado, você pode recuperar o original mesmo depois de perder alguns pedaços.

Eis a intuição. Suponha que você tenha um polinômio de grau 2 — uma parábola. Você precisa de 3 pontos para defini-lo de forma única. Mas se você avaliá-lo em 5 pontos, pode perder quaisquer 2 desses 5 e ainda reconstruir o polinômio a partir dos 3 restantes. Reed-Solomon generaliza essa ideia para trabalhar sobre corpos finitos (corpos de Galois), onde o "polinômio" são seus dados e os "pontos de avaliação" são seus fragmentos.

Em termos concretos:

1. **Divida** seus dados em *k* shards de dados
2. **Calcule** *m* shards de paridade a partir dos shards de dados
3. **Distribua** todos os *k + m* shards em diferentes locais
4. **Reconstrua** os dados originais a partir de quaisquer *k* dos *k + m* shards

Você pode perder até *m* shards — quaisquer *m*, de dados ou paridade, em qualquer combinação — e ainda recuperar tudo.

## Por que não simplesmente fazer cópias?

A abordagem ingênua para redundância é a replicação: faça 3 cópias, armazene-as em 3 lugares. Isso dá tolerância a 2 falhas ao custo de 3x o seu armazenamento.

Reed-Solomon é dramaticamente mais eficiente:

| Estratégia | Overhead de armazenamento | Falhas toleradas |
|----------|----------------:|-------------------:|
| Replicação 3x | 200% | 2 de 3 |
| Reed-Solomon (16,8) | 50% | 8 de 24 |
| Reed-Solomon (48,24) | 50% | 24 de 72 |

Com 16 shards de dados e 8 de paridade, você usa 50% de armazenamento extra mas pode sobreviver à perda de um terço de todos os fragmentos. Para alcançar a mesma tolerância a falhas só com replicação, você precisaria de 3x o armazenamento.

Para uma rede que visa preservar memórias ao longo de décadas e séculos, essa eficiência não é um luxo — é a diferença entre um sistema viável e um que se afoga no próprio overhead.

## Como o Tesseras usa Reed-Solomon

Nem todos os dados merecem o mesmo tratamento. Uma memória de texto de 500 bytes e um vídeo de 100 MB têm necessidades de redundância muito diferentes. O Tesseras usa uma estratégia de fragmentação em três camadas:

**Small (< 4 MB)** — Replicação do arquivo inteiro para 7 pares. Para tesseras pequenas, o overhead da codificação de apagamento (tempo de codificação, gerenciamento de fragmentos, lógica de reconstrução) supera seus benefícios. Cópias simples são mais rápidas e mais simples.

**Medium (4–256 MB)** — 16 shards de dados + 8 de paridade = 24 fragmentos no total. Cada fragmento tem aproximadamente 1/16 do tamanho original. Quaisquer 16 dos 24 fragmentos reconstroem o original. Distribuídos entre 7 pares.

**Large (≥ 256 MB)** — 48 shards de dados + 24 de paridade = 72 fragmentos no total. Maior contagem de shards significa fragmentos individuais menores (mais fáceis de transferir e armazenar) e maior tolerância absoluta a falhas. Também distribuídos entre 7 pares.

A implementação usa o crate `reed-solomon-erasure` operando sobre GF(2⁸) — o mesmo corpo de Galois usado em códigos QR e CDs. Cada fragmento carrega um checksum BLAKE3 para que a corrupção seja detectada imediatamente, não propagada silenciosamente.

```
Tessera (álbum de fotos de 120 MB)
    ↓ codificar
16 shards de dados (7,5 MB cada) + 8 shards de paridade (7,5 MB cada)
    ↓ distribuir
24 fragmentos entre 7 pares (diversidade de sub-rede)
    ↓ quaisquer 16 fragmentos
Tessera original recuperada
```

## Os desafios

Reed-Solomon resolve o problema matemático da redundância. Os desafios de engenharia estão em tudo ao redor.

### Rastreamento de fragmentos

Cada fragmento precisa ser localizável. O Tesseras usa uma DHT Kademlia para descoberta de pares e mapeamento de fragmentos para pares. Quando um nó fica offline, seus fragmentos precisam ser recriados e distribuídos para novos pares. Isso significa rastrear quais fragmentos existem, onde estão e se ainda estão intactos — numa rede sem autoridade central.

### Corrupção silenciosa

Um fragmento que retorna dados errados é pior que um ausente — pelo menos um fragmento ausente é honestamente ausente. O Tesseras aborda isso com verificações de saúde baseadas em atestação: o loop de reparo periodicamente pede aos detentores de fragmentos que provem posse retornando checksums BLAKE3. Se um checksum não bater, o fragmento é tratado como perdido.

### Falhas correlacionadas

Se todos os 24 fragmentos de uma tessera caírem em máquinas no mesmo datacenter, uma única queda de energia os elimina todos. A matemática do Reed-Solomon assume falhas independentes. O Tesseras impõe **diversidade de sub-rede** durante a distribuição: no máximo 2 fragmentos por sub-rede /24 IPv4 (ou prefixo /48 IPv6). Isso espalha fragmentos por diferentes infraestruturas físicas.

### Velocidade de reparo vs. carga na rede

Quando um par fica offline, o relógio começa a contar. Fragmentos perdidos precisam ser recriados antes que mais falhas se acumulem. Mas reparo agressivo inunda a rede. O Tesseras equilibra isso com um loop de reparo configurável (padrão: a cada 24 horas com 2 horas de jitter) e limites de transferências simultâneas (padrão: 4 transferências simultâneas). O jitter previne tempestades de reparo onde cada nó verifica seus fragmentos no mesmo momento.

### Gerenciamento de chaves a longo prazo

Reed-Solomon protege contra perda de dados, não contra perda de acesso. Se uma tessera é criptografada (visibilidade privada ou selada), você precisa da chave de descriptografia para tornar os dados recuperados úteis. O Tesseras separa essas preocupações: codificação de apagamento cuida da disponibilidade, enquanto o Compartilhamento de Segredo de Shamir (uma fase futura) cuidará da distribuição de chaves entre herdeiros. A filosofia de design do projeto — criptografar o mínimo possível — mantém o problema de gerenciamento de chaves pequeno.

### Limitações do corpo de Galois

O corpo GF(2⁸) limita o número total de shards a 255 (dados + paridade combinados). Para o Tesseras, isso não é uma restrição prática — mesmo a camada Large usa apenas 72 shards. Mas significa que arquivos extremamente grandes com milhares de fragmentos exigiriam um corpo diferente ou um esquema de codificação em camadas.

### Compatibilidade evolutiva do codec

Uma tessera codificada hoje precisa ser decodificável em 50 anos. Reed-Solomon sobre GF(2⁸) é um dos algoritmos mais amplamente implementados na computação — está em todo leitor de CD, em todo scanner de código QR, em toda sonda espacial. Essa ubiquidade é em si uma estratégia de sobrevivência. O algoritmo não será esquecido porque metade da infraestrutura do mundo depende dele.

## O quadro geral

Reed-Solomon é uma peça de um quebra-cabeça maior. Ele trabalha em conjunto com:

- **DHT Kademlia** para encontrar pares e rotear fragmentos
- **Checksums BLAKE3** para verificação de integridade
- **Reciprocidade bilateral** para troca justa de armazenamento (sem blockchain)
- **Diversidade de sub-rede** para independência de falhas
- **Reparo automático** para manter a redundância ao longo do tempo

Nenhuma técnica isolada faz memórias sobreviverem. Reed-Solomon garante que dados *podem* ser recuperados. A DHT garante que fragmentos *podem ser encontrados*. A reciprocidade garante que pares *querem ajudar*. O reparo garante que nada disso se degrade com o tempo.

Uma tessera é uma aposta de que a soma desses mecanismos, rodando em muitas máquinas independentes operadas por muitas pessoas independentes, é mais durável que qualquer instituição isolada. Reed-Solomon é a fundação matemática dessa aposta.
