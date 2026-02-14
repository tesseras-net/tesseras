# Replicação e Reparo

Este capítulo explica como o Tesseras mantém suas memórias seguras mesmo quando nós individuais ficam offline ou sofrem falhas de hardware. Você não precisa entender esses detalhes para usar o Tesseras — o daemon cuida de tudo automaticamente.

## Por que a replicação importa

Uma tessera armazenada em uma única máquina morre quando essa máquina morre. O Tesseras resolve isso dividindo os dados em fragmentos, espalhando-os entre múltiplos pares e verificando continuamente que cópias suficientes existem. Se alguns fragmentos desaparecem, a rede se repara automaticamente.

## Codificação de apagamento

O Tesseras usa **codificação de apagamento Reed-Solomon** para criar fragmentos redundantes. A ideia é simples: a partir de N fragmentos de dados, gerar M fragmentos extras de paridade. Quaisquer N dos N+M fragmentos totais podem reconstruir os dados originais.

Isso é muito mais eficiente em armazenamento do que replicação simples. Armazenar 3 cópias completas de um arquivo de 100 MB custa 300 MB. Com 16 dados + 8 fragmentos de paridade, você obtém proteção mais forte (pode perder até 8 de 24 fragmentos — 33%) por apenas 150 MB no total.

## Camadas de fragmentação

Nem toda tessera é tratada da mesma forma. Arquivos pequenos não se beneficiam do overhead da codificação de apagamento, então o Tesseras usa três camadas:

| Camada | Tamanho | Estratégia | Fragmentos |
|--------|---------|------------|------------|
| **Small** | < 4 MB | Replicação do arquivo inteiro | 7 cópias do arquivo completo |
| **Medium** | 4–256 MB | Reed-Solomon 16+8 | 16 dados + 8 paridade = 24 fragmentos |
| **Large** | ≥ 256 MB | Reed-Solomon 48+24 | 48 dados + 24 paridade = 72 fragmentos |

Todas as camadas visam um **fator de replicação de 7** — significando que os fragmentos são distribuídos para 7 pares diferentes.

## Como a distribuição funciona

Quando você cria uma tessera e o daemon a replica, isto é o que acontece:

1. **Codificar** — os dados da tessera são divididos em fragmentos de acordo com sua camada de tamanho
2. **Encontrar pares** — o daemon consulta a DHT pelos nós mais próximos ao hash da tessera
3. **Diversidade de sub-rede** — os pares são filtrados para que poucos venham da mesma sub-rede (para evitar falhas correlacionadas se um datacenter cair)
4. **Distribuir** — os fragmentos são enviados aos pares selecionados em ordem round-robin
5. **Confirmar** — cada par valida o checksum do fragmento e confirma o recebimento

O dono da tessera envia os fragmentos aos pares. Os pares não puxam — isso mantém o protocolo simples e garante distribuição imediata.

## Verificação de fragmentos

Cada fragmento carrega um checksum BLAKE3. Quando um nó recebe um fragmento, ele recalcula o hash e compara com o checksum esperado. Se não coincidem, o fragmento é rejeitado. Isso detecta tanto erros de transmissão quanto adulteração deliberada.

Os fragmentos são armazenados em disco como arquivos individuais com um índice de metadados SQLite rastreando seus checksums, tamanhos e propriedade.

## Loop de reparo

O daemon executa um loop de reparo em segundo plano a cada 24 horas (com jitter aleatório para evitar tempestades em toda a rede). Para cada tessera sob sua responsabilidade, o loop de reparo:

1. **Solicita atestações** dos detentores conhecidos — cada detentor prova que ainda possui os fragmentos reportando seus checksums
2. **Recorre ao ping** se a atestação falhar — para distinguir entre "nó está offline" e "nó perdeu os dados"
3. **Verifica fragmentos locais** — verifica a integridade de quaisquer fragmentos armazenados localmente recalculando checksums BLAKE3
4. **Decide a ação**:
   - **Healthy** — todos os detentores responderam, todos os checksums válidos, nada a fazer
   - **Needs replication** — alguns detentores sumiram, encontrar novos pares e redistribuir fragmentos ausentes
   - **Corrupt local** — um fragmento local tem dados corrompidos, buscar uma substituição na rede

## Reciprocidade

O Tesseras usa um **livro-razão de reciprocidade bilateral** para garantir troca justa de armazenamento. Não há criptomoeda, não há blockchain, não há consenso global — cada nó simplesmente rastreia seu saldo com cada par localmente:

```
par_a: +500 MB  (eles armazenam 500 MB meus)
par_b: -200 MB  (eu armazeno 200 MB a mais deles do que eles armazenam meu)
par_c:    0 MB  (equilibrado)
```

As regras são simples:

- Armazene 1 GB na rede → você deveria armazenar aproximadamente 1 GB para outros
- Nós com saldo positivo (eles armazenam mais para você) recebem prioridade quando você precisa distribuir novos fragmentos
- Free riders perdem redundância gradualmente — seus fragmentos são despriorizados para reparo, mas nunca deletados
- Ao receber um fragmento, um nó verifica o déficit do remetente. Se o remetente deve muito armazenamento, o fragmento é rejeitado
- Nós institucionais (universidades, arquivos) podem operar altruisticamente com proporções desequilibradas

## Tamanho máximo de tessera

O tamanho máximo de uma tessera é **1 GB**. Este é um limite prático que mantém os tamanhos de fragmentos gerenciáveis e a replicação rápida. Para coleções maiores de memórias, crie múltiplas tesseras.

## Configuração

O comportamento de replicação do daemon pode ser ajustado através da configuração:

| Parâmetro | Padrão | Descrição |
|-----------|--------|-----------|
| Intervalo de reparo | 24 horas | Com que frequência o loop de reparo roda |
| Jitter de reparo | 2 horas | Atraso aleatório adicionado para evitar tempestades na rede |
| Transferências simultâneas | 4 | Máximo de transferências paralelas de fragmentos |
| Espaço livre mínimo | 1 GB | Parar de aceitar fragmentos abaixo deste limite |
| Tolerância de déficit | 256 MB | Déficit máximo de armazenamento antes de rejeitar fragmentos de um par |
| Limite por par | 1 GB | Armazenamento total máximo para qualquer par individual |
