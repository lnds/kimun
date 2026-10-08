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

- **`counter.rs`** — FSM with states `Normal`, `InString(StringKind)`, `InBlockComment(depth)`, `InDocAttribute(Option<StringKind>)`. Processes files line-by-line via `BufReader`, classifying each line as blank/comment/code. Mixed lines (code + comment) count as code. Key design decisions:
  - Only `"` triggers string mode (not `'`) unless `single_quote_strings` is set — avoids Rust lifetime false positives
  - `InString` resets at line end for single/double quotes but persists for triple-quotes (Python)
  - Block comments track nesting depth when `nested_block_comments` is true
  - Pragmas (Haskell `{-# ... #-}`) are checked before block comments and counted as code
  - Shebang lines (`#!`) are always counted as code
  - `line_comment_not_before` field prevents `-->` from matching `--` in Haskell, and `#[` from matching `#` in Kaikai
  - `doc_attribute` delimiters (Kaikai `#[doc(` … `)]`) classify the whole attribute as comment; the state tracks the enclosed string literal so a `)]` inside the doc text does not close it early

- **`report.rs`** — Formats results as a table sorted by code lines descending, with totals.

- **`mod.rs`** — Orchestrates: walks directory tree (via `ignore` crate, respects `.gitignore`), detects language, deduplicates files by content hash (streaming), counts lines, aggregates by language.

### Adding a new language

Use the `lang!` macro in `language.rs`. For most languages:
```rust
lang!("LangName", ext: ["ext1", "ext2"],
      line: "//", block: "/*", "*/", sq: true,
      shebangs: ["interpreter"]),
```

Optional flags: `nested: true`, `sq: true` (single-quote strings), `tq: true` (triple-quote strings), `pragma: "{-#", "#-}"`. Use `lines: ["marker1", "marker2"]` for multiple line comment markers. For languages needing `line_comment_not_before` or `doc_attribute`, write the `LanguageSpec` struct directly (see Haskell and Kaikai).

## Conventions

- The tool's output should match `cloc` as closely as possible — use `cloc` as the reference when validating changes.
- Always run `cargo fmt` before `cargo clippy`. Then validate with `cargo clippy` (zero warnings required) and `cargo test` before considering a change complete.
- When adding or modifying a feature (new command, new flag, changed behavior), update `README.md` to reflect the change before considering the work done.
- `LEEME.md` is the Spanish edition of `README.md`, section by section. A change to one goes into the other.
- Tests in `counter.rs` use `count_reader(Cursor::new(...))` to test the FSM without touching the filesystem.
- Tests in `mod.rs` use `tempfile::tempdir()` for integration tests with real files.
- Tests exist in all modules: `counter.rs`, `language.rs`, `report.rs`, `mod.rs`.
- Edition 2024 Rust (requires recent toolchain).

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

### Shared: `src/git/`

`GitRepo` wraps `git2::Repository`. `mod.rs` holds history walks, blame and tree extraction. `changeset.rs` holds what concerns a change set: `diff_stats_since` (ref → working tree), `diff_stats_between` (two refs), `patch_stats` (a patch in git format), all through one `stats_of`, plus `co_change_history` and the merge base they start from. `FileDiffStat` carries the lines touched on both sides and a probe of added lines, so `lines_in` picks the side that matches a text: a patch need not be applied where it is measured. `tree.rs` reads files from the tree of a commit without checking it out (`files_at`, `has_file_at`).

### Shared: `src/projects/`

Projects of a repository and the dependencies between them, read from manifests. Independent of the ecosystem: each one contributes a reader that turns a manifest's text into a `Manifest` (name, raw dependencies, workspace table); discovery, resolution and reach are shared.

- **`mod.rs`** — The model: `Project`, `Edge`, `Scope`, `Manifest`, `DepTarget`, and `reader`, which maps a manifest file name to its reader.
- **`discover.rs`** — `ProjectGraph::from_manifests` builds the graph from `(path, text)` pairs, so the manifests can come from disk (`discover`) or from the tree of a commit. `discover` walks the repository (skipping `node_modules` and `vendor` anywhere, and `deps`, `_build`, `target` unless under `src`, `lib` or `app`, where they are modules of the project), builds one `Project` per directory and resolves each `DepTarget` (`Path`, `Name` within the same ecosystem, `Workspace`). Workspace roots (`[workspace]`, `workspaces`, `pnpm-workspace.yaml`, `apps_path`) are registered before projects, because an npm manifest at one is not a project. Skips `testdata` and `fixtures` inside test directories.
- **`radius.rs`** — `owner` gives the nearest project above a file; `workspace_of` tells when a file is the manifest or lock file of a workspace root, which governs every project under it. `radius` is a reverse BFS: non-dev edges first, then dev edges as terminal reaches, so a shorter dev path never hides a shipping one.
- **`cargo.rs`** / **`npm.rs`** / **`mix.rs`** / **`gomod.rs`** — Readers. `mix.rs` is lexical: strips comments, then picks out `{:name, ...}` tuples; a `path:` it cannot attribute is counted in `unread`.

