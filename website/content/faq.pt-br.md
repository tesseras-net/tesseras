+++
title = "FAQ"
description = "Perguntas frequentes sobre o Tesseras"
+++

### O que é uma tessera?

Uma tessera é uma cápsula do tempo autocontida de memórias — fotos, gravações de áudio, vídeo e texto — empacotada em um formato projetado para sobreviver independentemente de qualquer software, empresa ou infraestrutura. O nome vem das pequenas peças usadas em mosaicos romanos: cada peça é simples, mas juntas formam algo que perdura.

### Como meus dados sobrevivem se meu computador morrer?

Sua tessera é replicada em múltiplos nós na rede peer-to-peer do Tesseras. Utiliza codificação por apagamento (Reed-Solomon) para dividir seus dados em fragmentos redundantes. Mesmo que vários nós fiquem offline permanentemente, sua tessera pode ser reconstruída a partir dos fragmentos restantes.

### Meus dados são criptografados?

Por padrão, não. O Tesseras prioriza disponibilidade sobre sigilo — o objetivo é que suas memórias sobrevivam, mesmo que o software para descriptografá-las não exista mais. Você pode marcar memórias individuais como privadas (criptografadas com AES-256-GCM) ou seladas (para serem abertas após uma data específica), mas memórias públicas e de círculo são armazenadas sem criptografia para maximizar suas chances de sobrevivência a longo prazo.

### Preciso pagar alguma coisa?

Não. A rede funciona com ajuda mútua: você armazena fragmentos das tesseras de outras pessoas, e elas armazenam as suas. Não há tokens, blockchain ou taxas de assinatura. O único custo é o espaço de armazenamento que você contribui para a rede.

### Em quais plataformas funciona?

Tesseras funciona em Linux, macOS, FreeBSD, OpenBSD, Windows, Android e iOS. Também há um visualizador no navegador e suporte para dispositivos IoT de baixo consumo (ESP32) como nós de armazenamento passivo.

### Qual a diferença do IPFS, Filecoin ou Arweave?

Tesseras é projetado especificamente para preservação de memórias pessoais, não armazenamento de arquivos de propósito geral. Diferenças principais:

- **Sem criptomoeda ou tokens** — incentivos são baseados em reciprocidade bilateral, não mercados financeiros
- **Formato autodescritivo** — cada tessera inclui instruções para decodificar a si mesma em múltiplos idiomas, para que possa ser compreendida séculos no futuro sem nenhum software especial
- **Disponibilidade sobre sigilo** — a maioria dos dados é armazenada sem criptografia para maximizar a sobrevivência a longo prazo
- **Formatos de mídia mais simples possíveis** — JPEG, WAV, WebM, texto puro — escolhidos por durabilidade, não recursos

### Quais formatos de mídia são suportados?

- **Fotos:** JPEG
- **Áudio:** WAV PCM
- **Vídeo:** WebM
- **Texto:** UTF-8 texto puro

Esses formatos foram escolhidos por máxima longevidade e amplo suporte.

### Posso exportar minha tessera?

Sim. Uma tessera é um diretório padrão de arquivos. Você pode copiá-la para um pendrive, gravar em mídia óptica ou imprimir as partes de texto. O formato é projetado para ser legível sem nenhum software especial.
