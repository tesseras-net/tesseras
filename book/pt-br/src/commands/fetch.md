# tes fetch

Buscar uma tessera da rede e armazená-la localmente.

## Uso

```bash
tes fetch <HASH>
```

## Argumentos

| Argumento | Descrição |
|-----------|-----------|
| `<HASH>` | Hash completo da tessera (64 caracteres hexadecimais) |

Diferente de `publish` e `status`, o `fetch` requer o hash completo de 64 caracteres porque a tessera ainda não existe localmente e não pode ser resolvida a partir de um prefixo.

## Opções

| Opção | Descrição |
|-------|-----------|
| `--socket <CAMINHO>` | Caminho para o socket Unix do daemon |
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados |

## Pré-requisitos

O daemon tesseras deve estar em execução e conectado à rede:

```bash
tesd
```

## Exemplos

### Buscar uma tessera

```bash
tes fetch 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
```

```
Fetching tessera 9f2c4a1b from network...
Fetched tessera 9f2c4a1b (3 memories, 1.2 MB)
```

### Buscar com socket personalizado

```bash
tes fetch 9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01 \
    --socket /tmp/my-daemon.sock
```

## O que acontece internamente

1. Conecta ao daemon via socket Unix
2. O daemon procura fragmentos para este hash no armazenamento local
3. Para tesseras pequenas: o fragmento único contém todos os dados
4. Para tesseras maiores: coleta fragmentos suficientes e reconstrói os dados originais usando decodificação de apagamento Reed-Solomon
5. Desempacota o buffer de bytes em arquivos individuais (MANIFEST, memórias, blobs)
6. Armazena cada arquivo no armazenamento endereçável por conteúdo (CAS) local
7. Retorna o número de memórias e o tamanho total buscado
