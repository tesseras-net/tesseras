# tes status

Mostrar o estado de replicação de uma tessera na rede.

## Uso

```bash
tes status <HASH>
```

## Argumentos

| Argumento | Descrição |
|-----------|-----------|
| `<HASH>` | Hash de conteúdo da tessera ou prefixo |

Prefixos de hash são suportados, assim como no `tes publish`.

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

A tessera deve existir localmente.

## Estados de replicação

| Estado | Significado |
|--------|-------------|
| `Local (not published)` | A tessera existe apenas na sua máquina — ainda não foi publicada |
| `Publishing...` | Fragmentos estão sendo distribuídos, nível crítico de redundância |
| `Replicated` | Fragmentos distribuídos, mas abaixo da redundância alvo |
| `Healthy` | Todos os fragmentos colocados, redundância completa alcançada |

## Exemplos

### Verificar estado de uma tessera

```bash
tes status 9f2c4a1b
```

```
Tessera:     9f2c4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef01
State:       Healthy
Fragments:   24/24 placed
Peers:       0 holding copies
```

### Tessera não publicada

```bash
tes status a1b2
```

```
Tessera:     a1b2c3d4e5f6a7b89012345678abcdef0123456789abcdef0123456789abcdef
State:       Local (not published)
Fragments:   0/0 placed
Peers:       0 holding copies
```

## O que acontece internamente

1. Resolve o prefixo do hash no banco de dados local
2. Conecta ao daemon via socket Unix
3. O daemon verifica que a tessera existe no armazenamento local
4. Consulta o motor de replicação sobre a saúde dos fragmentos:
   - **Healthy**: todos os fragmentos vivos e com redundância alvo
   - **Degraded** (exibido como Replicated): alguns fragmentos ausentes, mas acima do limiar crítico
   - **Critical** (exibido como Publishing): abaixo da redundância mínima, reparo ativo necessário
5. Se nenhum fragmento existe, o estado é `Local`
6. Retorna o estado, contagem de fragmentos e número de peers
