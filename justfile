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
