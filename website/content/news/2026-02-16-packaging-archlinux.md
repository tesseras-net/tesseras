+++
title = "Packaging Tesseras for Arch Linux"
date = 2026-02-16
description = "How to build and install the Tesseras package on Arch Linux from source using makepkg."
+++

Tesseras now ships a PKGBUILD for Arch Linux. This post walks through building and installing the package from source.

## Prerequisites

You need a working Rust toolchain and the base-devel group:

```sh
sudo pacman -S --needed base-devel sqlite
rustup toolchain install stable
```

## Building

Clone the repository and run the `just arch` recipe:

```sh
git clone https://git.sr.ht/~ijanc/tesseras
cd tesseras
just arch
```

This runs `makepkg -sf` inside `packaging/archlinux/`, which:

1. **prepare** — fetches Cargo dependencies with `cargo fetch --locked`
2. **build** — compiles `tesseras-daemon` and `tes` (the CLI) in release mode
3. **package** — installs binaries, systemd service, sysusers/tmpfiles configs, shell completions (bash, zsh, fish), and a default config file

The result is a `.pkg.tar.zst` file in `packaging/archlinux/`.

## Installing

```sh
sudo pacman -U packaging/archlinux/tesseras-*.pkg.tar.zst
```

## Post-install setup

The package creates a `tesseras` system user and group automatically via systemd-sysusers. To use the CLI without sudo, add yourself to the group:

```sh
sudo usermod -aG tesseras $USER
```

Log out and back in, then start the daemon:

```sh
sudo systemctl enable --now tesseras-daemon
```

## What the package includes

| Path | Description |
|------|-------------|
| `/usr/bin/tesseras-daemon` | Full node daemon |
| `/usr/bin/tes` | CLI client |
| `/etc/tesseras/config.toml` | Default configuration (marked as backup) |
| `/usr/lib/systemd/system/tesseras-daemon.service` | Systemd unit with security hardening |
| `/usr/lib/sysusers.d/tesseras.conf` | System user definition |
| `/usr/lib/tmpfiles.d/tesseras.conf` | Data directory `/var/lib/tesseras` |
| Shell completions | bash, zsh, and fish |

## PKGBUILD details

The PKGBUILD builds directly from the local git checkout rather than downloading a source tarball. The `TESSERAS_ROOT` environment variable points makepkg to the workspace root. Cargo's target directory is set to `$srcdir/target` to keep build artifacts inside the makepkg sandbox.

The package depends only on `sqlite` at runtime and `cargo` at build time.

## Updating

After pulling new changes, simply run `just arch` again and reinstall:

```sh
git pull
just arch
sudo pacman -U packaging/archlinux/tesseras-*.pkg.tar.zst
```
