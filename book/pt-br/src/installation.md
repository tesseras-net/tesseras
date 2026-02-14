# Instalação

Tesseras está disponível atualmente através de compilação a partir do código-fonte.

## Pré-requisitos

### Rust

Tesseras requer **Rust 1.85 ou superior**. A maneira recomendada de instalar o Rust é via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Após a instalação, certifique-se de que `~/.cargo/bin` está no seu `PATH`. O instalador normalmente adiciona isso automaticamente. Verifique com:

```bash
rustc --version
cargo --version
```

Se você já tem o Rust instalado, atualize para a versão mais recente:

```bash
rustup update stable
```

### SQLite

Tesseras usa SQLite para armazenamento local. Você tem duas opções:

**Opção 1: SQLite do sistema (recomendada)**

Instale as bibliotecas de desenvolvimento do SQLite pelo gerenciador de pacotes do seu sistema:

| Distribuição | Comando |
|-------------|---------|
| Arch Linux | `sudo pacman -S sqlite` |
| Debian / Ubuntu | `sudo apt install libsqlite3-dev` |
| Fedora | `sudo dnf install sqlite-devel` |
| Alpine | `apk add sqlite-dev` |
| macOS (Homebrew) | `brew install sqlite` |
| FreeBSD | `pkg install sqlite3` |
| OpenBSD | Já incluído no sistema base |

**Opção 2: SQLite embutido**

Se preferir não instalar o SQLite no sistema, use a feature flag `bundled-sqlite` durante a compilação. Isso compila o SQLite junto com o Tesseras:

```bash
cargo install --path crates/tesseras-cli --features bundled-sqlite
cargo install --path crates/tesseras-daemon --features bundled-sqlite
```

### Ferramentas opcionais

| Ferramenta | Para quê | Instalação |
|-----------|----------|------------|
| [just](https://github.com/casey/just) | Executar comandos de build do projeto | `cargo install just` |
| [mdBook](https://rust-lang.github.io/mdBook/) | Compilar a documentação | `cargo install mdbook` |
| [Docker](https://docs.docker.com/get-docker/) | Executar nós em contêineres | Veja [Docker](./docker.md) |
| [Flutter](https://flutter.dev/docs/get-started/install) | Compilar o app mobile/desktop | Veja [App Flutter](#app-flutter) |

## Compilar a partir do código-fonte

Clone o repositório e instale os binários:

```bash
git clone https://git.sr.ht/~ijanc/tesseras
cd tesseras
cargo install --path crates/tesseras-cli
cargo install --path crates/tesseras-daemon
```

Ou, se você tiver o `just` instalado:

```bash
just install
```

Isso instala dois binários em `~/.cargo/bin/` e configura auto-completions para o seu shell:

- `tes` — ferramenta CLI para criar, verificar e exportar tesseras
- `tesseras-daemon` — daemon de nó completo que participa da rede P2P

## Verificar a instalação

```bash
tes --help
```

Você deverá ver:

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

## Auto-completions do shell

O comando `just install` configura completions automaticamente. Se você instalou manualmente, gere as completions para o seu shell:

```bash
# Fish
tes completions fish > ~/.config/fish/completions/tes.fish

# Zsh
tes completions zsh > "${XDG_DATA_HOME:-$HOME/.local/share}/zsh/site-functions/_tes"

# Bash
tes completions bash > "${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions/tes"
```

## App Flutter

Para compilar o app mobile ou desktop, você precisa de dependências adicionais:

### Pré-requisitos do Flutter

1. **Flutter SDK** — instale seguindo o [guia oficial](https://flutter.dev/docs/get-started/install)
2. **Rust** — já instalado conforme acima
3. **Dependências de plataforma:**

| Plataforma | Dependências |
|-----------|-------------|
| Android | Android SDK, Android NDK, Java 17+ |
| iOS | Xcode, CocoaPods |
| Linux desktop | GTK 3.0+, pkg-config (`sudo apt install libgtk-3-dev pkg-config` no Debian/Ubuntu) |
| macOS desktop | Xcode Command Line Tools |

### Compilar o app

```bash
cd apps/flutter
flutter pub get

# Linux desktop
flutter build linux --debug

# Android
flutter build apk --debug

# iOS
flutter build ios --debug

# Testes
flutter test
```

Ou usando `just` a partir da raiz do repositório:

```bash
just build-linux    # Linux desktop
just build-android  # Android APK
just test-flutter   # Testes
```

## Portas de rede

O daemon Tesseras usa QUIC (protocolo sobre UDP). Se você estiver atrás de um firewall, permita tráfego na porta:

| Protocolo | Porta | Direção |
|----------|-------|---------|
| UDP | 4433 | Entrada e saída |

## Próximos passos

- [Início Rápido](./quick-start.md) — crie sua primeira tessera
- [Executando um Nó](./running-a-node.md) — configure e execute o daemon
- [Configuração](./configuration.md) — opções de configuração
- [Docker](./docker.md) — execute em contêineres
