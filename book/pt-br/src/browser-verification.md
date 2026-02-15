# Verificação no Navegador

Tesseras podem ser verificadas diretamente no navegador sem instalar nenhum software. O pacote `@tesseras/verify` executa verificação criptográfica inteiramente no lado do cliente usando WebAssembly.

## Como funciona

Quando você arrasta um arquivo tessera para uma página de verificação:

1. O arquivo (`.tar.gz`, `.zip` ou `.tar`) é descompactado no navegador
2. O MANIFEST é parseado para extrair a chave pública do criador, lista de arquivos e hashes esperados
3. A assinatura Ed25519 é verificada contra a chave pública do criador
4. O hash BLAKE3 de cada arquivo é computado e comparado com o MANIFEST
5. Um resultado detalhado mostra quais arquivos estão intactos e se as assinaturas são válidas

Tudo isso acontece em um Web Worker para manter a página responsiva. Atualizações de progresso são transmitidas de volta para a UI conforme cada arquivo é verificado.

## Resultado da verificação

O resultado inclui:

| Campo | Descrição |
|-------|-----------|
| `valid` | Aprovado/reprovado geral |
| `tessera_hash` | Hash BLAKE3 do MANIFEST |
| `signatures.ed25519` | `valid`, `invalid` ou `missing` |
| `signatures.ml_dsa` | `valid`, `invalid` ou `missing` (atualmente sempre `missing`) |
| `files` | Status por arquivo com hashes esperados e reais |
| `unexpected_files` | Arquivos no arquivo não listados no MANIFEST |
| `errors` | Erros encontrados durante a verificação |

## O que é verificado

- **Autenticidade da assinatura** — o MANIFEST foi assinado pela chave privada Ed25519 do criador
- **Integridade dos arquivos** — cada arquivo no MANIFEST tem o hash BLAKE3 correto
- **Completude** — todos os arquivos listados no MANIFEST estão presentes no arquivo
- **Sem extras** — arquivos não presentes no MANIFEST são marcados como inesperados

## O que NÃO é verificado

- **Identidade** — a verificação no navegador confirma que a tessera foi assinada por uma chave específica, mas não diz quem é o dono dessa chave. Você precisa de um meio externo para confirmar a chave pública do criador.
- **ML-DSA (pós-quântico)** — a verificação de assinatura pós-quântica ainda não está disponível no navegador. Assinaturas Ed25519 são verificadas.

## Usando o pacote npm

Para desenvolvedores integrando verificação em suas próprias aplicações:

```typescript
import { verifyTessera } from "@tesseras/verify";

const archive = new Uint8Array(/* bytes do arquivo tessera */);

const result = await verifyTessera(archive, (current, total, file) => {
  console.log(`Verificando ${file} (${current}/${total})`);
});

if (result.valid) {
  console.log("Tessera é autêntica e intacta");
} else {
  console.log("Verificação falhou:", result.errors);
}
```

## Comparação com verificação via CLI

| Recurso | `tes verify` (CLI) | Verificação no navegador |
|---------|-------------------|--------------------------|
| Assinaturas Ed25519 | Sim | Sim |
| Assinaturas ML-DSA | Sim (quando disponível) | Ainda não |
| Hashes BLAKE3 de arquivos | Sim | Sim |
| Requer instalação | Sim | Não |
| Funciona offline | Sim | Sim (após carregar a página) |
| Arquivos grandes | Sem limite | Limitado pela memória do navegador |
| Tamanho do binário WASM | N/A | 44 KB com gzip |

## Detalhes técnicos

O binário WASM é compilado a partir de Rust usando `wasm-pack`. Ele inclui:

- `blake3` — para hashing de integridade de arquivos
- `ed25519-dalek` — para verificação de assinaturas
- `tesseras-core` — para parsing do MANIFEST

O binário tem 109 KB bruto (44 KB com gzip). Ele não inclui `tesseras-crypto` ou qualquer dependência C — todas as operações criptográficas usam implementações em Rust puro.
