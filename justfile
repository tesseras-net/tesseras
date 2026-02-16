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

# Check for outdated and vulnerable dependencies
deps-check:
    cargo audit
    cargo outdated --root-deps-only
    cargo deny check advisories

# Submit dependency check job to SourceHut builds
deps-check-submit:
    infra/scripts/srht-deps-check.sh

# Enable SourceHut patch validation (auto-runs on push)
ci-enable:
    mv .builds/validate-patch.yml.todo .builds/validate-patch.yml
    @echo "Enabled: .builds/validate-patch.yml"

# Disable SourceHut patch validation
ci-disable:
    mv .builds/validate-patch.yml .builds/validate-patch.yml.todo
    @echo "Disabled: .builds/validate-patch.yml.todo"

# Run dependency security audit
audit:
    cargo audit
    cargo deny check

# Run fuzz tests (proptest backend, stable Rust, ~60s per target)
audit-fuzz:
    cargo test -p tesseras-crypto --test fuzz_shamir --features shamir
    cargo test -p tesseras-crypto --test fuzz_erasure --features erasure
    cargo test -p tesseras-crypto --test fuzz_sealed --features encryption

# Run mutation testing on security-critical files
audit-mutants:
    cargo mutants --file crates/tesseras-crypto/src/dual.rs -p tesseras-crypto -- --all-features
    cargo mutants --file crates/tesseras-crypto/src/sealed.rs -p tesseras-crypto --features encryption

# Run full audit suite (audit + fuzz + mutants)
audit-full: audit audit-fuzz audit-mutants

# Generate full CHANGELOG.md from all tags
changelog:
    git cliff --output CHANGELOG.md

# Preview changelog for unreleased commits
changelog-preview:
    git cliff --unreleased

# Build .deb package for tesseras-daemon (static MUSL binary)
deb:
    cargo deb -p tesseras-daemon --target x86_64-unknown-linux-musl -- --features bundled-sqlite

# Build Arch Linux package (.pkg.tar.zst), then install with: sudo pacman -U packaging/archlinux/tesseras-*.pkg.tar.zst
arch:
    cd packaging/archlinux && TESSERAS_ROOT="$(git rev-parse --show-toplevel)" makepkg -sf

# Deploy .deb to a bootstrap node via scp + dpkg
deploy host="bootstrap1.tesseras.net":
    #!/usr/bin/env bash
    set -euo pipefail
    just deb
    DEB=$(ls -t target/debian/tesseras-daemon_*.deb | head -1)
    echo "Deploying $DEB to {{host}}..."
    scp "$DEB" root@{{host}}:/tmp/
    ssh root@{{host}} "dpkg -i /tmp/$(basename $DEB) && systemctl daemon-reload && systemctl restart tesseras-daemon && rm /tmp/$(basename $DEB)"
    echo "Deployed and restarted tesseras-daemon on {{host}}"

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
