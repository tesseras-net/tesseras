# tes list

Listar todas as tesseras locais.

## Uso

```bash
tes list
```

## Opções

| Opção | Descrição |
|-------|-----------|
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados (padrão: `~/.tesseras`) |

## Saída

Exibe uma tabela com as seguintes colunas:

| Coluna | Descrição |
|--------|-----------|
| **Hash** | Primeiros 16 caracteres do hash de conteúdo |
| **Created** | Data de criação (AAAA-MM-DD) |
| **Memories** | Número de memórias na tessera |
| **Size** | Tamanho total (B, KB, MB ou GB) |
| **Visibility** | Nível de visibilidade (public, private ou circle) |

## Exemplo

```bash
tes list
```

```
Hash             Created     Memories  Size    Visibility
9f2c4a1b3e7d8f0c 2026-02-14         3  284 KB  public
a3b7c2d9e4f01823 2026-02-10         1   12 KB  private
f8e7d6c5b4a39201 2026-01-28        12    4 MB  public
```

## Banco de dados vazio

Se nenhuma tessera foi criada ainda:

```bash
tes list
```

```
No tesseras found.
```
