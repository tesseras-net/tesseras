alias i := install

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

# Run NAT traversal tests
test-nat:
    cargo test -p tesseras-net stun -- --nocapture
    cargo test -p tesseras-net punch -- --nocapture
    cargo test -p tesseras-net relay -- --nocapture
    cargo test -p tesseras-net --test punch_integration -- --nocapture
    cargo test -p tesseras-net --test relay_integration -- --nocapture
    cargo test -p tesseras-dht -- --nocapture

# Run chaos tests (requires Docker with tc netem)
test-chaos:
    @echo "Chaos tests require Docker. See docs/plans/2026-02-15-nat-traversal-design.md"
    @echo "TODO: implement Docker Compose chaos test environment"

# Test the embedded crate
test-embedded:
    cargo test -p tesseras-embedded

# Build Flutter app for Linux desktop
build-linux:
    cd apps/flutter && flutter build linux --debug

# Build Flutter app for Android
build-android:
    cd apps/flutter && flutter build apk --debug

# Run Flutter widget tests
test-flutter:
    cd apps/flutter && flutter test

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
