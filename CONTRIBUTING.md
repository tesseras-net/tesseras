# Contributing to Tesseras

Thank you for your interest in contributing to tesseras! This document explains
how to get started.

## Getting Started

### Prerequisites

- **Rust 1.85+** (edition 2024)
- **just** — task runner (replaces Make)
- **cargo-deny** — dependency policy checker
- **dprint** — formatter for TOML/JSON/Markdown

### Building

```sh
# List all available tasks
just

# Build the entire workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Install CLI and daemon locally
just install
```

## Development Workflow

1. **Fork** the repository and create a feature branch from `main`.
2. **Write code** following the conventions below.
3. **Add tests** — unit tests in the same file (`#[cfg(test)] mod tests`),
   integration tests in `tests/`.
4. **Run checks** before submitting:
   ```sh
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test --workspace
   cargo deny check
   ```
5. **Submit your contribution** using one of the methods below.

### Submitting via Email (preferred)

The primary way to contribute is by sending a patch to the SourceHut mailing
list using `git send-email`:

```sh
# One-time setup: configure git send-email
git config sendemail.to "~ijanc/tesseras-devel@lists.sr.ht"
git config sendemail.annotate true

# Send your commits as a patch series
git send-email --to="~ijanc/tesseras-devel@lists.sr.ht" origin/main
```

If you are new to `git send-email`, see the
[git-send-email tutorial](https://git-send-email.io/) for setup instructions
on your platform.

Tips for email patches:

- Write a clear cover letter (`--cover-letter`) for multi-commit series.
- Ensure your patches apply cleanly on top of `main`.
- Respond to review feedback by sending a revised series with
  `git send-email -v2` (or `-v3`, etc.).

### Submitting via GitHub

You can also open a pull request on the
[GitHub mirror](https://github.com/ijanc/tesseras). Fork the repository,
push your branch, and open a PR against `main`.

## Coding Conventions

- **Language**: all code, comments, and docs in English.
- **Architecture**: hexagonal (Ports, Adapters, Application, Domain) where it
  makes sense.
- **Errors**: `thiserror` for `tesseras` lib, `anyhow` for `tes` binary.
- **Async**: `tokio` runtime for I/O in service layer; storage traits are sync.
- **Trait objects**: prefer `Box<dyn Trait + Send + Sync>` over generics.
- **Naming**: Rust conventions — `snake_case` for functions, `PascalCase` for
  types.
- **Formatting**: `cargo fmt` for Rust, `dprint` for TOML/JSON/Markdown.
- **Linting**: `cargo clippy -- -D warnings` — zero warnings policy.
- **Dependencies**: additions must pass `cargo deny check`.
- **License**: ISC — all contributions are licensed under the same terms.

## Workspace Structure

The workspace uses a simplified 2-crate layout:

| Crate | Purpose |
|-------|---------|
| `tesseras` | Library — all domain logic (types, crypto, storage, DHT, net, replication, RPC) |
| `tes` | Single binary — CLI + daemon management via `tes admin daemon` |
| `tesseras-embedded` | Mobile/desktop FFI (flutter_rust_bridge) |

When contributing, keep domain logic in the `tesseras` library. The `tes`
binary is a thin CLI layer — subcommands live in `tes/src/commands/`.

## Commit Messages

Write concise commit messages that explain **why**, not just what. Use the
imperative mood:

```
fix(dht): prevent routing table overflow on bootstrap

The routing table accepted duplicate entries during concurrent bootstrap
requests. Add a uniqueness check before inserting new peers.
```

Prefix with the affected module when applicable: `fix(crypto):`, `feat(net):`,
`docs(book):`, `test(storage):`, `chore:`, `feat(cli):`.

## Reporting Issues

File issues on the [SourceHut ticket tracker](https://todo.sr.ht/~ijanc/tesseras).

- Search existing issues before opening a new one.
- Include Rust version (`rustc --version`), OS, and steps to reproduce.
- For security issues, follow the process described in
  [SECURITY.md](SECURITY.md) — do not open a public issue.

## Code of Conduct

This project follows a [Code of Conduct](CODE_OF_CONDUCT.md). By participating
you agree to abide by its terms.
