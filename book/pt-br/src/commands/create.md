# tes create

Criar uma tessera a partir de um diretório de arquivos.

Alias: `tes c`

## Uso

```bash
tes create <CAMINHO> [OPÇÕES]
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
| `--sealed` | Criar uma tessera selada (bloqueada por tempo) | desativado |
| `--open-after <DATA>` | Data de abertura da tessera selada (AAAA-MM-DD, requer `--sealed`) | nenhuma |
| `--language <CÓDIGO>` | Código de idioma (ex.: `en`, `pt-BR`) | `en` |
| `--tags <LISTA>` | Tags separadas por vírgula | nenhuma |
| `--location <DESC>` | Descrição do local | nenhuma |
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados | `~/.local/share/tesseras` |

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

## Níveis de visibilidade

| Nível | Quem pode acessar |
|-------|-------------------|
| `public` | Qualquer pessoa (padrão) |
| `private` | Apenas você (e herdeiros designados) — requer chaves de criptografia |
| `circle` | Pessoas explicitamente escolhidas |
| `sealed` | Abre após uma data especificada — requer chaves de criptografia e `--sealed --open-after` |

Tesseras privadas e seladas requerem chaves de criptografia (X25519 + ML-KEM-768). Se estiverem ausentes, execute `tes init --upgrade` para gerá-las.

## Exemplos

### Pré-visualizar antes de criar

```bash
tes create ./minhas-fotos --dry-run
```

```
Dry run — files that would be included:
  ./minhas-fotos/praia.jpg (Moment)
  ./minhas-fotos/notas.txt (Reflection)
```

### Criar com metadados

```bash
tes create ./ferias-2026 \
    --tags "ferias,verao,praia" \
    --location "Florianópolis, Brasil" \
    --language pt-BR \
    --visibility public
```

```
Created tessera: 9y2m4a1b3e7d8f0cabc1
```

### Modo não-interativo

```bash
tes create ./diario --non-interactive --tags "cotidiano"
```

### Criar uma tessera selada (bloqueada por tempo)

```bash
tes create ./capsula-do-tempo \
    --sealed \
    --open-after 2050-01-01 \
    --tags "futuro"
```

A tessera é criptografada e não pode ser lida até 01/01/2050.

## O que acontece internamente

1. Varre o diretório em busca de arquivos suportados
2. Calcula um hash BLAKE3 para cada arquivo
3. Atribui um tipo de memória baseado na extensão do arquivo
4. Gera um MANIFEST listando todos os arquivos com seus checksums
5. Assina o MANIFEST com sua chave privada Ed25519
6. Para tesseras privadas/seladas: criptografa o conteúdo das memórias com AES-256-GCM, sela a chave de conteúdo com criptografia híbrida (X25519 + ML-KEM-768)
7. Armazena os arquivos e metadados no banco de dados local
8. Exibe o hash de conteúdo em codificação base32
