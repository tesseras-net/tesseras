# Docker

Tesseras provides a Docker image for running the daemon in containers. This is useful for servers, testing multi-node networks, and CI environments.

## Building the image

From the repository root:

```bash
docker build -t tesd .
```

The multi-stage Dockerfile uses `rust:1.85` to compile and `debian:bookworm-slim` as the runtime base. The resulting image is small and contains only the daemon binary and CA certificates.

## Running a single node

```bash
docker run -d \
  --name tesseras \
  -p 4433:4433/udp \
  tesd
```

This starts a node that:
- Listens on UDP port 4433
- Bootstraps from the default seed nodes
- Stores data inside the container (ephemeral)

To persist data across container restarts, mount a volume:

```bash
docker run -d \
  --name tesseras \
  -p 4433:4433/udp \
  -v tesseras-data:/root/.local/share/tesseras \
  tesd
```

## Running as a seed node

To run a seed node that doesn't bootstrap from anyone else:

```bash
docker run -d \
  --name tesseras-seed \
  -p 4433:4433/udp \
  tesd --listen 0.0.0.0:4433 --bootstrap ""
```

## Multi-node network with Docker Compose

The repository includes a Docker Compose file for testing a 3-node network:

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

Start the network:

```bash
cd tests/smoke
docker compose up --build -d
```

Check that all nodes are running:

```bash
docker compose logs --tail=5
```

You should see `daemon ready` in the logs for each node, and `bootstrap successful` for `boot2` and `client`.

Stop the network:

```bash
docker compose down
```

## Custom configuration

To use a config file with Docker, mount it into the container:

```bash
docker run -d \
  --name tesseras \
  -p 4433:4433/udp \
  -v ./config.toml:/etc/tesseras/config.toml:ro \
  -v tesseras-data:/root/.local/share/tesseras \
  tesd --config /etc/tesseras/config.toml
```

See the [Configuration](./configuration.md) chapter for all available options.
