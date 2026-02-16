# tes show

Mostrar informações detalhadas sobre uma tessera.

## Uso

```bash
tes show <HASH>
```

## Argumentos

| Argumento | Descrição |
|-----------|-----------|
| `<HASH>` | Hash da tessera ou prefixo (base32 ou hex) |

Você pode usar o hash completo ou um prefixo curto. Ambos os formatos base32 e hex são aceitos.

## Opções

| Opção | Descrição |
|-------|-----------|
| `--json` | Saída em formato JSON |
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados (padrão: `~/.local/share/tesseras`) |

## Saída

O comando exibe:

- **Hash da tessera** — hash de conteúdo completo em base32
- **Created** — data e hora de criação (UTC)
- **Visibility** — public, private, circle ou sealed
- **Language** — código de idioma (ex.: `en`, `pt-BR`)
- **Tags** — tags separadas por vírgula (se houver)
- **Location** — descrição do local (se houver)
- **Files** — lista de todos os arquivos com nome, tipo de memória e tamanho
- **Total size** — tamanho combinado de todos os arquivos
- **Signature** — status da assinatura Ed25519 (valid ou INVALID)

## Exemplos

### Mostrar detalhes de uma tessera

```bash
tes show 9y2m4a
```

```
Tessera: 9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
Created:    2026-02-14 15:30:00 UTC
Visibility: public
Language:   pt-BR
Tags:       familia, domingo
Location:   Casa

Files (3):
  media.jpg        Moment        128 KB
  media.jpg        Moment        144 KB
  media.txt        Reflection      1 KB

Total size: 273 KB
Signature:  valid
```

### Saída em JSON

```bash
tes show 9y2m4a --json
```

```json
{
  "hash": "9y2m4a1b3e7d8f0cabc123def456789012345678abcdef",
  "created_at": "2026-02-14T15:30:00+00:00",
  "visibility": "public",
  "memory_count": 3,
  "size_bytes": 279552,
  "total_file_size": 279552,
  "signature_valid": true,
  "language": "pt-BR",
  "tags": ["familia", "domingo"],
  "location": "Casa",
  "files": [
    {
      "path": "memories/a1b2c3d4/media.jpg",
      "mime_type": "image/jpeg",
      "size": 131072,
      "hash": "..."
    }
  ],
  "memories": [
    {
      "hash": "...",
      "memory_type": "Moment",
      "media_path": "memories/a1b2c3d4/media.jpg"
    }
  ]
}
```

## Casos de uso

- **Inspecionar antes de compartilhar** — revise metadados, tags e arquivos antes de publicar
- **Scripting** — use `--json` para direcionar detalhes da tessera para outras ferramentas
- **Auditoria** — verifique o status da assinatura e integridade dos arquivos rapidamente
