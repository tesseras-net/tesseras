# tes init

Inicializar identidade e banco de dados local.

## Uso

```bash
tes init
```

## Descrição

Configura seu ambiente Tesseras local. Este é o primeiro comando que você deve executar após instalar o Tesseras.

O comando cria:

| Caminho | Conteúdo |
|---------|----------|
| `~/.tesseras/identity/` | Par de chaves Ed25519 para assinar tesseras |
| `~/.tesseras/db/` | Banco de dados SQLite para indexação |
| `~/.tesseras/blobs/` | Armazenamento de blobs para arquivos de memória |
| `~/.tesseras/config.toml` | Arquivo de configuração |

## Opções

| Opção | Descrição |
|-------|-----------|
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados (padrão: `~/.tesseras`) |

## Idempotente

Executar `init` novamente é seguro. Se uma identidade já existe, ela é preservada:

```bash
tes init
```

```
Ed25519 identity already exists
Database initialized
Tesseras initialized at /home/user/.tesseras
```

## Diretório de dados personalizado

```bash
tes --data-dir /mnt/usb/tesseras init
```

Isso cria toda a estrutura de diretórios em `/mnt/usb/tesseras/` ao invés do local padrão.

## O que acontece internamente

1. Cria a estrutura de diretórios (`identity/`, `db/`, `blobs/`)
2. Gera um par de chaves Ed25519 (a chave privada permanece local, a chave pública identifica você)
3. Executa as migrações SQLite para configurar o esquema do banco de dados
4. Escreve um `config.toml` padrão
