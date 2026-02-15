# Criptografia e Tesseras Seladas

A maioria das tesseras são públicas — projetadas para serem acessíveis a qualquer pessoa, para sempre. Mas algumas memórias precisam de privacidade. Tesseras suporta dois modos de visibilidade criptografada:

- **Privada** — apenas o criador (e seus herdeiros) podem acessar o conteúdo
- **Selada** — o conteúdo é bloqueado por tempo e se torna acessível após uma data específica

Tesseras públicas nunca são criptografadas. Disponibilidade é mais importante que sigilo para preservação.

## Como a criptografia funciona

Quando você cria uma tessera privada ou selada, o seguinte acontece:

1. Uma **chave de conteúdo** aleatória (256 bits) é gerada
2. Cada arquivo de memória é criptografado com **AES-256-GCM** usando essa chave de conteúdo
3. A chave de conteúdo é envolvida em um **envelope de chave selada** usando sua chave pública de criptografia
4. A chave envolvida é armazenada junto ao conteúdo criptografado

Apenas o detentor da chave privada correspondente pode desembrulhar a chave de conteúdo e decriptar o conteúdo.

## Encapsulamento de chave híbrido pós-quântico

O envelope de chave selada usa um **Mecanismo de Encapsulamento de Chave (KEM) híbrido** combinando dois algoritmos:

- **X25519** — uma troca de chaves clássica bem testada baseada em curva elíptica
- **ML-KEM-768** — um KEM pós-quântico baseado em reticulados padronizado pelo NIST (anteriormente Kyber)

Ambos os algoritmos produzem segredos compartilhados que são combinados usando derivação de chaves BLAKE3. Um atacante precisa quebrar **ambos** os algoritmos para recuperar a chave de conteúdo. Isso segue o mesmo princípio das assinaturas duplas do Tesseras (Ed25519 + ML-DSA): não sabemos quais suposições criptográficas se manterão ao longo dos séculos, então apostamos nos dois.

## Dados autenticados associados (AAD)

AES-256-GCM suporta dados autenticados associados — informações extras que são verificadas durante a decriptação mas não são criptografadas. Tesseras vincula as seguintes informações no AAD:

- O **hash do conteúdo** da tessera (sempre)
- O **timestamp open_after** (apenas para tesseras seladas)

Isso previne **ataques de troca de texto cifrado**: um atacante não pode copiar conteúdo criptografado de uma tessera para outra, porque o AAD não vai corresponder e a decriptação vai falhar. Para tesseras seladas, isso também significa que você não pode alterar a data do selo — o timestamp está criptograficamente vinculado ao texto cifrado.

## Tesseras seladas: cápsulas do tempo

Uma tessera selada é uma verdadeira cápsula do tempo. Quando você cria uma, você especifica uma data `open_after`. O conteúdo é criptografado e a chave é selada em um envelope que apenas você pode abrir.

Quando a data `open_after` passa, o dono publica a chave de conteúdo como uma **Publicação de Chave** assinada — um artefato independente contendo a chave, o hash da tessera e a assinatura do dono. Outros nós podem verificar a assinatura e usar a chave publicada para decriptar o conteúdo.

O manifesto da tessera nunca é modificado. A Publicação de Chave é um documento separado, preservando a natureza imutável e endereçada por conteúdo das tesseras.

## E as chaves?

Cada identidade agora inclui um **par de chaves de criptografia** junto ao par de chaves de assinatura:

| Tipo de chave | Algoritmo | Finalidade |
|---------------|-----------|------------|
| Ed25519 | Clássico | Assinatura de manifestos e publicações de chave |
| ML-DSA | Pós-quântico | Assinatura (quando habilitado) |
| X25519 | Clássico | Encapsulamento de chave (criptografia) |
| ML-KEM-768 | Pós-quântico | Encapsulamento de chave (criptografia) |

O par de chaves de criptografia é gerado quando a identidade é criada. A metade pública é armazenada no diretório de identidade da tessera; a metade privada fica no dispositivo do dono.

## Princípios de design

- **Criptografar o mínimo possível** — apenas conteúdo privado e selado é criptografado. Memórias públicas permanecem abertas para acessibilidade a longo prazo.
- **Algoritmos duplos desde o início** — criptografia clássica e pós-quântica, para que o conteúdo esteja protegido mesmo que um algoritmo seja quebrado.
- **Manifestos imutáveis** — chaves são publicadas separadamente, nunca modificando dados existentes.
- **Falhar fechado** — o sistema rejeita tentativas de criar tesseras privadas ou seladas sem chaves de criptografia.
