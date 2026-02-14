# Installation

Tesseras is currently available by building from source.

## Prerequisites

- **Rust 1.85+** — install via [rustup](https://rustup.rs/)
- **SQLite** — usually available via your system package manager

## Build from source

Clone the repository and install the CLI binary:

```bash
git clone https://git.sr.ht/~ijanc/tesseras
cd tesseras
cargo install --path crates/tesseras-cli
```

This installs the `tesseras` binary to `~/.cargo/bin/`.

## Verify installation

```bash
tesseras --help
```

You should see:

```
Create and preserve human memories

Usage: tesseras [OPTIONS] <COMMAND>

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
