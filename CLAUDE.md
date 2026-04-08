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

## Architecture

CLI tool for code metrics: lines of code (like `cloc`), duplicate detection, Halstead complexity, cyclomatic complexity, cognitive complexity, indentation analysis, Maintainability Index, code smells, hotspots, temporal coupling, knowledge maps, code churn, file age, and an overall health score. Built around a character-level finite state machine for line classification.

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
- When adding or modifying a feature (new command, new flag, changed behavior), update `README.md` to reflect the change before considering the work done.
- Tests in `counter.rs` use `count_reader(Cursor::new(...))` to test the FSM without touching the filesystem.
- Tests in `mod.rs` use `tempfile::tempdir()` for integration tests with real files.
- Tests exist in all modules: `counter.rs`, `language.rs`, `report.rs`, `mod.rs`.
- Edition 2024 Rust (requires recent toolchain).

### Output modes

All analysis commands support four output modes via mutually exclusive flags:
- `--json`: full JSON (default: table). Each module's `report.rs` has `print_json()`.
- `--short`: single compact line of `key:value` pairs. Each module's `report.rs` has `print_short()`.
- `--terse`: single headline metric value for piping. Each module's `report.rs` has `print_terse()`.

The `OutputMode` enum in `src/cli.rs` (`Table | Json | Short | Terse`) replaces the old `json: bool` parameter. Every module's `run()` function takes `output: OutputMode`. The `dispatch!` macro in `main.rs` derives it from `CommonArgs::output_mode()`. For modules using `report_helpers::output_results_mode()`, all four formatters are passed as function pointers.

### Module structure: `src/mi/`

Maintainability Index (Visual Studio variant, 0–100 scale). Invoked via `km mi`.

- **`analyzer.rs`** — `MILevel` enum (Green/Yellow/Red), `MIMetrics` struct, `compute_mi()` with VS formula: `MAX(0, raw * 100/171)`.
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: walks files, calls `hal::analyze_file` and `cycom::analyze_file` (pub(crate)) for volume and complexity, classifies lines for LOC.

### Module structure: `src/miv/`

Maintainability Index (verifysoft variant with comment weight). Invoked via `km miv`.

- **`analyzer.rs`** — `MILevel` enum (Good/Moderate/Difficult), `MIMetrics` struct, `compute_mi()` function implementing the verifysoft formula with radians conversion for comment percentage.
- **`report.rs`** — Table and JSON output formatters (`FileMIMetrics`, `print_report`, `print_json`).
- **`mod.rs`** — Orchestration: walks files, calls `hal::analyze_file` and `cycom::analyze_file` (pub(crate)) for Halstead volume and cyclomatic complexity, classifies lines for LOC/comment counts, computes MI. Note: each file is read three times (once per analyzer) due to per-module architecture.

### Module structure: `src/cogcom/`

Cognitive complexity (SonarSource method). Invoked via `km cogcom`.

- **`analyzer.rs`** — Core computation: penalizes nesting increments and non-linear control structures. Resets `opens_flow` after consuming the first `{` on a control-flow line so closure/struct braces on the same line are not double-counted.
- **`detection.rs`** — Language-aware function boundary detection.
- **`markers.rs`** — Per-language complexity markers (keywords that trigger nesting increments).
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: walks files, computes per-function and per-file scores, sorts/filters results.

### Module structure: `src/hotspots/`

Hotspot analysis (Thornhill, change frequency × complexity). Invoked via `km hotspots`.

- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, counts commits per file, computes complexity (indent or cyclomatic), multiplies for hotspot score. Uses `util::parse_since` for `--since` flag.

### Module structure: `src/knowledge/`

Knowledge maps (Thornhill, code ownership via git blame). Invoked via `km knowledge`.

- **`analyzer.rs`** — `RiskLevel` enum (Critical/High/Medium/Low), `FileOwnership` struct, `compute_ownership()` function that calculates primary owner, concentration, and knowledge loss risk.
- **`report.rs`** — Table and JSON output formatters. `--summary` mode aggregates by author (files owned, lines, languages, worst risk) as an alternative to the per-file view.
- **`mod.rs`** — Orchestration: opens git repo, walks files (filtering generated files), runs `blame_file()` per file, computes ownership, sorts/filters results. Uses `util::parse_since` for `--since` flag. `--summary` flag switches to author-level aggregation.

