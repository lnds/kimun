# Contributing

## Build & test commands

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

## Conventions

- The tool's output should match `cloc` as closely as possible — use `cloc` as the reference when validating changes.
- Always run `cargo fmt` before `cargo clippy`. Then validate with `cargo clippy` (zero warnings required) and `cargo test` before considering a change complete.
- When adding or modifying a feature (new command, new flag, changed behavior), update the relevant file in `docs/commands/` before considering the work done.
- Tests in `counter.rs` use `count_reader(Cursor::new(...))` to test the FSM without touching the filesystem.
- Tests in `mod.rs` use `tempfile::tempdir()` for integration tests with real files.
- Tests exist in all modules: `counter.rs`, `language.rs`, `report.rs`, `mod.rs`.
- Edition 2024 Rust (requires recent toolchain).

## Documentation

Command documentation lives in `docs/commands/<command>.md`. The `README.md` is a top-level index — it links to these files rather than containing the full content.

When adding a new command:
1. Add `docs/commands/<name>.md` with usage, options, and example output.
2. Add a row to the commands table in `README.md`.
3. Add the module description to `docs/architecture.md`.
