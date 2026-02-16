# Installation

Tesseras is currently available by building from source.

## Prerequisites

### Rust

Tesseras requires **Rust 1.85 or higher**. The recommended way to install Rust is via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, make sure `~/.cargo/bin` is in your `PATH`. The installer usually adds it automatically. Verify with:

```bash
rustc --version
cargo --version
```

If you already have Rust installed, update to the latest version:

```bash
rustup update stable
```

### SQLite

Tesseras uses SQLite for local storage. You have two options:

**Option 1: System SQLite (recommended)**

Install SQLite development libraries via your system package manager:

| Distribution | Command |
|-------------|---------|
| Arch Linux | `sudo pacman -S sqlite` |
| Debian / Ubuntu | `sudo apt install libsqlite3-dev` |
| Fedora | `sudo dnf install sqlite-devel` |
| Alpine | `apk add sqlite-dev` |
| macOS (Homebrew) | `brew install sqlite` |
| FreeBSD | `pkg install sqlite3` |
| OpenBSD | Included in the base system |

**Option 2: Bundled SQLite**

If you prefer not to install SQLite on your system, use the `bundled-sqlite` feature flag during compilation. This compiles SQLite alongside Tesseras:

```bash
cargo install --path crates/tesseras-cli --features bundled-sqlite
cargo install --path crates/tesd --features bundled-sqlite
```

### Optional tools

| Tool | Purpose | Installation |
|------|---------|-------------|
| [just](https://github.com/casey/just) | Run project build commands | `cargo install just` |
| [mdBook](https://rust-lang.github.io/mdBook/) | Build the documentation | `cargo install mdbook` |
| [Docker](https://docs.docker.com/get-docker/) | Run nodes in containers | See [Docker](./docker.md) |
| [Flutter](https://flutter.dev/docs/get-started/install) | Build the mobile/desktop app | See [Flutter App](#flutter-app) |

## Build from source

Clone the repository and install the binaries:

```bash
git clone https://git.sr.ht/~ijanc/tesseras
cd tesseras
cargo install --path crates/tesseras-cli
cargo install --path crates/tesd
```

Or, if you have `just` installed:

```bash
just install
```

This installs two binaries to `~/.cargo/bin/` and configures shell auto-completions:

- `tes` — CLI tool for creating, verifying, and exporting tesseras
- `tesd` — full node daemon that participates in the P2P network

## Verify installation

```bash
tes --help
```

You should see:

```
Create and preserve human memories

Usage: tes [OPTIONS] <COMMAND>

Commands:
  init    Initialize identity and local database
  create  Create a tessera from a directory of files
  verify  Verify integrity of a stored tessera
  export  Export tessera to a self-contained directory
  list    List local tesseras
  help    Print this message or the help of the given subcommand(s)

Options:
      --data-dir <DATA_DIR>  Base directory for data storage [default: ~/.tesseras]
  -h, --help                 Print help
```

## Shell completions

The `just install` command configures completions automatically. If you installed manually, generate completions for your shell:

```bash
# Fish
tes completions fish > ~/.config/fish/completions/tes.fish

# Zsh
tes completions zsh > "${XDG_DATA_HOME:-$HOME/.local/share}/zsh/site-functions/_tes"

# Bash
tes completions bash > "${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions/tes"
```

## Flutter App

To build the mobile or desktop app, you need additional dependencies:

### Flutter prerequisites

1. **Flutter SDK** — install following the [official guide](https://flutter.dev/docs/get-started/install)
2. **Rust** — already installed as above
3. **Platform dependencies:**

| Platform | Dependencies |
|----------|-------------|
| Android | Android SDK, Android NDK, Java 17+ |
| iOS | Xcode, CocoaPods |
| Linux desktop | GTK 3.0+, pkg-config (`sudo apt install libgtk-3-dev pkg-config` on Debian/Ubuntu) |
| macOS desktop | Xcode Command Line Tools |

### Build the app

```bash
cd apps/flutter
flutter pub get

# Linux desktop
flutter build linux --debug

# Android
flutter build apk --debug

# iOS
flutter build ios --debug

# Tests
flutter test
```

Or using `just` from the repository root:

```bash
just build-linux    # Linux desktop
just build-android  # Android APK
just test-flutter   # Tests
```

## Network ports

The Tesseras daemon uses QUIC (protocol over UDP). If you are behind a firewall, allow traffic on the port:

| Protocol | Port | Direction |
|----------|------|-----------|
| UDP | 4433 | Inbound and outbound |

## Next steps

- [Quick Start](./quick-start.md) — create your first tessera
- [Running a Node](./running-a-node.md) — configure and run the daemon
- [Configuration](./configuration.md) — configuration options
- [Docker](./docker.md) — run in containers