### Module structure: `src/tc/`

Temporal coupling analysis (Thornhill, files that change together). Invoked via `km tc`.

- **`analyzer.rs`** — `CouplingLevel` enum (Strong/Moderate/Weak), `FileCoupling` struct, `compute_coupling()` function that pairs co-changing files, calculates strength = shared_commits / min(commits_a, commits_b), and classifies by threshold (0.5 strong, 0.3 moderate).
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, calls `file_frequencies()` (filtered by `min_degree`), `co_changing_commits()`, `compute_coupling()`, sorts/filters results. Uses `util::parse_since` for `--since` flag. No filesystem walk needed — works entirely from git data.

### Module structure: `src/churn/`

Code churn — pure change frequency per source file. Invoked via `km churn`.

- **`analyzer.rs`** — `ChurnLevel` enum (High/Medium/Low), `FileChurn` struct, rate computed as commits ÷ active months (minimum 1 month).
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, counts commits per file, computes rate, sorts/filters. Uses `util::parse_since` for `--since` flag.

### Module structure: `src/smells/`

Code smell detection. Invoked via `km smells`.

- **`rules.rs`** — Individual smell detectors: long functions, long parameter lists, TODO debt, magic numbers (note: `'_'` is a valid digit separator, included in `is_numeric_char`), and commented-out code.
- **`analyzer.rs`** — Per-file smell aggregation.
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: walks files (or a specific `--files` list), supports `--since-ref <REF>` to limit analysis to files changed since a git ref (ideal for CI/PR checks).

### Module structure: `src/age/`

File age classification. Invoked via `km age`.

- **`analyzer.rs`** — Classifies files as Active/Stale/Frozen based on days since last git commit. Thresholds configurable via `--active-days` and `--frozen-days`.
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, resolves last-commit date per file, classifies, sorts/filters.

### Module structure: `src/authors/`

Per-author ownership summary. Invoked via `km authors`.

- **`analyzer.rs`** — Aggregates `git blame` data across all files to produce per-author totals: files owned, lines, languages, last active date.
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, runs blame per file, aggregates by author. Uses `util::parse_since` for `--since` flag.

### Module structure: `src/report/`

Comprehensive multi-metric report. Invoked via `km report`.

- **`data.rs`** / **`builder.rs`** — Data types and per-file metric collection (LOC, dups, indent, Halstead, cyclomatic, MI).
- **`markdown.rs`** — Human-readable table report.
- **`json.rs`** — JSON output formatter.
- **`mod.rs`** — Orchestration: walks files once, runs all analyzers, emits unified report.

### Module structure: `src/score/`

Overall code health score (A++ to F--). Invoked via `km score`. Static metrics only (no git required); `--trend` and `km score diff` require a git repo.

- **`analyzer.rs`** — `Grade` enum (16 grades: A++ to F--), `DimensionScore`/`FileScore`/`ProjectScore` structs, `score_to_grade()`, `compute_project_score()`, `Grade::numeric_rank()` (used for gate comparisons), `Grade::parse()` (for `--fail-below` CLI arg), and 6 normalization functions. Halstead normalization uses effort-per-LOC.
- **`report.rs`** — Table and JSON output formatters.
- **`diff.rs`** — `ScoreDiff`/`ScoreDelta` types and `compute_diff()`. Asserts dimension count and names match before zipping to prevent silent model mismatches.
- **`diff_report.rs`** — Table and JSON formatters for diff output.
- **`mod.rs`** — `ScoreGate` struct (`fail_if_worse`, `fail_below`). `run()` for normal score. `run_diff()` for `--trend`/`km score diff`: computes before/after snapshots, prints report, then evaluates quality gates (gates always evaluated after output so CI logs are complete).
- **`scoring.rs`** / **`collector.rs`** / **`normalizer.rs`** — Dimension definitions, per-file metric extraction, and piecewise linear normalization curves.
