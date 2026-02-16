# Configuracao

O daemon pode ser configurado via um arquivo TOML. Passe o caminho com `--config`:

```bash
tesd --config /etc/tesseras/config.toml
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

## Suporte a IPv6

Tesseras suporta IPv6 nativamente. Os campos `listen_addr` e `listen_addrs` aceitam tanto enderecos IPv4 quanto IPv6.

### Escutando em IPv6

Para escutar em todas as interfaces IPv6:

```toml
[node]
listen_addr = "[::]:4433"
```

No Linux e na maioria dos BSDs, vincular a `[::]` tambem aceita conexoes IPv4 (dual-stack) por padrao. Em alguns sistemas (notavelmente OpenBSD), `[::]` e somente IPv6 porque `IPV6_V6ONLY` e habilitado por padrao. Para garantir tanto IPv4 quanto IPv6 em todas as plataformas, use `listen_addrs` com enderecos explicitos:

```toml
[node]
listen_addrs = ["0.0.0.0:4433", "[::]:4433"]
```

Para loopback IPv6 apenas (testes):

```toml
[node]
listen_addr = "[::1]:4433"
```

### Bootstrap com IPv6

Enderecos de bootstrap podem ser IPv6:

```toml
[bootstrap]
hardcoded = [
    "boot1.tesseras.net:4433",
    "[2001:db8::1]:4433",
]
```

Hostnames DNS com registros A e AAAA sao resolvidos para todos os enderecos, entao o daemon se conectara pelo protocolo que estiver acessivel.

### Comportamento de `IPV6_V6ONLY` por SO

| SO | `[::]` aceita IPv4? | Notas |
|----|---------------------|-------|
| Linux | Sim (dual-stack) | `IPV6_V6ONLY` padrao 0 |
| macOS | Sim (dual-stack) | `IPV6_V6ONLY` padrao 0 |
| FreeBSD | Sim (dual-stack) | `IPV6_V6ONLY` padrao 0 |
| OpenBSD | Nao (somente IPv6) | `IPV6_V6ONLY` sempre 1 |
| Windows | Sim (dual-stack) | `IPV6_V6ONLY` padrao 0 |

Se precisar de controle explicito, use `listen_addrs` com um endereco IPv4 e um IPv6.

## Configuracao minima

A maioria dos usuarios nao precisa de um arquivo de configuracao. Se precisar, uma configuracao minima sobrescrevendo apenas o necessario e suficiente:

```toml
[node]
listen_addr = "0.0.0.0:5000"

[bootstrap]
hardcoded = ["192.168.1.10:4433"]
```

Todos os outros valores usam seus padroes.
