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
cargo run --bin cm -- loc    # run on current directory
cargo run --bin cm -- loc src/  # run on specific path
```

The binary is named `cm` (configured in `[[bin]]` in Cargo.toml). After `cargo install --path .` it installs as `cm`.

## Architecture

CLI tool for code metrics: lines of code (like `cloc`), duplicate detection, Halstead complexity, cyclomatic complexity, indentation analysis, and Maintainability Index. Built around a character-level finite state machine for line classification.

### Module structure: `src/loc/`

- **`language.rs`** — `LanguageSpec` struct + `lang!` macro defining 40+ languages. Detection by filename, extension, or shebang. Each spec declares: line comment markers, block comment delimiters, nesting support, string delimiter rules, pragma syntax, and exception characters for comment detection.

- **`counter.rs`** — FSM with states `Normal`, `InString(StringKind)`, `InBlockComment(depth)`. Processes files line-by-line via `BufReader`, classifying each line as blank/comment/code. Mixed lines (code + comment) count as code. Key design decisions:
  - Only `"` triggers string mode (not `'`) unless `single_quote_strings` is set — avoids Rust lifetime false positives
  - `InString` resets at line end for single/double quotes but persists for triple-quotes (Python)
  - Block comments track nesting depth when `nested_block_comments` is true
  - Pragmas (Haskell `{-# ... #-}`) are checked before block comments and counted as code
  - Shebang lines (`#!`) are always counted as code
  - `line_comment_not_before` field prevents `-->` from matching `--` in Haskell

- **`report.rs`** — Formats results as a table sorted by code lines descending, with totals.

- **`mod.rs`** — Orchestrates: walks directory tree (via `ignore` crate, respects `.gitignore`), detects language, deduplicates files by content hash (streaming), counts lines, aggregates by language.

### Adding a new language

Use the `lang!` macro in `language.rs`. For most languages:
```rust
lang!("LangName", ext: ["ext1", "ext2"],
      line: "//", block: "/*", "*/", sq: true,
      shebangs: ["interpreter"]),
```

Optional flags: `nested: true`, `sq: true` (single-quote strings), `tq: true` (triple-quote strings), `pragma: "{-#", "#-}"`. Use `lines: ["marker1", "marker2"]` for multiple line comment markers. For languages needing `line_comment_not_before`, write the `LanguageSpec` struct directly (see Haskell).

## Conventions

- The tool's output should match `cloc` as closely as possible — use `cloc` as the reference when validating changes.
- Always run `cargo fmt` before `cargo clippy`. Then validate with `cargo clippy` (zero warnings required) and `cargo test` before considering a change complete.
- Tests in `counter.rs` use `count_reader(Cursor::new(...))` to test the FSM without touching the filesystem.
- Tests in `mod.rs` use `tempfile::tempdir()` for integration tests with real files.
- Tests exist in all modules: `counter.rs`, `language.rs`, `report.rs`, `mod.rs`.
- Edition 2024 Rust (requires recent toolchain).

### Module structure: `src/mi/`

Maintainability Index (Visual Studio variant, 0–100 scale). Invoked via `cm mi`.

- **`analyzer.rs`** — `MILevel` enum (Green/Yellow/Red), `MIMetrics` struct, `compute_mi()` with VS formula: `MAX(0, raw * 100/171)`.
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: walks files, calls `hal::analyze_file` and `cycom::analyze_file` (pub(crate)) for volume and complexity, classifies lines for LOC.

### Module structure: `src/miv/`

Maintainability Index (verifysoft variant with comment weight). Invoked via `cm miv`.

- **`analyzer.rs`** — `MILevel` enum (Good/Moderate/Difficult), `MIMetrics` struct, `compute_mi()` function implementing the verifysoft formula with radians conversion for comment percentage.
- **`report.rs`** — Table and JSON output formatters (`FileMIMetrics`, `print_report`, `print_json`).
- **`mod.rs`** — Orchestration: walks files, calls `hal::analyze_file` and `cycom::analyze_file` (pub(crate)) for Halstead volume and cyclomatic complexity, classifies lines for LOC/comment counts, computes MI. Note: each file is read three times (once per analyzer) due to per-module architecture.

### Module structure: `src/knowledge/`

Knowledge maps (Thornhill, code ownership via git blame). Invoked via `cm knowledge`.

- **`analyzer.rs`** — `RiskLevel` enum (Critical/High/Medium/Low), `FileOwnership` struct, `compute_ownership()` function that calculates primary owner, concentration, and knowledge loss risk.
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, walks files (filtering generated files), runs `blame_file()` per file, computes ownership, sorts/filters results. Uses `util::parse_since` for `--since` flag.

### Module structure: `src/tc/`

Temporal coupling analysis (Thornhill, files that change together). Invoked via `cm tc`.

- **`analyzer.rs`** — `CouplingLevel` enum (Strong/Moderate/Weak), `FileCoupling` struct, `compute_coupling()` function that pairs co-changing files, calculates strength = shared_commits / min(commits_a, commits_b), and classifies by threshold (0.5 strong, 0.3 moderate).
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, calls `file_frequencies()` (filtered by `min_degree`), `co_changing_commits()`, `compute_coupling()`, sorts/filters results. Uses `util::parse_since` for `--since` flag. No filesystem walk needed — works entirely from git data.
