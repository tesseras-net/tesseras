# tesseras create

Criar uma tessera a partir de um diretório de arquivos.

## Uso

```bash
tesseras create <CAMINHO> [OPÇÕES]
```

## Argumentos

| Argumento | Descrição |
|-----------|-----------|
| `<CAMINHO>` | Diretório contendo os arquivos a incluir |

## Opções

| Opção | Descrição | Padrão |
|-------|-----------|--------|
| `-n, --non-interactive` | Pular prompts | desativado |
| `--dry-run` | Pré-visualizar o que seria incluído | desativado |
| `--visibility <VALOR>` | Nível de visibilidade: `public`, `private`, `circle` | `public` |
| `--language <CÓDIGO>` | Código de idioma (ex.: `en`, `pt-BR`) | `en` |
| `--tags <LISTA>` | Tags separadas por vírgula | nenhuma |
| `--location <DESC>` | Descrição do local | nenhuma |
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados | `~/.tesseras` |

## Formatos de arquivo suportados

| Extensão | Tipo | Tipo de memória |
|----------|------|-----------------|
| `.jpg`, `.jpeg` | Imagem (JPEG) | Momento |
| `.png` | Imagem (PNG) | Momento |
| `.wav` | Áudio (WAV PCM) | Momento |
| `.webm` | Vídeo (WebM) | Momento |
| `.txt` | Texto puro (UTF-8) | Reflexão |

Arquivos com outras extensões são ignorados.

## Inferência de tipo de memória

O comando atribui automaticamente um tipo de memória baseado no formato do arquivo:

- **Arquivos de texto** (`.txt`) são classificados como **Reflexão** — pensamentos, crenças ou opiniões
- **Todos os outros formatos** são classificados como **Momento** — uma foto, gravação ou vídeo de algo acontecendo

## Exemplos

### Pré-visualizar antes de criar

```bash
tesseras create ./minhas-fotos --dry-run
```

### Criar com metadados

```bash
tesseras create ./ferias-2026 \
    --tags "ferias,verao,praia" \
    --location "Florianópolis, Brasil" \
    --language pt-BR \
    --visibility public
```

### Modo não-interativo

```bash
tesseras create ./diario --non-interactive --tags "cotidiano"
```

## Níveis de visibilidade

| Nível | Quem pode acessar |
|-------|-------------------|
| `public` | Qualquer pessoa (padrão) |
| `private` | Apenas você (e herdeiros designados) |
| `circle` | Pessoas explicitamente escolhidas |

## O que acontece internamente

1. Varre o diretório em busca de arquivos suportados
2. Calcula um hash BLAKE3 para cada arquivo
3. Atribui um tipo de memória baseado na extensão do arquivo
4. Gera um MANIFEST listando todos os arquivos com seus checksums
5. Assina o MANIFEST com sua chave privada Ed25519
6. Armazena os arquivos e metadados no banco de dados local
7. Exibe o hash de conteúdo que identifica unicamente esta tessera
