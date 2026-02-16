# tes publish

Publicar uma tessera na rede para replicação e preservação de longo prazo.

## Uso

```bash
tes publish <HASH>
```

## Argumentos

| Argumento | Descrição |
|-----------|-----------|
| `<HASH>` | Hash de conteúdo da tessera ou prefixo |

Prefixos de hash são suportados — se o hash da sua tessera começa com `a1b2c3`, você pode usar `tes publish a1b2c3` e o CLI resolverá o hash completo a partir do banco de dados local.

## Opções

| Opção | Descrição |
|-------|-----------|
| `--socket <CAMINHO>` | Caminho para o socket Unix do daemon |
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados |

## Pré-requisitos

O daemon tesseras deve estar em execução:

```bash
tesseras-daemon
```

A tessera deve existir localmente (criada com `tes create`).

## Exemplos

### Publicar pelo hash completo

```bash
tes publish 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
```

```
Published tessera 9f2c4a1b (2 fragments created)
Distribution in progress — use `tes status 9f2c4a1b` to track.
```

### Publicar por prefixo curto

```bash
tes publish 9f2c
```

```
Published tessera 9f2c4a1b (24 fragments created)
Distribution in progress — use `tes status 9f2c4a1b` to track.
```

### Publicar com socket personalizado

```bash
tes publish a1b2 --socket /tmp/my-daemon.sock
```

## O que acontece internamente

1. Resolve o prefixo do hash no banco de dados local para encontrar o hash completo
2. Conecta ao daemon via socket Unix
3. O daemon lê todos os arquivos da tessera (MANIFEST, assinaturas, memórias, blobs) do armazenamento local
4. Empacota tudo em um único buffer de bytes usando serialização MessagePack
5. Para tesseras pequenas (< 4 MB): replica os dados brutos como um único fragmento para r=7 peers
6. Para tesseras maiores: aplica codificação de apagamento Reed-Solomon, produzindo fragmentos redundantes
7. Distribui os fragmentos pela rede via DHT, priorizando peers com reciprocidade positiva
8. Retorna o número de fragmentos criados
