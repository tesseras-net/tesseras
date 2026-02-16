# Conceitos de Rede

Este capitulo explica como os nos do Tesseras se encontram e localizam ponteiros de tesseras na rede. Voce nao precisa entender esses detalhes para usar o Tesseras, mas eles ajudam a explicar o que o daemon esta fazendo em segundo plano.

## Como os nos se encontram

Tesseras usa uma **tabela hash distribuida (DHT) Kademlia** — um algoritmo comprovado usado pelo BitTorrent e outros sistemas P2P por mais de 20 anos. Nao ha servidor central. Cada no mantem uma tabela de roteamento dos pares que conhece, e os nos cooperam para direcionar consultas ao lugar certo.

Quando seu no inicia, ele contacta um ou mais **nos de bootstrap** (nos semente com enderecos conhecidos). Atraves dessas conexoes iniciais, seu no descobre outros pares e constroi sua tabela de roteamento. Com o tempo, seu no naturalmente aprende sobre mais pares conforme participa da rede.

## O que a DHT armazena

A DHT armazena **ponteiros**, nao dados. Um ponteiro e um registro leve que diz "a tessera X esta com os nos Y e Z." Quando alguem quer recuperar uma tessera, primeiro busca seu ponteiro na DHT para descobrir quais nos a possuem, depois conecta diretamente a esses nos para baixar os dados reais.

Isso significa que a DHT permanece pequena e rapida — ela rastreia apenas quem tem o que, nao o conteudo em si.

## Identidade do no e prova de trabalho

Cada no tem um **ID de no** de 160 bits derivado de sua chave publica. Para evitar que um atacante crie milhares de nos falsos de forma barata (um **ataque Sybil**), gerar um ID de no requer uma pequena prova de trabalho: o no deve encontrar um nonce tal que `BLAKE3(chave_publica || nonce)` comece com 8 bits zero.

Isso leva cerca de 256 tentativas de hash — menos de um segundo em qualquer dispositivo, incluindo um Raspberry Pi. Mas um atacante tentando criar 10.000 identidades falsas precisaria de milhoes de tentativas, tornando o ataque impraticavel.

## Distancia XOR

Kademlia define "proximidade" entre nos usando a **metrica XOR**: a distancia entre dois IDs de no e seu XOR bit a bit. Os nos sao responsaveis por armazenar ponteiros cujas chaves estao proximas de seu proprio ID (em distancia XOR). Isso distribui dados uniformemente pela rede sem nenhuma coordenacao.

Ao buscar um ponteiro de tessera, seu no pergunta aos pares que conhece que estao mais proximos da chave alvo. Esses pares apontam para outros ainda mais proximos, e assim por diante, ate que o ponteiro seja encontrado. Essa **busca iterativa** tipicamente alcanca qualquer no na rede em poucos saltos.

## Transporte: QUIC

Toda comunicacao entre nos usa **QUIC**, um protocolo de transporte moderno construido sobre UDP. O QUIC oferece:

- **Criptografia integrada** — cada conexao usa TLS 1.3
- **Amigavel a NAT** — funciona atraves da maioria dos tradutores de endereco de rede por ser baseado em UDP
- **Multiplexacao** — multiplas operacoes independentes sobre uma conexao sem bloqueio head-of-line
- **Migracao de conexao** — sobrevive a mudancas de rede (ex: trocar de Wi-Fi para dados moveis)

O daemon escuta na porta UDP **4433** por padrao.

## Processo de bootstrap

Quando um no inicia, ele segue esta sequencia:

1. **Contactar nos semente** — conectar a um ou mais enderecos de bootstrap conhecidos
2. **Trocar pings** — verificar que o semente esta vivo e trocar identidades de no
3. **Auto-busca** — perguntar ao semente por nos proximos ao seu proprio ID, para popular sua tabela de roteamento
4. **Descoberta iterativa** — contactar os nos recem-descobertos, que apontam para ainda mais pares

Apos o bootstrap, o no mantem sua tabela de roteamento automaticamente: ele atualiza buckets periodicamente e substitui pares nao responsivos por novos.

## Tipos de no

Nem todo dispositivo participa da rede da mesma forma:

| Tipo | Descricao | Sempre ligado? |
|------|-----------|---------------|
| **No completo** | Desktop, servidor ou Raspberry Pi executando `tesd`. Participa plenamente da DHT e armazena dados de outros nos. | Sim |
| **No movel** | Celular ou tablet executando o app Tesseras. Participa da DHT quando o app esta ativo. | Nao |
| **No navegador** | Navegador web executando o cliente WASM. Conecta via um no relay. Somente leitura. | Nao |
| **No IoT** | ESP32 ou dispositivo similar na rede local. Armazena fragmentos passivamente, nao participa da DHT. | Sim |

O daemon de no completo e a espinha dorsal da rede. Quanto mais nos completos em execucao, mais resiliente a rede se torna.
