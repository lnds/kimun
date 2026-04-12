# Architecture

Kimün is a CLI tool for code metrics built around a character-level finite state machine for line classification. All commands share a common file-walking layer (`src/walk.rs`, via the `ignore` crate) that respects `.gitignore` rules and deduplicates files by content hash.

## Module map

| Module | Command | Description |
|--------|---------|-------------|
| `src/loc/` | `km loc` | Line counting FSM |
| `src/dups/` | `km dups` | Duplicate detection |
| `src/indent/` | `km indent` | Indentation complexity |
| `src/hal/` | `km hal` | Halstead metrics |
| `src/cycom/` | `km cycom` | Cyclomatic complexity |
| `src/cogcom/` | `km cogcom` | Cognitive complexity |
| `src/mi/` | `km mi` | Maintainability Index (VS) |
| `src/miv/` | `km miv` | Maintainability Index (verifysoft) |
| `src/hotspots/` | `km hotspots` | Change frequency × complexity |
| `src/knowledge/` | `km knowledge` | Code ownership via git blame |
| `src/tc/` | `km tc` | Temporal coupling |
| `src/churn/` | `km churn` | Pure change frequency |
| `src/smells/` | `km smells` | Code smell detection |
| `src/deps/` | `km deps` | Dependency graph + cycle detection |
| `src/authors/` | `km authors` | Per-author ownership summary |
| `src/age/` | `km age` | File age classification |
| `src/score/` | `km score` | Overall health grade |
| `src/report/` | `km report` | Multi-metric unified report |

## `src/loc/` — Line counter FSM

- **`language.rs`** — `LanguageSpec` struct + `lang!` macro defining 40+ languages. Detection by filename, extension, or shebang. Each spec declares: line comment markers, block comment delimiters, nesting support, string delimiter rules, pragma syntax, and exception characters for comment detection.

- **`counter.rs`** — FSM with states `Normal`, `InString(StringKind)`, `InBlockComment(depth)`. Processes files line-by-line via `BufReader`, classifying each line as blank/comment/code. Mixed lines (code + comment) count as code. Key design decisions:
  - Only `"` triggers string mode (not `'`) unless `single_quote_strings` is set — avoids Rust lifetime false positives
  - `InString` resets at line end for single/double quotes but persists for triple-quotes (Python)
  - Block comments track nesting depth when `nested_block_comments` is true
  - Pragmas (Haskell `{-# ... #-}`) are checked before block comments and counted as code
  - Shebang lines (`#!`) are always counted as code
  - `line_comment_not_before` field prevents `-->` from matching `--` in Haskell

- **`report.rs`** — Formats results as a table sorted by code lines descending, with totals.

- **`mod.rs`** — Orchestrates: walks directory tree, detects language, deduplicates files by content hash (streaming), counts lines, aggregates by language.

### Adding a new language

Use the `lang!` macro in `language.rs`. For most languages:

```rust
lang!("LangName", ext: ["ext1", "ext2"],
      line: "//", block: "/*", "*/", sq: true,
      shebangs: ["interpreter"]),
```

Optional flags: `nested: true`, `sq: true` (single-quote strings), `tq: true` (triple-quote strings), `pragma: "{-#", "#-}"`. Use `lines: ["marker1", "marker2"]` for multiple line comment markers. For languages needing `line_comment_not_before`, write the `LanguageSpec` struct directly (see Haskell).

## `src/mi/` and `src/miv/`

Both share the same orchestration pattern: walk files, call `hal::analyze_file` and `cycom::analyze_file` (`pub(crate)`) for Halstead volume and cyclomatic complexity, classify lines for LOC/comment counts, compute MI.

Note: each file is read three times (once per analyzer) due to per-module architecture.

- **`src/mi/analyzer.rs`** — `MILevel` enum (Green/Yellow/Red), VS formula: `MAX(0, raw * 100/171)`.
- **`src/miv/analyzer.rs`** — `MILevel` enum (Good/Moderate/Difficult), verifysoft formula with radians conversion for comment percentage.

## `src/cogcom/` — Cognitive complexity

- **`analyzer.rs`** — Core computation: penalizes nesting increments and non-linear control structures. Resets `opens_flow` after consuming the first `{` on a control-flow line so closure/struct braces on the same line are not double-counted.
- **`detection.rs`** — Language-aware function boundary detection.
- **`markers.rs`** — Per-language complexity markers (keywords that trigger nesting increments).

## `src/score/` — Health score

- **`analyzer.rs`** — `Grade` enum (16 grades: A++ to F--), `DimensionScore`/`FileScore`/`ProjectScore` structs, `score_to_grade()`, `compute_project_score()`, `Grade::numeric_rank()` (used for gate comparisons), `Grade::parse()` (for `--fail-below` CLI arg), and 6 normalization functions. Halstead normalization uses effort-per-LOC.
- **`diff.rs`** — `ScoreDiff`/`ScoreDelta` types and `compute_diff()`. Asserts dimension count and names match before zipping to prevent silent model mismatches.
- **`mod.rs`** — `ScoreGate` struct (`fail_if_worse`, `fail_below`). `run_diff()` for `--trend`/`km score diff`: computes before/after snapshots, prints report, then evaluates quality gates (gates always evaluated after output so CI logs are complete).
- **`scoring.rs`** / **`collector.rs`** / **`normalizer.rs`** — Dimension definitions, per-file metric extraction, and piecewise linear normalization curves.

## Git-based modules

`hotspots`, `knowledge`, `tc`, `churn`, `authors`, `age` all open the git repo via `git2`, count commits or run blame, and use `util::parse_since` for the `--since DURATION` flag. `tc` works entirely from git data — no filesystem walk needed.

## `src/smells/`

- **`rules.rs`** — Individual smell detectors: long functions, long parameter lists, TODO debt, magic numbers (note: `'_'` is a valid digit separator, included in `is_numeric_char`), and commented-out code.
- **`mod.rs`** — Supports `--since-ref <REF>` to limit analysis to files changed since a git ref (ideal for CI/PR checks).

## `src/report/`

- **`data.rs`** / **`builder.rs`** — Data types and per-file metric collection (LOC, dups, indent, Halstead, cyclomatic, MI).
- **`markdown.rs`** / **`json.rs`** — Output formatters.
- **`mod.rs`** — Walks files once, runs all analyzers, emits unified report.
