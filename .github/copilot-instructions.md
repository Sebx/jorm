Short guidance to make AI coding agents productive in this repository (jorm/jorm-rs).

Purpose
- Help an AI agent understand the big-picture architecture, developer workflows, and project-specific patterns so changes are safe and consistent.

Quick facts
- Primary Rust crate: `jorm-rs/` (library + CLI).
- Main responsibilities: parse DAGs (`src/parser/`), schedule (`src/scheduler/`), execute tasks via pluggable executors (`src/executor/`), and optional AI helpers (`src/ai/`).
- Config lives under `.jorm/config.toml` (do not expose secrets in commits).

When making changes
- Keep a single top-level change per PR (parser, scheduler, executor, ai). Cross-cutting changes must update tests under `tests/`.
- Run `cargo fmt` and `cargo clippy` locally; CI expects no clippy warnings.

Architecture notes (essential files)
- Entrypoints
  - `src/main.rs` — CLI; wire commands to library functions.
  - `src/lib.rs` — public library surface and orchestration.
- Parser
  - `src/parser/dag.rs`, `src/parser/parsers.rs` — canonical DAG representation and parsing rules. Small changes here affect downstream scheduling and executor inputs.
- Scheduler
  - `src/scheduler/cron_scheduler.rs`, `src/scheduler/daemon.rs` — scheduling/daemon lifecycle and concurrency. Prefer graceful shutdown and idempotent scheduling.
- Executors
  - `src/executor/traits.rs` — central trait definitions; read this before modifying or adding executors.
  - `src/executor/executors/*.rs` — concrete implementations (shell, http, file, python). Follow existing patterns for timeouts, retries (`retry.rs`) and error classification (`error.rs`).
- AI helpers
  - `src/ai/model_manager.rs`, `src/ai/generation.rs` — model selection and prompt orchestration. Treat external calls as network-bound and mark tests that rely on them as `#[ignore]`.

Project-specific conventions
- Placeholders: configuration values use `.jorm/config.toml` not hardcoded constants.
- Panics vs Result: library functions prefer `Result<T, E>`; reserve `panic!` for unrecoverable boot errors.
- Executors return structured `RunResult` in `executor/result.rs`; tests assert on `RunResult` status and logs.

Testing & CI
- Run full tests: `cargo test` (integration tests may require env or be ignored).
- Use `cargo clippy --all-targets --all-features -- -D warnings` locally.
- Security: run `cargo audit` in CI to catch vulnerable crates.

Integration points to be careful about
- File and process execution: `executor/executors/shell.rs` and `python.rs` — sanitize inputs, add timeouts and resource limits.
- External AI models and HTTP: `ai/*` and `executor/executors/http.rs` — network errors and rate limits must be handled gracefully.
- Persistence / state: scheduler/daemon may persist state; avoid changing formats of persisted data without migration logic.

Examples (copy/paste approach)
- Add a new executor: mimic `src/executor/executors/file.rs` structure, implement the trait in `traits.rs`, and add unit tests in `tests/unit/executor_tests.rs`.
- Add CLI command: wire in `src/main.rs`, add a public function in `lib.rs`, and test via `cargo test` (integration tests under `tests/integration/`).

Misc
- Never commit secrets from `.jorm/`. If you find them, redact and instruct maintainers to rotate.
- If changing public APIs in `lib.rs`, bump version in `Cargo.toml` and update README notes.

If anything is unclear or you'd like a shorter focused guide (e.g., "How to add an executor"), ask and I will produce a compact recipe.
