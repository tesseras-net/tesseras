+++
title = "Empacotando o Tesseras para Debian"
date = 2026-02-16T10:00:00Z
description = "Como compilar e instalar o pacote .deb do Tesseras no Debian/Ubuntu usando cargo-deb."
+++

O Tesseras agora inclui um pacote `.deb` para Debian e Ubuntu. Este post explica como compilar e instalar o pacote a partir do código-fonte usando `cargo-deb`.

## Pré-requisitos

Você precisa de uma toolchain Rust funcional e das bibliotecas de sistema necessárias:

```sh
sudo apt install build-essential pkg-config libsqlite3-dev
rustup toolchain install stable
cargo install cargo-deb
```

## Compilando

Clone o repositório e execute a recipe `just deb`:

```sh
git clone https://git.sr.ht/~ijanc/tesseras
cd tesseras
just deb
```

Essa recipe faz três coisas:

1. **Compila** `tesd` (o daemon) e `tes` (o CLI) em modo release com `cargo build --release`
2. **Gera completions de shell** para bash, zsh e fish a partir do binário `tes`
3. **Empacota** tudo em um arquivo `.deb` com `cargo deb -p tesseras-daemon --no-build`

O resultado é um arquivo `.deb` em `target/debian/`.

## Instalando

```sh
sudo dpkg -i target/debian/tesseras-daemon_*.deb
```

Se houver dependências faltando, corrija com:

```sh
sudo apt install -f
```

## Configuração pós-instalação

O script `postinst` cria automaticamente um usuário de sistema `tesseras` e o diretório de dados `/var/lib/tesseras`. Para usar o CLI sem sudo, adicione seu usuário ao grupo:

```sh
sudo usermod -aG tesseras $USER
```

Faça logout e login novamente, depois inicie o daemon:

```sh
sudo systemctl enable --now tesd
```

## O que o pacote inclui

| Caminho | Descrição |
|---------|-----------|
| `/usr/bin/tesd` | Daemon do nó completo |
| `/usr/bin/tes` | Cliente CLI |
| `/etc/tesseras/config.toml` | Configuração padrão (marcado como conffile) |
| `/lib/systemd/system/tesd.service` | Unit systemd com hardening de segurança |
| Completions de shell | bash, zsh e fish |

## Como o cargo-deb funciona

Os metadados de empacotamento ficam em `crates/tesseras-daemon/Cargo.toml` na seção `[package.metadata.deb]`. Essa seção define:

- **depends** — dependências em tempo de execução: `libc6` e `libsqlite3-0`
- **assets** — arquivos incluídos no pacote (binários, config, unit systemd, completions de shell)
- **conf-files** — arquivos tratados como configuração (preservados na atualização)
- **maintainer-scripts** — scripts `postinst` e `postrm` em `packaging/debian/scripts/`
- **systemd-units** — integração automática com systemd

O script `postinst` cria o usuário de sistema `tesseras` e o diretório de dados na instalação. O script `postrm` remove o usuário, grupo e diretório de dados apenas no `purge` (não na remoção simples).

## Hardening do systemd

A unit `tesd.service` inclui diretivas de hardening de segurança:

```ini
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/tesseras
PrivateTmp=true
PrivateDevices=true
ProtectKernelTunables=true
ProtectControlGroups=true
RestrictSUIDSGID=true
MemoryDenyWriteExecute=true
```

O daemon roda como o usuário não-privilegiado `tesseras` e só pode escrever em `/var/lib/tesseras`.

## Deploy para um servidor remoto

O justfile inclui uma recipe `deploy` para enviar o `.deb` a um host remoto:

```sh
just deploy bootstrap1.tesseras.net
```

Isso compila o `.deb`, copia via `scp`, instala com `dpkg -i` e reinicia o serviço `tesd`.

## Atualizando

Depois de baixar novas mudanças, basta rodar `just deb` novamente e reinstalar:

```sh
git pull
just deb
sudo dpkg -i target/debian/tesseras-daemon_*.deb
```
