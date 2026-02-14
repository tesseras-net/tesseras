# Configuracao

O daemon pode ser configurado via um arquivo TOML. Passe o caminho com `--config`:

```bash
tesseras-daemon --config /etc/tesseras/config.toml
```

Se nenhum arquivo de configuracao for fornecido, o daemon usa padroes sensiveis. Opcoes CLI (`--listen`, `--bootstrap`, `--data-dir`) sobrescrevem os valores correspondentes da configuracao.

## Exemplo completo

```toml
[node]
data_dir = "~/.local/share/tesseras"
listen_addr = "0.0.0.0:4433"

[dht]
k = 20
alpha = 3
bucket_refresh_interval_secs = 3600
republish_interval_secs = 3600
pointer_ttl_secs = 86400
max_stored_pointers = 100000
ping_failure_threshold = 3

[bootstrap]
dns_domain = "_tesseras._udp.tesseras.net"
hardcoded = [
    "boot1.tesseras.net:4433",
    "boot2.tesseras.net:4433",
]

[network]
enable_mdns = true

[observability]
metrics_addr = "127.0.0.1:9190"
log_format = "json"
```

## Secoes

### `[node]`

Configuracoes basicas do no.

| Chave | Tipo | Padrao | Descricao |
|-------|------|--------|-----------|
| `data_dir` | caminho | Especifico da plataforma | Onde armazenar identidade, banco de dados e blobs |
| `listen_addr` | endereco | `0.0.0.0:4433` | Endereco do listener QUIC |

O `data_dir` padrao e `~/.local/share/tesseras` no Linux e `~/Library/Application Support/tesseras` no macOS.

### `[dht]`

Parametros de ajuste da DHT Kademlia. Os padroes funcionam bem para a maioria das implantacoes.

| Chave | Tipo | Padrao | Descricao |
|-------|------|--------|-----------|
| `k` | inteiro | `20` | Maximo de entradas por bucket da tabela de roteamento |
| `alpha` | inteiro | `3` | Paralelismo para buscas iterativas |
| `bucket_refresh_interval_secs` | inteiro | `3600` | Com que frequencia atualizar buckets da tabela de roteamento (segundos) |
| `republish_interval_secs` | inteiro | `3600` | Com que frequencia republicar ponteiros armazenados (segundos) |
| `pointer_ttl_secs` | inteiro | `86400` | Quanto tempo manter um ponteiro antes de expirar (segundos) |
| `max_stored_pointers` | inteiro | `100000` | Numero maximo de ponteiros armazenados localmente |
| `ping_failure_threshold` | inteiro | `3` | Quantas falhas consecutivas de ping antes de remover um par |

### `[bootstrap]`

Como o no descobre seus primeiros pares ao entrar na rede.

| Chave | Tipo | Padrao | Descricao |
|-------|------|--------|-----------|
| `dns_domain` | string | `_tesseras._udp.tesseras.net` | Dominio DNS para descoberta de pares via registros TXT |
| `hardcoded` | lista de strings | `["boot1.tesseras.net:4433", "boot2.tesseras.net:4433"]` | Enderecos de bootstrap de fallback |

### `[network]`

Funcionalidades de nivel de rede.

| Chave | Tipo | Padrao | Descricao |
|-------|------|--------|-----------|
| `enable_mdns` | booleano | `true` | Habilitar descoberta na rede local via mDNS |

### `[observability]`

Monitoramento e logging.

| Chave | Tipo | Padrao | Descricao |
|-------|------|--------|-----------|
| `metrics_addr` | endereco | `127.0.0.1:9190` | Endereco para o endpoint de metricas Prometheus |
| `log_format` | string | `json` | Formato de saida de log (`json` ou `text`) |

## Configuracao minima

A maioria dos usuarios nao precisa de um arquivo de configuracao. Se precisar, uma configuracao minima sobrescrevendo apenas o necessario e suficiente:

```toml
[node]
listen_addr = "0.0.0.0:5000"

[bootstrap]
hardcoded = ["192.168.1.10:4433"]
```

Todos os outros valores usam seus padroes.
