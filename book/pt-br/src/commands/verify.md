# tes verify

Verificar integridade de uma tessera armazenada.

Alias: `tes v`

## Uso

```bash
tes verify <HASH>
```

## Argumentos

| Argumento | Descrição |
|-----------|-----------|
| `<HASH>` | Hash da tessera ou prefixo (base32 ou hex) |

Você pode usar o hash completo ou um prefixo curto. Ambos os formatos base32 e hex são aceitos.

## Opções

| Opção | Descrição |
|-------|-----------|
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados (padrão: `~/.local/share/tesseras`) |

## O que é verificado

1. **Validade da assinatura** — verifica a assinatura Ed25519 sobre o MANIFEST
2. **Integridade dos arquivos** — recalcula o hash BLAKE3 de cada arquivo e compara com o MANIFEST

## Códigos de saída

| Código | Significado |
|--------|-------------|
| `0` | Verificação passou — todos os arquivos intactos, assinatura válida |
| `1` | Verificação falhou — arquivos corrompidos ou assinatura inválida |

## Exemplos

### Verificação bem-sucedida

```bash
tes verify 9y2m4a
```

```
Tessera: 9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
Signature: VALID
  [OK] memories/a1b2c3d4/media.jpg
  [OK] memories/e5f6a7b8/media.txt
  [OK] memories/c9d0e1f2/media.wav
Verification: PASSED
```

### Verificação com falha

Se um arquivo foi modificado ou corrompido:

```
Tessera: 9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
Signature: VALID
  [OK] memories/a1b2c3d4/media.jpg
  [FAILED] memories/e5f6a7b8/media.txt
  [OK] memories/c9d0e1f2/media.wav
Verification: FAILED
```

## Casos de uso

- **Verificações rotineiras de integridade** — verifique periodicamente que suas tesseras armazenadas não foram corrompidas
- **Após transferência** — verifique após copiar tesseras para um novo dispositivo ou meio de armazenamento
- **Verificação de confiança** — confirme que uma tessera recebida de outra pessoa não foi adulterada
