# Jorm-RS

A fast, reliable DAG execution engine with optional AI assistance, implemented in Rust.

This repository contains the Rust implementation in `jorm-rs/` (library and CLI). The engine parses DAG definitions, schedules and executes tasks using native executors (shell, HTTP, file, Python bridge), and includes optional AI helpers for DAG analysis and generation.

## Features

- Pure Rust engine for performance and portability
- AI-assisted DAG analysis and generation (pluggable model backends)
- Multiple DAG formats supported (.txt, .md, .yaml)
- Native executors: shell, HTTP, file operations, Python bridge
- Cron-based scheduler and daemon for recurring jobs
- Interactive CLI and chat interface

## Quick examples

Run a DAG (example):

```bash
# from repository root
cd jorm-rs
cargo run -- run path/to/my_dag.yaml
```

Analyze a DAG for optimization:

```bash
cargo run -- analyze path/to/my_dag.yaml
```

Generate a DAG from natural language (AI helper):

```bash
cargo run -- generate "Create a data pipeline that downloads, processes, and uploads data"
```

Start interactive chat:

```bash
cargo run -- chat
```

Note: the binary name produced by `cargo build` is the crate name (check `Cargo.toml` for the exact binary). You can also install with `cargo install --path .` for system-wide use.

## Build & test (local)

Use the Rust toolchain (rustup recommended). Example commands (PowerShell):

```powershell
# Enter project folder
cd 'c:\Users\seban\OneDrive\Documentos\code\jorm\jorm-rs'

# Build (debug)
cargo build

# Build release
cargo build --release

# Run the test suite (unit + integration). Some integration tests may require external services.
cargo test

# Run only ignored/long tests (if present)
cargo test -- --ignored
```

## Formatting, linting & security checks

Run these as part of local checks or CI:

```powershell
# Format
rustup component add rustfmt
cargo fmt

# Lint (Clippy)
rustup component add clippy
cargo clippy --all-targets --all-features -- -D warnings

# Dependency audit (install once)
cargo install cargo-audit
cargo audit
```

Notes:

- `cargo-audit` requires network access to check advisory database.
- `cargo clippy` and `cargo fmt` are recommended pre-commit hooks.

## Configuration

Runtime configuration is stored under the project configuration directory `.jorm/` (e.g. `.jorm/config.toml`). Do not commit secrets into this file. Use environment variables or a secrets manager for production credentials.

## Tests and CI guidance

- The repository contains unit and integration tests under `tests/` — integration tests may require external resources (AI model endpoints, databases, S3). Mark such tests with `#[ignore]` or provide local mocks for CI.
- For CI (GitHub Actions) use a Linux runner for coverage tools (e.g., tarpaulin) and to run `cargo tarpaulin` or similar.
- Minimal CI steps to add:
  1. checkout
  2. cache cargo target and registry
  3. run `cargo fmt -- --check`
  4. run `cargo clippy -- -D warnings`
  5. run `cargo test --lib`
  6. run `cargo audit`

## Security & hardening notes

- Avoid storing API keys or DB passwords in `.jorm/config.toml` inside the repo. Use CI secrets or environment variables.
- Review executor implementations (`src/executor/executors/*.rs`) for command injection and enforce input sanitization and timeouts.

## Contributing

Please open issues or PRs. If you're adding features that touch executors or scheduling, include unit tests and update documentation in this README.

## Where to go next

- Run `cargo fmt` and `cargo clippy` locally and fix any warnings.
- Run `cargo test` to verify the test-suite on your machine.
- If you'd like, I can add a GitHub Actions workflow that runs format, clippy, tests and cargo-audit — tell me and I'll create it.
