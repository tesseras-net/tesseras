+++
title = "Packaging Tesseras for Debian"
date = 2026-02-16T10:00:00Z
description = "How to build and install the Tesseras .deb package on Debian/Ubuntu using cargo-deb."
+++

Tesseras now ships a `.deb` package for Debian and Ubuntu. This post walks through building and installing the package from source using `cargo-deb`.

## Prerequisites

You need a working Rust toolchain and the required system libraries:

```sh
sudo apt install build-essential pkg-config libsqlite3-dev
rustup toolchain install stable
cargo install cargo-deb
```

## Building

Clone the repository and run the `just deb` recipe:

```sh
git clone https://git.sr.ht/~ijanc/tesseras
cd tesseras
just deb
```

This recipe does three things:

1. **Compiles** `tesd` (the daemon) and `tes` (the CLI) in release mode with `cargo build --release`
2. **Generates shell completions** for bash, zsh, and fish from the `tes` binary
3. **Packages** everything into a `.deb` file with `cargo deb -p tesseras-daemon --no-build`

The result is a `.deb` file in `target/debian/`.

## Installing

```sh
sudo dpkg -i target/debian/tesseras-daemon_*.deb
```

If there are missing dependencies, fix them with:

```sh
sudo apt install -f
```

## Post-install setup

The `postinst` script automatically creates a `tesseras` system user and the data directory `/var/lib/tesseras`. To use the CLI without sudo, add yourself to the group:

```sh
sudo usermod -aG tesseras $USER
```

Log out and back in, then start the daemon:

```sh
sudo systemctl enable --now tesd
```

## What the package includes

| Path | Description |
|------|-------------|
| `/usr/bin/tesd` | Full node daemon |
| `/usr/bin/tes` | CLI client |
| `/etc/tesseras/config.toml` | Default configuration (marked as conffile) |
| `/lib/systemd/system/tesd.service` | Systemd unit with security hardening |
| Shell completions | bash, zsh, and fish |

## How cargo-deb works

The packaging metadata lives in `crates/tesseras-daemon/Cargo.toml` under `[package.metadata.deb]`. This section defines:

- **depends** — runtime dependencies: `libc6` and `libsqlite3-0`
- **assets** — files to include in the package (binaries, config, systemd unit, shell completions)
- **conf-files** — files treated as configuration (preserved on upgrade)
- **maintainer-scripts** — `postinst` and `postrm` scripts in `packaging/debian/scripts/`
- **systemd-units** — automatic systemd integration

The `postinst` script creates the `tesseras` system user and data directory on install. The `postrm` script cleans up the user, group, and data directory only on `purge` (not on simple removal).

## Systemd hardening

The `tesd.service` unit includes security hardening directives:

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

The daemon runs as the unprivileged `tesseras` user and can only write to `/var/lib/tesseras`.

## Deploying to a remote server

The justfile includes a `deploy` recipe for pushing the `.deb` to a remote host:

```sh
just deploy bootstrap1.tesseras.net
```

This builds the `.deb`, copies it via `scp`, installs it with `dpkg -i`, and restarts the `tesd` service.

## Updating

After pulling new changes, simply run `just deb` again and reinstall:

```sh
git pull
just deb
sudo dpkg -i target/debian/tesseras-daemon_*.deb
```
