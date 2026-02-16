# Docker

Tesseras fornece uma imagem Docker para executar o daemon em conteineres. Isso e util para servidores, testar redes com multiplos nos e ambientes de CI.

## Construindo a imagem

A partir da raiz do repositorio:

```bash
docker build -t tesd .
```

O Dockerfile multi-estagio usa `rust:1.85` para compilar e `debian:bookworm-slim` como base de execucao. A imagem resultante e pequena e contem apenas o binario do daemon e certificados CA.

## Executando um unico no

```bash
docker run -d \
  --name tesseras \
  -p 4433:4433/udp \
  tesd
```

Isso inicia um no que:
- Escuta na porta UDP 4433
- Faz bootstrap a partir dos nos semente padrao
- Armazena dados dentro do conteiner (efemero)

Para persistir dados entre reinicializacoes do conteiner, monte um volume:

```bash
docker run -d \
  --name tesseras \
  -p 4433:4433/udp \
  -v tesseras-data:/root/.local/share/tesseras \
  tesd
```

## Executando como no semente

Para executar um no semente que nao faz bootstrap de ninguem:

```bash
docker run -d \
  --name tesseras-seed \
  -p 4433:4433/udp \
  tesd --listen 0.0.0.0:4433 --bootstrap ""
```

## Rede multi-no com Docker Compose

O repositorio inclui um arquivo Docker Compose para testar uma rede de 3 nos:

```yaml
services:
  boot1:
    build: ../..
    command: ["--listen", "0.0.0.0:4433", "--bootstrap", ""]
    ports: ["4433:4433/udp"]

  boot2:
    build: ../..
    command: ["--listen", "0.0.0.0:4433", "--bootstrap", "boot1:4433"]
    depends_on: [boot1]

  client:
    build: ../..
    command: ["--listen", "0.0.0.0:4433", "--bootstrap", "boot2:4433"]
    depends_on: [boot2]
```

Iniciar a rede:

```bash
cd tests/smoke
docker compose up --build -d
```

Verificar que todos os nos estao executando:

```bash
docker compose logs --tail=5
```

Voce devera ver `daemon ready` nos logs de cada no, e `bootstrap successful` para `boot2` e `client`.

Parar a rede:

```bash
docker compose down
```

## Configuracao personalizada

Para usar um arquivo de configuracao com Docker, monte-o no conteiner:

```bash
docker run -d \
  --name tesseras \
  -p 4433:4433/udp \
  -v ./config.toml:/etc/tesseras/config.toml:ro \
  -v tesseras-data:/root/.local/share/tesseras \
  tesd --config /etc/tesseras/config.toml
```

Veja o capitulo [Configuracao](./configuration.md) para todas as opcoes disponiveis.
