# tes verify

Verificar integridade de uma tessera armazenada.

## Uso

```bash
tes verify <HASH>
```

## Argumentos

| Argumento | Descrição |
|-----------|-----------|
| `<HASH>` | Hash de conteúdo da tessera (64 caracteres hexadecimais) |

## Opções

| Opção | Descrição |
|-------|-----------|
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados (padrão: `~/.tesseras`) |

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
tes verify 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
```

```
Tessera: 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
Signature: VALID
  [OK] memories/a1b2c3d4/media.jpg
  [OK] memories/e5f6a7b8/media.txt
  [OK] memories/c9d0e1f2/media.wav
Verification: PASSED
```

### Verificação com falha

Se um arquivo foi modificado ou corrompido:

```
Tessera: 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
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
