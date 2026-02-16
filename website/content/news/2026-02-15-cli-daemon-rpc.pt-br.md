+++
title = "CLI Encontra a Rede: Comandos Publish, Fetch e Status"
date = 2026-02-15
description = "O CLI do tesseras agora pode publicar tesseras na rede, buscá-las de peers e monitorar o estado de replicação — tudo através de uma nova ponte RPC via socket Unix para o daemon."
+++

Até agora o CLI operava isoladamente: criar uma tessera, verificar, exportar, listar o que você tem. Tudo ficava na sua máquina. Com esta atualização, o `tes` ganha três comandos que fazem a ponte entre o armazenamento local e a rede P2P — `publish`, `fetch` e `status` — comunicando-se com um `tesd` em execução através de um socket Unix.

## O que foi construído

**Crate `tesseras-rpc`** — Um novo crate compartilhado entre CLI e daemon. Define o protocolo RPC usando serialização MessagePack com enquadramento prefixado por tamanho (cabeçalho big-endian de 4 bytes, máximo de 64 MiB). Três tipos de requisição (`Publish`, `Fetch`, `Status`) e suas respostas correspondentes. Um `DaemonClient` síncrono gerencia a conexão do socket Unix com timeouts configuráveis. O protocolo é deliberadamente simples — uma requisição, uma resposta, conexão fechada — para manter a implementação auditável.

**`tes publish <hash>`** — Publica uma tessera na rede. Aceita hashes completos ou prefixos curtos (ex.: `tes publish a1b2`), que são resolvidos no banco de dados local. O daemon lê todos os arquivos da tessera do armazenamento, empacota em um único buffer MessagePack e entrega ao motor de replicação. Tesseras pequenas (< 4 MB) são replicadas como um único fragmento; maiores passam por codificação de apagamento Reed-Solomon. A saída mostra o hash curto e a contagem de fragmentos:

```
Published tessera 9f2c4a1b (24 fragments created)
Distribution in progress — use `tes status 9f2c4a1b` to track.
```

**`tes fetch <hash>`** — Busca uma tessera da rede usando o hash de conteúdo completo. O daemon coleta fragmentos disponíveis localmente, reconstrói os dados originais via decodificação de apagamento se necessário, desempacota os arquivos e armazena no CAS (content-addressable store). Retorna o número de memórias e o tamanho total buscado.

**`tes status <hash>`** — Exibe a saúde de replicação de uma tessera. A saída mapeia diretamente o modelo interno de saúde do motor de replicação:

| Estado | Significado |
|--------|-------------|
| Local | Ainda não publicada — existe apenas na sua máquina |
| Publishing | Fragmentos sendo distribuídos, redundância crítica |
| Replicated | Distribuída, mas abaixo da redundância alvo |
| Healthy | Redundância completa alcançada |

**Listener RPC no daemon** — O daemon agora escuta em um socket Unix (padrão: `$XDG_RUNTIME_DIR/tesseras/daemon.sock`) com permissões de diretório adequadas (0700), limpeza de sockets obsoletos e shutdown gracioso. Cada conexão é tratada em uma task Tokio — o listener converte o stream assíncrono para I/O síncrono para a camada de enquadramento, despacha para o handler RPC e escreve a resposta de volta.

**Pack/unpack no `tesseras-core`** — Um módulo pequeno que serializa uma lista de entradas de arquivo (caminho + dados) em um único buffer MessagePack e vice-versa. Esta é a ponte entre a estrutura de diretórios da tessera e os blobs opacos do motor de replicação.

## Decisões de arquitetura

- **Socket Unix ao invés de TCP**: a comunicação RPC entre CLI e daemon acontece na mesma máquina. Sockets Unix são mais rápidos, não precisam de alocação de porta, e as permissões do sistema de arquivos fornecem controle de acesso sem TLS.
- **MessagePack ao invés de JSON**: o mesmo formato wire usado em todo o Tesseras. Compacto, sem schema, e já é uma dependência do workspace. Uma ida-e-volta típica de publish request/response ocupa menos de 200 bytes.
- **Cliente síncrono, daemon assíncrono**: o `DaemonClient` usa I/O bloqueante porque o CLI não precisa de concorrência — envia uma requisição e espera. O listener do daemon é assíncrono (Tokio) para tratar múltiplas conexões. A camada de enquadramento funciona com qualquer impl `Read`/`Write`, conectando ambos os mundos.
- **Resolução de prefixo no lado do cliente**: `publish` e `status` resolvem prefixos curtos localmente antes de enviar o hash completo ao daemon. Isso mantém o daemon stateless — ele não precisa acessar o banco de dados do CLI.
- **Alinhamento do diretório de dados padrão**: o padrão do CLI mudou de `~/.tesseras` para `~/.local/share/tesseras` (via `dirs::data_dir()`) para coincidir com o daemon. Um aviso de migração é exibido quando dados no caminho antigo são detectados.

## Próximos passos

- **Contagem de peers no DHT**: o comando `status` atualmente reporta 0 peers — conectar a contagem real do DHT é o próximo passo
- **`tes show`**: exibir o conteúdo de uma tessera (memórias, metadados) sem exportar
- **Fetch com streaming**: para tesseras grandes, transmitir fragmentos conforme chegam ao invés de esperar por todos
