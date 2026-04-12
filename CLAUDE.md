# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                  # build debug binary
cargo build --release        # build release binary
cargo test                   # run all tests
cargo test <test_name>       # run a single test, e.g. cargo test haskell_arrow_not_comment
cargo fmt                    # format code — always run before clippy
cargo clippy                 # lint — must pass with zero warnings before committing
cargo tarpaulin --out stdout # coverage report (currently ~92%)
cargo run --bin km -- loc    # run on current directory
cargo run --bin km -- loc src/  # run on specific path
```

The binary is named `km` (configured in `[[bin]]` in Cargo.toml). After `cargo install --path .` it installs as `km`.

## Documentation

- Architecture and module structure: [docs/architecture.md](docs/architecture.md)
- Contributing conventions and test patterns: [docs/contributing.md](docs/contributing.md)
- Per-command reference: [docs/commands/](docs/commands/)

## Conventions

- Always run `cargo fmt` before `cargo clippy`. Then validate with `cargo clippy` (zero warnings required) and `cargo test` before considering a change complete.
- The tool's output should match `cloc` as closely as possible — use `cloc` as the reference when validating changes.
- When adding or modifying a feature (new command, new flag, changed behavior), update the corresponding file in `docs/commands/` — **not** `README.md` directly.
- When adding a new command, also add a row to the commands table in `README.md` and a module entry in `docs/architecture.md`.
- Edition 2024 Rust (requires recent toolchain).