To add an ecosystem: write a reader `fn read(&str) -> Manifest`, add its file name to `reader` and a variant to `Ecosystem`.

### Module structure: `src/ai/`

`km ai`: tools and skill for LLMs. `executor.rs` maps a tool name to `km <subcommand> --format json`; `schema.rs` holds the tool schemas; `skill.rs` the installable skill text; `permissions.rs` the Claude Code permissions. A new command for LLMs is added in all four.

### Module structure: `src/impact/`

Impact of a change: projects reached, diffusion and logical radius. Invoked via `km impact` with `--since-ref <REF>` (optionally `--until-ref`), `--diff <FILE>` or `--pr <NUMBER>` (needs `gh`).

- **`analyzer.rs`** — Pure functions. `compute_diffusion()` counts files, directories, subsystems (top-level directories), lines, and the normalized Shannon entropy of the modified lines. `missing_co_changes()` finds the files that usually change with the changed ones; confidence is directional (`shared / commits of the changed file`), unlike the symmetric strength of `tc`. Several changed files predicting the same file are merged as `Trigger`s, strongest first.
- **`help.rs`** — The long help of the command, apart from `cli_help.rs`.
- **`source.rs`** — `DiffSource` (what was asked: refs, a patch, a pull request) and `Change` (the source resolved against the repository). A `Change` answers the four things every source must: the files changed, where the preceding history ends, where manifests are read from (`graph`), and where a file is looked up (`has_file`). Between two refs the last two read the tree of the second ref, not the working tree.
- **`pr.rs`** — Pull requests through the GitHub CLI, run as a program. `plan` is pure: given which commits are local it picks refs or the patch. With base and head local it compares them. A squash or rebase merge whose head was never fetched is measured from the patch GitHub serves against the tree of the merge commit (`Plan::Merged`): the base GitHub reports may be many merges behind, so base..merge would count other pull requests. Tests never call the real `gh`: the program name is a parameter, and a script stands in for it.
- **`inert.rs`** — Changed files that reach nothing: documentation, plus the globs of `[impact] inert` in `.kimun.toml` (read from the repository root). They neither change their project nor make the reach unknown.
- **`entry.rs`** — Files that are run rather than used: the conventional task, script, example and bench directories and what a Rust package runs (`src/main.rs`, `build.rs`), when no source file uses them, plus the globs of `[impact] entry_points`. A dependent that is one and has no test is listed apart, not warned about as unprotected.
- **`routes.rs`** — Tests that reach a file through the route it serves: each test asks the routers nearest to it, and counts as a test that refers to the module behind the route. A signal of protection only: the router is not made a dependent of what it routes.
- **`reading.rs`** — The sources of a change read into a graph: templates go to the file that renders them and Cargo manifests name the packages; neither is a file of the graph.
- **`role.rs`** — What each file is to the radius: `Source`, `Test` or `Other` (test support, scripts). A file that holds tests is a test when it sits among tests or is a `tests.rs` module, and has a test of its own when it is a source.
- **`protection.rs`** — How tests protect a source file, in degrees: `Direct` (a test refers to it or sits at the same `place`), `Named` (a test carries its name, or that of its directory, a directory apart, and mirrors no source file itself), `Users` (a file that uses it has a test), `None`. Only `None` is reported as unprotected.
- **`structural.rs`** — Blast radius at source level. Each changed file is narrowed on its own (`Narrowing`); the radius always goes through the exposed dependents. Reads `deps::graph::FileGraph` backwards from the changed sources. `Role` separates sources, tests and the rest (test support, scripts). `Exposure` says whether a dependent calls a changed function, refers to the module, or calls elsewhere. Tests protect what they refer to and what sits at the same `place` in the layout. Only languages in `RELIABLE` are measured.
- **`structural_report.rs`** / **`structural_json.rs`** — The structural block as text and as JSON.
- **`projects.rs`** — Blast radius at project level: maps the diff to projects of `crate::projects::ProjectGraph`, computes the reach, and renders its block of the report and of the JSON. A changed workspace root reaches its members at distance 1 with scope `workspace`. `affected()` is the list `--affected` prints: every project when a changed file belongs to none. `--affected` skips the history walk (`print_affected` in `mod.rs`).
- **`report.rs`** — `render_report` builds the table as a `String` and `print_report` prints it, so tests assert on the text. JSON, short and terse formatters.
- **`mod.rs`** — Orchestration: `GitRepo::diff_stats_since` for the diff (merge base with the ref → working tree, deletions included), `GitRepo::co_change_history` for the history, which ends at the merge base so the commits of the diff are never evidence, and skips commits touching more than `--max-changeset` files (sweeping changes relate files by accident). Drops generated files (`util::is_generated`), files already in the diff and files that no longer exist. New files and files outside `--since` go to `without_history`. Test files are always included.

### Module structure: `src/churn/`

Code churn — pure change frequency per source file. Invoked via `km churn`.

- **`analyzer.rs`** — `ChurnLevel` enum (High/Medium/Low), `FileChurn` struct, rate computed as commits ÷ active months (minimum 1 month).
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, counts commits per file, computes rate, sorts/filters. Uses `util::parse_since` for `--since` flag.

