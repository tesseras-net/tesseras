[private]
default:
    @just --list --unsorted

# Build release binaries
build:
    cargo build --release

# Run all tests
test:
    cargo test --workspace

# Run E2E tests (requires Docker)
test-e2e:
    bash tests/e2e/run.sh

# Run clippy lints
clippy:
    cargo clippy --workspace -- -D warnings

# Check formatting
fmt:
    cargo fmt --check

# Run all checks (clippy + fmt + test)
check: clippy fmt test

# Install the CLI binary
install:
    cargo install --path tes

# Build Arch Linux package (run on Arch)
arch:
    cd packaging/archlinux && TESSERAS_ROOT="$(pwd)/../.." makepkg -sf

# Build Alpine package (run on Alpine)
alpine:
    cd packaging/alpine && TESSERAS_ROOT="$(pwd)/../.." abuild -r

# Build OpenBSD package (run on OpenBSD)
openbsd:
    sh packaging/openbsd/create-package.sh

# Build OpenBSD package via QEMU (starts VM, builds, stops)
openbsd-qemu image="$HOME/vms/openbsd77/openbsd77.qcow2":
    OPENBSD_IMAGE={{image}} sh packaging/openbsd/qemu-build.sh --start-vm --stop-vm

# Build OpenBSD package via remote SSH host
openbsd-remote host="obsd-build" port="7722":
    OPENBSD_HOST={{host}} OPENBSD_PORT={{port}} sh packaging/openbsd/qemu-build.sh

# Build Windows binary and installer via SSH
windows host user="$USER":
    WIN_HOST={{host}} WIN_USER={{user}} sh packaging/windows/build.sh
