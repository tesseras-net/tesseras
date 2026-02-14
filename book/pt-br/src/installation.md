# Instalação

Tesseras está disponível atualmente através de compilação a partir do código-fonte.

## Pré-requisitos

- **Rust 1.85+** — instale via [rustup](https://rustup.rs/)
- **SQLite** — geralmente disponível pelo gerenciador de pacotes do seu sistema

## Compilar a partir do código-fonte

Clone o repositório e instale os binários:

```bash
git clone https://git.sr.ht/~ijanc/tesseras
cd tesseras
cargo install --path crates/tesseras-cli
cargo install --path crates/tesseras-daemon
```

Isso instala dois binários em `~/.cargo/bin/`:

- `tesseras` — ferramenta CLI para criar, verificar e exportar tesseras
- `tesseras-daemon` — daemon de nó completo que participa da rede P2P

## Verificar a instalação

```bash
tesseras --help
```

Você deverá ver:

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