### Module structure: `src/deps/`

Dependency graph (file-level coupling from imports, cycles via Tarjan SCC). Invoked via `km deps`.

- **`elixir.rs`** — Elixir, where dependencies are between modules. `parse` sets comments and literals aside, names nested modules by indentation, undoes `alias`, and returns every module referred to with the function called on it. `changed_functions` maps touched lines to the public functions affected, carrying a change to a private function to the public ones that call it. Outside functions, `alias`/`require` are neutral and a module attribute changes the functions that read it; anything else is `Err(line)`.
- **`elixir_literals.rs`** — `code_only` blanks comments, strings, sigils and character literals, keeping lines and columns. The code a string interpolates (`#{...}`) stays: it runs like any other.
- **`heex.rs`** — HEEx templates. `code` keeps only the Elixir of a template (braces, EEx tags, components called as tags), so prose never reads as a module; `owners` tells the module that renders a template kept in a file of its own. `embedded` picks the `~H` ones out of a source. `elixir::parse_with` reads both kinds with the aliases of the file. `km impact` hands the templates over; `km deps` reads only the `~H` ones.
- **`phoenix.rs`** — What Phoenix relates by name alone: a controller uses the views named after it (`PageController` and `PageJSON`, `PageHTML`, `PageView`).
- **`routes.rs`** — Phoenix routes, lexically. `routes` reads a router: each `scope` adds its path and alias to what is inside, by indentation. `requests` reads the paths a test asks for, only from calls that make a request. `serves` matches the two segment by segment.
- **`rust_literals.rs`** — Blanks comments and character literals of Rust, keeping lines; a string keeps its quotes and its content is handed apart, for `#[path = "…"]`.
- **`rust.rs`** — Rust, read lexically: the `mod` declarations of a file (with `#[path]` and the inline modules around them), every path of `use` trees and of code, the names `use` brings in, the types `impl` blocks are for, and whether the file holds `#[test]`s.
- **`rust_crates.rs`** — The module tree of each crate, grown from its root files by `mod` declarations, and `uses`: each path made absolute from the module it is written in and led to the file of the deepest module it names. Library crates answer to their package name. `mod x;` is no edge; a type uses the files of its `impl` blocks.
- **`graph.rs`** — `FileGraph::build` builds the file graph from `(path, language, text)` sources, so they can come from disk or from a commit's tree. `Layout` carries what is known besides the sources: paths to look up, the Go module, the Cargo packages. Elixir references resolve through a module index (nearest file, then `Related`: what the projects declare); other languages go through `extract_imports` + `resolve_import`. `Use.calls` keeps the functions called.
- **`extractor.rs`** — `extract_imports` dispatches by language name and returns raw import strings: relative imports for Python and JS/TS, every quoted import path for Go. Elixir and Rust are read apart, over the whole graph. `is_supported` decides which languages enter the graph.
- **`kaikai.rs`** — Kaikai extraction and resolution: `import a.b.c` names a file under a package root, found by trying each ancestor directory of the importer.
- **`analyzer.rs`** — `resolve_import` maps a raw import to project files per language; `build_graph` computes fan-in, fan-out and cycles (`tarjan_scc`, iterative).
- **`report.rs`** — `render_report` builds the table as a `String` and `print_report` prints it, so tests assert on the text. JSON, short and terse formatters.
- **`mod.rs`** — Orchestration: walks files, leaves unsupported languages out of the graph and counts them, extracts and resolves imports. `sort_entries` and `visible_entries` hold the `--sort-by` and `--cycles-only`/`--top` logic apart from `run`, which only prints.

### Shared: `src/string_mask.rs`

`multi_line_string_mask` marks the interior lines of triple-quoted strings;
`demote_multi_line_strings` turns those lines into `LineKind::Blank` so the
analyzers that select `LineKind::Code` skip them. Used by `hal`, `cycom`,
`cogcom`, and `smells` — prose or data inside a multi-line literal must not
count as control flow. `loc` is deliberately left alone: those lines are still
code for line counting.

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
- **`changed.rs`** — `--gate-scope changed`: pairs `GitRepo::changes_since_in_workdir` with per-file scores from both snapshots (keyed by path relative to the walked directory, displayed repo-relative), and fails when a file ending below the ref's project score dropped more than the tolerance (default 0.5, `GateScope::default_tolerance`), or when absolute duplicated lines grow. Files above that score, new files and deleted files never fail.
- **`mod.rs`** — `ScoreGate` struct (`max_drop` from `--fail-if-worse` + `--gate-tolerance`, compared on unrounded scores; `scope` from `--gate-scope`; `fail_below`). `compute_snapshot()` keeps all per-file scores and duplicated lines for the changed-scope gate; `compute_score()` wraps it. `run()` for normal score. `run_diff()` for `--trend`/`km score diff`: computes before/after snapshots, prints report, then evaluates quality gates (gates always evaluated after output so CI logs are complete).
- **`scoring.rs`** / **`collector.rs`** / **`normalizer.rs`** — Dimension definitions, per-file metric extraction, and piecewise linear normalization curves.
