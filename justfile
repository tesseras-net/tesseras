[private]
default:
    @just --list --unsorted

mod website
mod book

# Install the CLI and daemon locally
install:
    cargo install --path crates/tesseras-cli
    cargo install --path crates/tesseras-daemon
