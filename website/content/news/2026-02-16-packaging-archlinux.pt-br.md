+++
title = "Empacotando o Tesseras para Arch Linux"
date = 2026-02-16
description = "Como compilar e instalar o pacote Tesseras no Arch Linux a partir do código-fonte usando makepkg."
+++

O Tesseras agora inclui um PKGBUILD para Arch Linux. Este post explica como compilar e instalar o pacote a partir do código-fonte.

## Pré-requisitos

Você precisa de uma toolchain Rust funcional e do grupo base-devel:

```sh
sudo pacman -S --needed base-devel sqlite
rustup toolchain install stable
```

## Compilando

Clone o repositório e execute a recipe `just arch`:

```sh
git clone https://git.sr.ht/~ijanc/tesseras
cd tesseras
just arch
```

Isso executa `makepkg -sf` dentro de `packaging/archlinux/`, que:

1. **prepare** — baixa as dependências Cargo com `cargo fetch --locked`
2. **build** — compila `tesseras-daemon` e `tes` (o CLI) em modo release
3. **package** — instala binários, serviço systemd, configs sysusers/tmpfiles, completions de shell (bash, zsh, fish) e um arquivo de configuração padrão

O resultado é um arquivo `.pkg.tar.zst` em `packaging/archlinux/`.

## Instalando

```sh
sudo pacman -U packaging/archlinux/tesseras-*.pkg.tar.zst
```

## Configuração pós-instalação

O pacote cria automaticamente um usuário e grupo de sistema `tesseras` via systemd-sysusers. Para usar o CLI sem sudo, adicione seu usuário ao grupo:

```sh
sudo usermod -aG tesseras $USER
```

Faça logout e login novamente, depois inicie o daemon:

```sh
sudo systemctl enable --now tesseras-daemon
```

## O que o pacote inclui

| Caminho | Descrição |
|---------|-----------|
| `/usr/bin/tesseras-daemon` | Daemon do nó completo |
| `/usr/bin/tes` | Cliente CLI |
| `/etc/tesseras/config.toml` | Configuração padrão (marcado como backup) |
| `/usr/lib/systemd/system/tesseras-daemon.service` | Unit systemd com hardening de segurança |
| `/usr/lib/sysusers.d/tesseras.conf` | Definição do usuário de sistema |
| `/usr/lib/tmpfiles.d/tesseras.conf` | Diretório de dados `/var/lib/tesseras` |
| Completions de shell | bash, zsh e fish |

## Detalhes do PKGBUILD

O PKGBUILD compila diretamente a partir do checkout git local em vez de baixar um tarball. A variável de ambiente `TESSERAS_ROOT` aponta o makepkg para a raiz do workspace. O diretório target do Cargo é configurado para `$srcdir/target` para manter os artefatos de build dentro do sandbox do makepkg.

O pacote depende apenas de `sqlite` em tempo de execução e `cargo` em tempo de build.

## Atualizando

Depois de baixar novas mudanças, basta rodar `just arch` novamente e reinstalar:

```sh
git pull
just arch
sudo pacman -U packaging/archlinux/tesseras-*.pkg.tar.zst
```
