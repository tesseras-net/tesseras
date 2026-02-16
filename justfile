alias i := install

[private]
default:
    @just --list --unsorted

mod website
mod book

# Install the CLI, daemon, and shell completions
install:
    cargo install --path crates/tesseras-cli
    cargo install --path crates/tesseras-daemon --bin tesd
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

# Build Flutter Windows installer (.exe) via QEMU VM
build-windows host="localhost" port="2222" user="ijanc":
    #!/usr/bin/env bash
    set -euo pipefail
    SSH="ssh -p {{port}} {{user}}@{{host}}"
    SCP="scp -P {{port}}"
    ROOT="$(git rev-parse --show-toplevel)"
    RELEASE_DIR='C:\tesseras\apps\flutter\build\windows\x64\runner\Release'
    VERSION="0.1.0"

    echo "==> Syncing source to Windows VM..."
    TARBALL="/tmp/tesseras-win-src.tar.gz"
    tar czf "$TARBALL" --exclude='target' --exclude='.dart_tool' --exclude='build' --exclude='.git' -C "$(dirname "$ROOT")" "$(basename "$ROOT")"
    $SCP "$TARBALL" {{user}}@{{host}}:'C:\tesseras-win-src.tar.gz'
    $SSH "rmdir /s /q C:\tesseras 2>nul & tar xzf C:\tesseras-win-src.tar.gz -C C:\ && del C:\tesseras-win-src.tar.gz"
    rm -f "$TARBALL"

    echo "==> Running flutter pub get..."
    $SSH "set PATH=%PATH%;C:\flutter\bin;C:\Program Files\Git\cmd;%USERPROFILE%\.cargo\bin && cd C:\tesseras\apps\flutter && C:\flutter\bin\flutter pub get"

    echo "==> Building Windows release..."
    $SSH "set PATH=%PATH%;C:\flutter\bin;C:\Program Files\Git\cmd;%USERPROFILE%\.cargo\bin && cd C:\tesseras\apps\flutter && C:\flutter\bin\flutter build windows --release"

    echo "==> Building installer..."
    $SCP "$ROOT/packaging/windows/tesseras-installer.iss" {{user}}@{{host}}:'C:\tesseras-installer.iss'
    $SSH "\"C:\Program Files (x86)\Inno Setup 6\ISCC.exe\" C:\tesseras-installer.iss"
    $SSH "del C:\tesseras-installer.iss"

    echo "==> Downloading installer..."
    mkdir -p "$ROOT/target/packages"
    $SCP {{user}}@{{host}}:"C:\\tesseras-installer\\tesseras-setup-${VERSION}-win64.exe" "$ROOT/target/packages/"

    echo "Windows installer: target/packages/tesseras-setup-${VERSION}-win64.exe"
    ls -lh "$ROOT/target/packages/tesseras-setup-${VERSION}-win64.exe"

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

# Build .deb package for tesd + tes
deb:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release -p tesseras-daemon -p tesseras-cli
    target/release/tes completions bash > target/release/tes.bash
    target/release/tes completions zsh  > target/release/_tes
    target/release/tes completions fish > target/release/tes.fish
    cargo deb -p tesseras-daemon --no-build

# Build Arch Linux package (.pkg.tar.zst), then install with: sudo pacman -U packaging/archlinux/tesseras-*.pkg.tar.zst
arch:
    cd packaging/archlinux && TESSERAS_ROOT="$(git rev-parse --show-toplevel)" makepkg -sf

# Build Alpine Linux package (.apk)
alpine:
    cd packaging/alpine && TESSERAS_ROOT="$(git rev-parse --show-toplevel)" abuild -r

# Deploy .deb to a bootstrap node via scp + dpkg
deploy host="bootstrap1.tesseras.net":
    #!/usr/bin/env bash
    set -euo pipefail
    just deb
    DEB=$(ls -t target/debian/tesseras-daemon_*.deb | head -1)
    echo "Deploying $DEB to {{host}}..."
    scp "$DEB" root@{{host}}:/tmp/
    ssh root@{{host}} "dpkg -i /tmp/$(basename $DEB) && systemctl daemon-reload && systemctl restart tesd && rm /tmp/$(basename $DEB)"
    echo "Deployed and restarted tesd on {{host}}"

# Deploy .deb to all bootstrap nodes
deploy-all:
    #!/usr/bin/env bash
    set -euo pipefail
    just deb
    DEB=$(ls -t target/debian/tesseras-daemon_*.deb | head -1)
    HOSTS=("bootstrap1.tesseras.net" "bootstrap2.tesseras.net")
    for host in "${HOSTS[@]}"; do
        echo "Deploying $DEB to $host..."
        scp "$DEB" root@"$host":/tmp/
        ssh root@"$host" "dpkg -i /tmp/$(basename $DEB) && systemctl daemon-reload && systemctl restart tesd && rm /tmp/$(basename $DEB)"
        echo "Deployed and restarted tesd on $host"
    done
    echo "All bootstrap nodes updated"

# Build OpenBSD package (.tgz) via QEMU VM
openbsd:
    #!/usr/bin/env bash
    set -euo pipefail
    ROOT="$(git rev-parse --show-toplevel)"
    TARBALL="/tmp/tesseras-src.tar.gz"
    # Include tracked files + untracked packaging/openbsd files
    (cd "$ROOT" && { git ls-files; git ls-files --others --exclude-standard -- packaging/openbsd/; } | sort -u | tar czf "$TARBALL" --transform='s,^,tesseras/,' -T -)
    echo "Uploading source to obsd-build..."
    scp "$TARBALL" obsd-build:/tmp/
    ssh obsd-build 'rm -rf ~/tesseras && cd /tmp && tar xzf tesseras-src.tar.gz && mv tesseras ~/'
    echo "Building package inside OpenBSD VM..."
    ssh obsd-build 'cd ~/tesseras && sh packaging/openbsd/create-package.sh'
    mkdir -p "$ROOT/target/packages"
    scp 'obsd-build:~/tesseras/target/packages/tesseras-*.tgz' "$ROOT/target/packages/"
    echo "Package downloaded to target/packages/"
    ls -lh "$ROOT"/target/packages/tesseras-*.tgz

# Deploy OpenBSD package to a VPS (uses SSH config user + doas)
deploy-openbsd host:
    #!/usr/bin/env bash
    set -euo pipefail
    PKG=$(ls -t target/packages/tesseras-*.tgz | head -1)
    echo "Deploying $PKG to {{host}}..."
    scp "$PKG" {{host}}:/tmp/
    ssh {{host}} "doas rcctl stop tesd 2>/dev/null || true; doas pkg_add -D unsigned -aD snap /tmp/$(basename $PKG); doas rcctl enable tesd; doas rcctl start tesd; rm /tmp/$(basename $PKG)"
    echo "Verifying..."
    ssh {{host}} "doas rcctl check tesd"
    echo "Deployed and started tesd on {{host}}"

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
