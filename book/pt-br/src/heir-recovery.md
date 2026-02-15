# Recuperação de Chaves por Herdeiros

Suas tesseras podem sobreviver a falhas de infraestrutura, computadores quânticos e séculos de tempo. Mas o que acontece quando você não consegue mais acessar suas próprias chaves? Tesseras usa **Shamir's Secret Sharing** para permitir que você distribua sua identidade criptográfica para herdeiros de confiança.

## Como funciona

Shamir's Secret Sharing divide um segredo em N fragmentos com um limiar T. Qualquer T fragmentos podem reconstruir o segredo original. Menos que T fragmentos não revelam **nada** — isso é informação-teoricamente seguro, não apenas computacionalmente difícil de quebrar.

Por exemplo, com limiar 2 e 3 fragmentos totais:
- Dê o fragmento 1 ao seu cônjuge
- Dê o fragmento 2 ao seu irmão
- Dê o fragmento 3 ao seu advogado

Quaisquer dois deles podem recuperar sua identidade. Um único fragmento sozinho é inútil.

## Criando fragmentos de herdeiros

```bash
tes heir create --threshold 2 --shares 3
```

Isso divide sua chave de identidade Ed25519 em 3 fragmentos (necessitando 2 para reconstruir) e os salva em `./heir-shares/`:

```
heir-shares/
├── heir_share_1.bin   # Binário MessagePack
├── heir_share_1.txt   # Texto base64 legível por humanos
├── heir_share_2.bin
├── heir_share_2.txt
├── heir_share_3.bin
└── heir_share_3.txt
```

Cada fragmento é gerado em dois formatos:
- **Binário** (`.bin`) — MessagePack compacto, adequado para pendrives ou armazenamento digital
- **Texto** (`.txt`) — base64 com cabeçalho legível, adequado para impressão em papel

O formato texto se parece com isso:

```
--- TESSERAS HEIR SHARE ---
Format: v1
Owner: a1b2c3d4e5f6a7b8 (fingerprint)
Share: 1 of 3 (threshold: 2)
Session: 9f8e7d6c5b4a3210
Created: 2026-02-15

<dados codificados em base64>
--- END HEIR SHARE ---
```

## Reconstruindo a partir de fragmentos

Quando os herdeiros precisam recuperar a identidade:

```bash
tes heir reconstruct heir_share_1.txt heir_share_2.bin --output-dir ./recovered-keys
```

O comando detecta automaticamente se cada arquivo é formato binário ou texto. Ele valida que todos os fragmentos pertencem à mesma sessão e dono, verifica checksums e reconstrói o par de chaves Ed25519.

Para instalar as chaves recuperadas como identidade ativa:

```bash
tes heir reconstruct share1.txt share2.txt --output-dir ./recovered --install
```

Isso faz backup da identidade atual antes de substituí-la.

## Inspecionando um fragmento

Para ver metadados sobre um fragmento sem expor dados secretos:

```bash
tes heir info heir_share_1.txt
```

Saída:
```
Heir Share Information:
  Format version: 1
  Share: 1 of 3 (threshold: 2)
  Session: 9f8e7d6c5b4a3210
  Owner fingerprint: a1b2c3d4e5f6a7b8
  Share data size: 34 bytes
  Checksum: valid
```

## Considerações de segurança

- **Escolha do limiar**: um limiar de 2-de-3 ou 3-de-5 é recomendado para a maioria das pessoas. Limiares mais altos são mais seguros mas requerem mais herdeiros para cooperar.
- **Armazenamento físico**: imprima os arquivos `.txt` em papel livre de ácido e armazene em locais físicos separados (cofres bancários, casas diferentes). Papel sobrevive décadas sem degradação.
- **Nunca armazene fragmentos juntos**: todo o propósito da divisão é a distribuição. Manter todos os fragmentos em um lugar anula o objetivo.
- **Isolamento de sessão**: cada chamada `heir create` gera um novo ID de sessão. Fragmentos de sessões diferentes não podem ser misturados — isso previne confusão após rotações de chave.
- **Verificação de checksum**: cada fragmento inclui um checksum BLAKE3. Fragmentos corrompidos (erros de OCR, degradação de bits) são detectados antes de qualquer tentativa de reconstrução.
- **Re-dividir após mudanças de chave**: se você regenerar sua identidade, crie novos fragmentos de herdeiros e destrua com segurança os antigos.

## Princípios de design

- **Segurança informação-teórica** — T-1 fragmentos revelam exatamente zero informação sobre o segredo. Isso não é uma suposição computacional; é matematicamente provado.
- **Detecção de corrupção** — checksums BLAKE3 detectam degradação de bits, erros de OCR e truncamento antes de qualquer tentativa de reconstrução.
- **Resiliência de formato** — saída dupla (binário + texto) garante que fragmentos sobrevivam a diferentes modos de falha de mídia de armazenamento.
- **Compatibilidade retroativa** — o blob do segredo é versionado, para que versões futuras possam incluir material de chave adicional sem quebrar fragmentos existentes.
