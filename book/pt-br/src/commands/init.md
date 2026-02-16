# tes init

Inicializar identidade e banco de dados local.

## Uso

```bash
tes init [OPÇÕES]
```

## Descrição

Configura seu ambiente Tesseras local. Este é o primeiro comando que você deve executar após instalar o Tesseras.

O comando cria:

| Caminho | Conteúdo |
|---------|----------|
| `identity/node.ed25519.pub` | Chave pública Ed25519 (sua identidade) |
| `identity/node.ed25519.key` | Chave secreta Ed25519 (assinatura) |
| `identity/node.x25519.pub` | Chave pública X25519 (troca de chaves) |
| `identity/node.x25519.key` | Chave secreta X25519 (criptografia) |
| `identity/node.mlkem768.pub` | Chave pública ML-KEM-768 (encapsulamento pós-quântico) |
| `identity/node.mlkem768.key` | Chave secreta ML-KEM-768 (criptografia pós-quântica) |
| `db/tesseras.db` | Banco de dados SQLite para indexação |
| `blobs/` | Armazenamento de blobs para arquivos de memória |
| `config.toml` | Arquivo de configuração |

Todos os caminhos são relativos ao diretório de dados (padrão: `~/.local/share/tesseras`).

## Opções

| Opção | Descrição |
|-------|-----------|
| `--upgrade` | Adicionar chaves de criptografia a uma identidade existente |
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados (padrão: `~/.local/share/tesseras`) |

## Tipos de chave

**Ed25519** — Chave de assinatura clássica de curva elíptica. Usada para assinar o MANIFEST de cada tessera, provando autoria. Esta é sua identidade primária na rede.

**X25519** — Troca de chaves Diffie-Hellman, derivada da Curve25519. Usada para criptografar tesseras privadas e seladas para que apenas você (e seus herdeiros) possam lê-las.

**ML-KEM-768** — Mecanismo de encapsulamento de chave pós-quântico (anteriormente CRYSTALS-Kyber). Pareado com X25519 em um esquema híbrido para que suas tesseras criptografadas permaneçam seguras mesmo se computadores quânticos de grande escala forem construídos no futuro.

## Idempotente

Executar `init` novamente é seguro. Chaves existentes são preservadas:

```bash
tes init
```

```
Ed25519 identity already exists
Encryption keys already exist
Database initialized
Tesseras initialized at /home/user/.local/share/tesseras
```

## Atualizar identidade existente

Se você inicializou antes das chaves de criptografia estarem disponíveis, use `--upgrade` para adicioná-las sem alterar sua identidade Ed25519:

```bash
tes init --upgrade
```

```
Generating encryption keypair (X25519 + ML-KEM-768)...
Generated encryption keypair
Tesseras initialized at /home/user/.local/share/tesseras
```

A atualização é atômica — se a chave ML-KEM-768 falhar ao salvar, a chave X25519 é revertida para que você nunca fique com uma identidade de criptografia parcial.

## Diretório de dados personalizado

```bash
tes --data-dir /mnt/usb/tesseras init
```

Isso cria toda a estrutura de diretórios em `/mnt/usb/tesseras/` ao invés do local padrão.

## Migração de dados antigos

Se dados existentes forem encontrados em `~/.tesseras` (o local padrão anterior) enquanto o diretório de dados atual é diferente, um aviso é exibido:

```
Note: found existing data at /home/user/.tesseras. Consider moving it to /home/user/.local/share/tesseras
```

## O que acontece internamente

1. Cria a estrutura de diretórios (`identity/`, `db/`, `blobs/`)
2. Gera um par de chaves Ed25519 (a chave privada permanece local, a chave pública identifica você)
3. Gera um par de chaves de criptografia híbrido (X25519 + ML-KEM-768) atomicamente
4. Executa as migrações SQLite para configurar o esquema do banco de dados (modo WAL)
5. Escreve um `config.toml` padrão
