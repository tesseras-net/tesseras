[private]
default:
    @just --list --unsorted

mod website
mod book

# Install the CLI, daemon, and shell completions
install:
    cargo install --path crates/tesseras-cli
    cargo install --path crates/tesseras-daemon
    @just _install-completions

[private]
_install-completions:
    #!/usr/bin/env sh
    set -eu
    SHELL_NAME="$(basename "$SHELL")"
    case "$SHELL_NAME" in
        fish)
            DIR="$HOME/.config/fish/completions"
            mkdir -p "$DIR"
            tes completions fish > "$DIR/tes.fish"
            echo "Installed fish completions to $DIR/tes.fish"
            ;;
        zsh)
            DIR="${XDG_DATA_HOME:-$HOME/.local/share}/zsh/site-functions"
            mkdir -p "$DIR"
            tes completions zsh > "$DIR/_tes"
            echo "Installed zsh completions to $DIR/_tes"
            echo "Make sure $DIR is in your \$fpath"
            ;;
        bash)
            DIR="${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions"
            mkdir -p "$DIR"
            tes completions bash > "$DIR/tes"
            echo "Installed bash completions to $DIR/tes"
            ;;
        *)
            echo "Unknown shell '$SHELL_NAME' — skipping completions"
            echo "You can manually run: tes completions <shell>"
            ;;
    esac
