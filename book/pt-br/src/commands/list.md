# tes list

Listar todas as tesseras locais.

Alias: `tes ls`

## Uso

```bash
tes list
```

## Opções

| Opção | Descrição |
|-------|-----------|
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados (padrão: `~/.local/share/tesseras`) |

## Exemplos

### Listar tesseras

```bash
tes list
```

```
┌────────────┬────────────┬──────────┬────────┬────────────┐
│ Hash       │ Created    │ Memories │ Size   │ Visibility │
├────────────┼────────────┼──────────┼────────┼────────────┤
│ 9y2m4a1b3e │ 2026-02-14 │        3 │ 284 KB │ public     │
│ f7g8h9j0kl │ 2026-02-15 │        1 │  12 KB │ private    │
└────────────┴────────────┴──────────┴────────┴────────────┘
```

A coluna hash mostra os primeiros 10 caracteres do hash de conteúdo codificado em base32. Use esse prefixo com outros comandos (ex.: `tes verify 9y2m4a1b3e`).

### Banco de dados vazio

```bash
tes list
```

```
No tesseras found.
```

## Referência de colunas

| Coluna | Descrição |
|--------|-----------|
| Hash | Primeiros 10 caracteres do hash de conteúdo base32 |
| Created | Data de criação da tessera (AAAA-MM-DD) |
| Memories | Número de memórias na tessera |
| Size | Tamanho total de todos os arquivos (B, KB, MB, GB) |
| Visibility | Nível de visibilidade: public, private, circle ou sealed |
