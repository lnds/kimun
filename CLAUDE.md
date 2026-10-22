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

- **`counter.rs`** — FSM with states `Normal`, `InString(StringKind)`, `InBlockComment(block, depth)`, `InDocAttribute(Option<StringKind>)`, `InDocString(StringKind)`. Processes files line-by-line via `BufReader`, classifying each line as blank/comment/code. Bytes that are not UTF-8 are replaced, so a Latin-1 or Windows-1252 source is read whole. Mixed lines (code + comment) count as code. Key design decisions:
  - Only `"` triggers string mode (not `'`) unless `single_quote_strings` is set — avoids Rust lifetime false positives
  - `InString` resets at line end for single/double quotes but persists for triple-quotes (Python) and raw strings
  - `raw_strings` (Rust): `r"…"`, `r#"…"#`, with a `b` or `c` prefix too, end at a quote followed by as many `#` as opened them (`raw.rs`). `raw_string_spans` tells where they are on each line
  - Block comments track nesting depth when `nested_block_comments` is true. A language may have a second kind (`other_block_comment`, Pascal's `(* *)` besides `{ }`); each is closed only by its own delimiter
  - `column_one_comments` (Fortran's fixed form: `C`, `c`, `*`) make a comment of the whole line; `directives` (`$omp`, `$acc`, `$ `) right after a comment marker make it code
  - `backslash_escapes` is false in Fortran and Pascal, where `'\'` is a one-character string
  - Pragmas (Haskell `{-# ... #-}`) are checked before block comments and counted as code
  - Shebang lines (`#!`) are always counted as code
  - `line_comment_not_before` field prevents `-->` from matching `--` in Haskell, and `#[` from matching `#` in Kaikai
  - `doc_attribute` delimiters (Kaikai `#[doc(` … `)]`) classify the whole attribute as comment; the state tracks the enclosed string literal so a `)]` inside the doc text does not close it early
  - `triple_quote_comments` (Python, `tqc: true`): a triple-quoted string is `InDocString`, comment unless code shares its line, as cloc counts docstrings
  - `detect_file` refines `detect` by content: a `.h` holding C++ (`namespace`, `template`, `class`, `std::`, ...) is C++

- **`report.rs`** — Formats results as a table sorted by code lines descending, with totals.

- **`mod.rs`** — Orchestrates: walks directory tree (via `ignore` crate, respects `.gitignore`), detects language, deduplicates files by content hash (streaming), counts lines, aggregates by language.

### Shared: `src/walk.rs`

`source_files` is every recognized file (language by `detect_file`) of the kinds `--kind` reads; with tests excluded it also leaves out `EXAMPLE_DIRS` (examples, benchmarks) and the Rust modules declared under `#[cfg(test)]` (`test_blocks::rust_test_modules`). `code_files` is what metrics of code read: the kinds `kind::Selection::measures` keeps (code, and infrastructure in a language of code unless `[kinds] separate`) and no generated files (`util::is_generated_file`, by name, header marker or source map). `collect_analysis`, `score`, `report`, `dups`, `init`, `hotspots` and `churn` use it; `loc` and `deps` read `source_files`; `tc` filters history paths with `is_code_path`.

### Adding a new language

Use the `lang!` macro in `language.rs`. For most languages:
```rust
lang!("LangName", ext: ["ext1", "ext2"],
      line: "//", block: "/*", "*/", sq: true,
      shebangs: ["interpreter"]),
```

Optional flags: `nested: true`, `sq: true` (single-quote strings), `tq: true` (triple-quote strings), `tqc: true` (triple-quoted strings count as comment, line-comment variant), `pragma: "{-#", "#-}"`. Use `lines: ["marker1", "marker2"]` for multiple line comment markers. For languages needing `line_comment_not_before`, `doc_attribute`, `raw_strings`, `other_block_comment`, `column_one_comments`, `directives`, `case_insensitive` or no `backslash_escapes`, write the `LanguageSpec` struct directly (see Rust, Haskell, Kaikai, Fortran and Pascal).

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

- **`analyzer.rs`** — Core computation: penalizes nesting increments and non-linear control structures. Brace-scoped code is read token by token along each line (`line_tokens`: keywords and braces in order), through `BraceScan`: a control keyword leaves its block pending until the next `{`, on its line or the next (Allman), and only that first brace nests, so closure/struct braces on the same line are not double-counted.
- **`detection.rs`** — Language-aware function boundary detection.
- **`markers.rs`** — Per-language complexity markers (keywords that trigger nesting increments).
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: walks files, computes per-function and per-file scores, sorts/filters results.

### Module structure: `src/hotspots/`

Hotspot analysis (Tornhill, change frequency × complexity). Invoked via `km hotspots`.

- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, counts commits per file, computes complexity (indent, cyclomatic or cognitive; lines of text for a document, `metric_of`), multiplies for hotspot score. Each `FileHotspot` carries its `metric`; the table is labelled by the one the files share. Uses `util::parse_since` for `--since` flag.

### Module structure: `src/knowledge/`

Knowledge maps (Tornhill, code ownership via git blame). Invoked via `km knowledge`.

- **`analyzer.rs`** — `RiskLevel` enum (Critical/High/Medium/Low), `FileOwnership` struct, `compute_ownership()` function that calculates primary owner, concentration, and knowledge loss risk. `compute_bus_factor()` is the truck factor: a file's authors hold at least `AUTHOR_SHARE` (0.75) of its main author's lines, and the authors of the most lines leave first until more than the threshold of the code, weighed by lines, has no author; with `--since`, inactive authors have left before counting.
- **`report.rs`** — Table and JSON output formatters. `--summary` mode aggregates by author (files owned, lines, languages, worst risk) as an alternative to the per-file view.
- **`mod.rs`** — Orchestration: opens git repo, walks files (filtering generated files), runs `blame_file()` per file, computes ownership, sorts/filters results. Uses `util::parse_since` for `--since` flag. `--summary` flag switches to author-level aggregation.

### Module structure: `src/tc/`

Temporal coupling analysis (Tornhill, files that change together). Invoked via `km tc`.

- **`analyzer.rs`** — `CouplingLevel` enum (Strong/Moderate/Weak), `FileCoupling` struct, `compute_coupling()` function that pairs co-changing files, calculates strength = shared_commits / min(commits_a, commits_b), and classifies by threshold (0.5 strong, 0.3 moderate).
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, calls `file_frequencies()` (`eligible_files`: `min_degree`, still in the working tree, code (`read`; with `--across` every kind `Selection::across` takes, and only pairs whose kinds differ are kept), the path filters through `kept_by`), `co_changing_commits()` (skips commits over `--max-changeset` and reverts with the commit they undo), `compute_coupling()` (pairs sharing at least `--min-shared`), sorts/filters results. Uses `util::parse_since` for `--since` flag. No filesystem walk needed — works entirely from git data.

### Shared: `src/git/`

`GitRepo` wraps `git2::Repository`. `blame.rs` runs `git blame --line-porcelain -w -M -C HEAD` (with `.git-blame-ignore-revs` when present) and falls back to libgit2 when the program is missing or fails; `parse_porcelain` joins its lines into runs. Blame and the commit walk read authors through `.mailmap`, and `identity` (email without case, or the name when there is none) is how every command tells people apart. `mod.rs` holds history walks, blame and tree extraction. `changeset.rs` holds what concerns a change set: `diff_stats_since` (ref → working tree), `diff_stats_between` (two refs), `patch_stats` (a patch in git format), all through one `stats_of`, plus `co_change_history` and the merge base they start from. `FileDiffStat` carries the lines touched on both sides and a probe of added lines, so `lines_in` picks the side that matches a text: a patch need not be applied where it is measured. `tree.rs` reads files from the tree of a commit without checking it out (`files_at`, `has_file_at`).

### Shared: `src/projects/`

Projects of a repository and the dependencies between them, read from manifests. Independent of the ecosystem: each one contributes a reader that turns a manifest's text into a `Manifest` (name, raw dependencies, workspace table); discovery, resolution and reach are shared.

- **`mod.rs`** — The model: `Project`, `Edge`, `Scope`, `Manifest`, `DepTarget`, and `reader`, which maps a manifest file name to its reader.
- **`discover.rs`** — `ProjectGraph::from_manifests` builds the graph from `(path, text)` pairs, so the manifests can come from disk (`discover`) or from the tree of a commit. `discover` walks the repository (skipping `node_modules`, `vendor` and `.terraform` anywhere, and `deps`, `_build`, `target` unless under `src`, `lib` or `app`, where they are modules of the project), builds one `Project` per directory and resolves each `DepTarget` (`Path`, `Name` within the same ecosystem, `Workspace`). Workspace roots (`[workspace]`, `workspaces`, `pnpm-workspace.yaml`, `apps_path`, `[tool.uv.workspace]`) are registered before projects, because an npm manifest at one is not a project. Skips `testdata` and `fixtures` inside test directories.
- **`radius.rs`** — `owner` gives the nearest project above a file; `workspace_of` tells when a file is the manifest or lock file of a workspace root, which governs every project under it. `radius` is a reverse BFS: non-dev edges first, then dev edges as terminal reaches, so a shorter dev path never hides a shipping one.
- **`cargo.rs`** / **`npm.rs`** / **`mix.rs`** / **`gomod.rs`** / **`pyproject.rs`** / **`terraform.rs`** — Readers. `terraform.rs` reads every `.tf` file: a module is a directory, named after it (`project_name`), and the `source` of a `module` block that starts with `./` or `../` is a `Path` dependency; `terraform_modules` hands the same to `deps/terraform.rs`. `mix.rs` is lexical: strips comments, then picks out `{:name, ...}` tuples; a `path:` it cannot attribute is counted in `unread`. `pyproject.rs` reads requirements as names and gives a directory to those a source names (`[tool.uv.sources]`, a Poetry `path`); `Ecosystem::key` makes Python names compare whatever their case and separators.

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
- **`role.rs`** — What each file is to the radius: `Source`, `Test` or `Other` (test support, scripts). A file that holds tests is a test when it sits among tests or is a `tests.rs` module, and has a test of its own when it is a source. A Python `tests.py` is a test and a `conftest.py` test support, wherever they sit.
- **`protection.rs`** — How tests protect a source file, in degrees: `Direct` (a test refers to it or sits at the same `place`), `Named` (a test carries its name, or that of its directory, a directory apart, and mirrors no source file itself), `Users` (a file that uses it has a test), `None`. Only `None` is reported as unprotected.
- **`structural.rs`** — Blast radius at source level. Each changed file is narrowed on its own (`Narrowing`); the radius always goes through the exposed dependents. Reads `deps::graph::FileGraph` backwards from the changed sources. `Role` separates sources, tests and the rest (test support, scripts). `Exposure` says whether a dependent calls a changed function, refers to the module, or calls elsewhere. Tests protect what they refer to and what sits at the same `place` in the layout. Measured for every language that has a graph (`is_measured`).
- **`function_reach.rs`** — The radius past distance 1 through functions: a file is reached when one of its public functions calls a reached function (`deps::Calls`, per Elixir file); a `whole` use, a module named without a call, a call outside functions or a file whose functions are not told counts whole. `--max-distance` cuts it (`Structural.cut_at`).
- **`structural_report.rs`** / **`structural_json.rs`** — The structural block as text and as JSON.
- **`documentation.rs`** — Documents left behind by a change (`stale`): one per document, by its strongest `Signal`: `Declared` (`covers:` in its front matter, `[docs] covers`), `Referenced` (a link or code span names a changed file, whole, by its end, or a directory of two names or more above it; `gone` when the change deleted or moved it), `CoChanged` (the documents `missing_co_changes` finds, which `mod.rs` takes out of the logical radius). Documents in the change, changelogs and `[docs] ignore` are left out. **`documentation_report.rs`** renders the block, its JSON and the CI annotations on the changed files; `--fail-on-stale-docs` fails on the first two signals.
- **`projects.rs`** — Blast radius at project level: maps the diff to projects of `crate::projects::ProjectGraph`, computes the reach, and renders its block of the report and of the JSON. A changed workspace root reaches its members at distance 1 with scope `workspace`. `affected()` is the list `--affected` prints: every project when a changed file belongs to none. `--affected` skips the history walk (`print_affected` in `mod.rs`).
- **`report.rs`** — `render_report` builds the table as a `String` and `print_report` prints it, so tests assert on the text. JSON, short and terse formatters.
- **`mod.rs`** — Orchestration: `GitRepo::diff_stats_since` for the diff (merge base with the ref → working tree, deletions included), `GitRepo::co_change_history` for the history, which ends at the merge base so the commits of the diff are never evidence, and skips commits touching more than `--max-changeset` files (sweeping changes relate files by accident). Drops generated files (`util::is_generated`), files already in the diff and files that no longer exist. New files and files outside `--since` go to `without_history`. Test files are always included.

### Module structure: `src/churn/`

Code churn — pure change frequency per source file. Invoked via `km churn`.

- **`analyzer.rs`** — `ChurnLevel` enum (High/Medium/Low), `FileChurn` struct, rate computed as commits ÷ months from the file's first commit, or the start of the `--since` window, to now (minimum 1 month).
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, counts commits per file, computes rate, sorts/filters. Uses `util::parse_since` for `--since` flag.

### Module structure: `src/deps/`

Dependency graph (file-level coupling from imports, cycles via Tarjan SCC). Invoked via `km deps`.

- **`elixir.rs`** — Elixir, where dependencies are between modules. `parse` sets comments and literals aside, names nested modules by indentation, undoes `alias`, and returns every module referred to with the function called on it. `changed_functions` maps touched lines to the public functions affected, carrying a change to a private function to the public ones that call it. Outside functions, `alias`/`require` are neutral and a module attribute changes the functions that read it; anything else is `Err(line)`. `is_build_config` tells `mix.exs`, `.formatter.exs` and `config/*.exs`, which neither define nor use modules in the graph.
- **`elixir_directives.rs`** — `import`, `use` and `defimpl`. `import Foo, only: [...]` gives calls; an import without `only:` and a protocol implemented are `whole` (any function may be reached); `use` is kept as `Using` (module and option). `bare_calls` gives the calls without a module that the file does not define.
- **`elixir_calls.rs`** — `calls`: for each public function of an Elixir file, the `Module.function` calls it makes, directly or through the functions of its file it reaches; `Source::function_calls` hands them to `km impact`.
- **`elixir_using.rs`** — What a file takes from modules of the project without naming it: `complete` reads again each file that `use`s one, with the `Injection` of the function the `use` runs (`name` for `use Foo, :name`, else `__using__`) and of what it splices with `unquote(helper())`: aliases, imports, references. Then each whole `import` of a module of the project answers the bare calls that match its public functions. `graph.rs` calls it before resolving references; `Use.whole` keeps that an edge may reach any function, so `km impact` reads such a user as referring to the module when the calls seen are left alone.
- **`elixir_compile.rs`** — What a file needs at compile time (`CompileTime`): the modules it `use`s and the calls it makes, outside `quote` blocks, plus what its `use`s inject. `graph.rs` marks `EdgeKind::Compile` a `use` and a call to a public macro or guard of the file used (`macros`). `import` and `require` alone are no compile-time use.
- **`elixir_literals.rs`** — `code_only` blanks comments, strings, sigils and character literals, keeping lines and columns. The code a string interpolates (`#{...}`) stays: it runs like any other.
- **`heex.rs`** — HEEx templates. `code` keeps only the Elixir of a template (braces, EEx tags, components called as tags; `<.button` as the local call `button(`), so prose never reads as a module; `owners` tells the module that renders a template kept in a file of its own. `embedded` picks the `~H` ones out of a source. `elixir::parse_with` reads both kinds with the aliases of the file. `km impact` hands the templates over; `km deps` reads only the `~H` ones.
- **`phoenix.rs`** — What Phoenix relates by name alone: a controller uses the views named after it (`PageController` and `PageJSON`, `PageHTML`, `PageView`).
- **`routes.rs`** — Phoenix routes, lexically. `routes` reads a router: each `scope` adds its path and alias to what is inside, by indentation. `requests` reads the paths a test asks for, only from calls that make a request. `serves` matches the two segment by segment.
- **`rust_literals.rs`** — Blanks comments and character literals of Rust, keeping lines; a string keeps its quotes and its content is handed apart, for `#[path = "…"]`.
- **`rust.rs`** — Rust, read lexically: the `mod` declarations of a file (with `#[path]` and the inline modules around them), every path of `use` trees and of code, the names `use` brings in, the types `impl` blocks are for, and whether the file holds `#[test]`s.
- **`rust_crates.rs`** — The module tree of each crate, grown from its root files by `mod` declarations, and `uses`: each path made absolute from the module it is written in and led to the file of the deepest module it names. Library crates answer to their package name. A module may have several files (`mod imp;` under opposite `#[cfg]`s, each with its `#[path]`); a path goes on through a crate re-exported under a name (`pub extern crate x as y`, `pub use x as y`). `mod x;` is no edge; a type uses the files of its `impl` blocks.
- **`graph.rs`** — `FileGraph::build` builds the file graph from `(path, language, text)` sources, so they can come from disk or from a commit's tree. `Layout` carries what is known besides the sources: paths to look up, the Go modules, the Cargo packages, the import roots of Python projects, the aliases and packages of JavaScript. Elixir references resolve through a module index (nearest file, then `Related`: what the projects declare); Python, Go, C/C++, Java, C# and Ruby are read apart (`Apart`: `python_uses`, `golang`, `c_family`, `java`, `csharp`, `ruby_files`); the rest go through `extract_imports` + `resolve_import`. `Use.calls` keeps the functions called. Each `Use` has an `EdgeKind`: `Compile` (Elixir, `elixir_compile`), `Load`, `Deferred` (a Python import in a function, `import()`) or `Type` (`import type`, `if TYPE_CHECKING:`); `analyzer::build_graph` leaves the last two out of cycles but counts them in fan-in/fan-out, and `km impact` follows them all, except that a test's `Type` import protects nothing. Readers return a `Found`: the uses, their kinds and the imports that look internal but resolve to no file (`FileGraph.unresolved`): a quoted include that is no system header, a relative specifier of a script or relative Python import, a Go import within a module of the repository.
- **`extractor.rs`** — `extract_imports` dispatches by language name and returns raw import strings: the specifiers of the import forms for JS/TS, the import paths of Go (`golang_source`). Elixir and Rust are read apart, over the whole graph. `is_supported` decides which languages enter the graph.
- **`python.rs`** — Python extraction and resolution. `extract` reads every `import` and `from … import`, on several lines too, outside docstrings; `from a.b import c` is kept as `a.b:c`, since `c` may be a submodule. `resolve` looks an absolute import up from each ancestor of the importer that is not a package, and from `src` under each; a relative one from the importer's package. Then come the roots `Layout.python_roots` holds for the nearest project: what its `pyproject.toml` declares, read by `projects::python_roots`.
- **`python_blocks.rs`** — Where a Python statement runs, by indentation: under `if TYPE_CHECKING:` (`Type`), in a function (`Deferred`) or on load. Lines that go on inside brackets change no block.
- **`python_names.rs`** — What a Python module offers, by lines and indentation: `defined` gives its top-level names (functions, classes, assignments), `changed` the ones a change to some lines affects, carrying it to the names that mention them (`Err(line)` for code that runs on import), and `taken` what a file reads on a module it imports. `python::read` keeps the name each import binds.
- **`python_uses.rs`** — The files a Python source imports, each with the names it takes there, as `module.name`: what fills `Use.calls` for Python.
- **`golang.rs`** — Go, where an import names a package and a package is a directory. `Packages` holds the files of each directory that a package offers (no `_test.go`) and the modules of the repository, by the path each `go.mod` declares; an import leads to every file directly inside the directory it names. No edges between the files of one package, nor from a file to its own package unless it is an external test.
- **`golang_source.rs`** — The package clause and the imports of a Go source, read from tokens so that comments and literals hold no imports.
- **`javascript.rs`** — JavaScript and TypeScript. `extract` reads the specifiers of the import forms only (`import`, `export … from`, `require()`, `import()`) through a coarse tokenizer that skips comments. `resolve` takes a relative specifier from the importer, any other through the `paths` and `baseUrl` of the nearest config, then through the packages of the repository by name; `file_at` tries extensions, the TypeScript source of a `.js` and `index` files.
- **`javascript_imports.rs`** — The import forms over the tokens of `javascript.rs`, each with its `EdgeKind`: type-only imports and re-exports, `import()` as deferred.
- **`javascript_config.rs`** — What `tsconfig.json`/`jsconfig.json` (JSON with comments, `extends` followed within the repository) and `package.json` (name and entry points) say about imports, as `Modules` in `Layout.javascript`.
- **`c_family.rs`** — C and C++: `includes` reads `#include` outside comments; `Files` resolves each one beside the includer (quoted), then in the directories above and their `include`, then by path suffix when one file is nearest. A header's sources (same name beside it, or `include/` mirrored to `src/`) are `implied` uses: in `FileGraph::implied`, not in `edges`, so `km deps` shows no cycle they would make, while `km impact` follows them (`uses_of`).
- **`c_like.rs`** — The tokens of Java and C#, without comments and literals (text blocks, raw, verbatim and interpolated strings, characters). A leading byte order mark is skipped.
- **`c_like_reader.rs`** — `chain_at` reads a chain of names (`a.b.C`, `a.b.*`); `Reader` is the walk the Java and C# readers share over a `Cursor`: braces through `punct`, `top_level` outside every type, `name` anywhere, never from a word after a dot.
- **`types.rs`** — `Types`: the files that declare each type, by qualified name. `Scope` is what a file sees: namespaces whole, aliases, enclosing namespaces; `resolve` reads a chain of names (`models.User.find`) to the type it starts with, `holding` gives the outer type of a nested one or a member.
- **`java.rs`** — Java: `read` gives the package, the imports (single, static, on demand), the top-level types and every chain of names of a file (`c_like_reader::chain_at`); `Classes` resolves them through `Types`, keeping the nearest of several declarations of one type (`types::nearest_all`).
- **`csharp.rs`** — C#: `read` follows the braces as namespace, type or body frames; at namespace level it reads `using` (plain, `global`, `static`, aliases), `namespace` (block or file-scoped) and type declarations (`record struct`, `delegate` too), and every chain of names, an attribute also as `NameAttribute`. `Namespaces` resolves through `Types`: a file sees its namespaces and their parents, the global namespace, its usings and the `global using`s at or above its directory.
- **`csharp_extensions.rs`** — C# extension methods: `read` records the non-private ones a top-level type declares (`this` first parameter) with their arity, and every call after a dot with its argument count. A call uses the one file that declares a method of that name and arity in a namespace the file sees; when several do, it is ambiguous and adds no edge.
- **`ruby_literals.rs`** — The tokens of Ruby, line by line, each with the column of its first token: comments, `=begin`/`=end` and what follows `__END__` give none; a string, heredoc, `%` literal, regular expression or character is one `Str` token on the line it starts on, while the code a string interpolates is read as code. `/`, `%`, `?` and `<<` open a literal or are operators by what precedes them.
- **`ruby.rs`** — Ruby: `read` follows classes and modules by indentation and gives the requires (`require_relative`, `require`, `load`, `autoload`, paths built from `__dir__`/`__FILE__`), the constants defined (a module that only holds other definitions is a namespace; a constant assigned in one is a definition) and every constant referred to, with the namespaces it is looked up from.
- **`ruby_files.rs`** — `Files` resolves Ruby over the graph: requires by path (`require_relative`) or under `lib`, `test`, `spec` and `app/*`; constants through an index of definitions, always on, whole from the innermost namespace out. Several definitions: the file named after the constant (`underscore`), else the nearest when they share a directory, else ambiguous (counted in `FileGraph::ambiguous`). Core classes are skipped, and only tests see what tests define.
- **`terraform.rs`** — Terraform files, read only when `--kind infra` is chosen: a file that calls a local module uses every `.tf` file of its directory (`Modules`, by directory); a local source that leads to no module is unresolved.
- **`markdown.rs`** — Markdown documents, read only when `--kind docs` is chosen: each links to what `docs::links::Notes` resolves its links and wikilinks to; a broken link is unresolved.
- **`kaikai.rs`** — Kaikai extraction and resolution: `import a.b.c` names a file under a package root, found by trying each ancestor directory of the importer; a package directory (`kai.toml`) names the file of its name, or every file in it when it has none. Each file is a module and the files of a package import each other, so they have no edges besides their imports (unlike Go).
- **`edges.rs`** — The file-level edges, kept in `DepResult.edges` (sorted, with their `EdgeKind`); the JSON lists those that leave the files shown.
- **`matrix_tree.rs`** — The order of the matrix over `crate::file_tree::Tree`: `order` sorts the children of each directory so that a sibling comes before those it uses, by the uses that load or compile: SCCs stay together, inside one by uses given minus taken, ties by name.
- **`matrix.rs`** — The dependency structure matrix: `Expand` (`--depth`, `--focus`) picks the directories shown expanded, `Matrix::build` the rows and the cells (files of the column using files of the row), each with its `Mark` (cycle, against the layering, type-only or deferred).
- **`matrix_json.rs`** — The matrix as JSON (`--matrix --format json`): rows, expanded groups, every cell with its mark and uses, and the cells above the diagonal.
- **`matrix_text.rs`** — The matrix as text: rows numbered and named, columns by number, `!` on cells of a cycle, colours on a terminal, and the cells above the diagonal with the uses behind each.
- **`matrix_html.rs`** — The matrix as a page of `crate::html`: the tree already ordered, the files and the edges (with whether each loads and is in a cycle) as embedded JSON, and the directories to open expanded; `matrix_html.js` expands, collapses and counts the cells again in the browser, `matrix_html.css` styles it.
- **`analyzer.rs`** — `resolve_import` maps a raw import to project files per language (JavaScript and TypeScript resolve through `javascript::resolve` in the graph, since they need the layout); `build_graph` computes fan-in, fan-out and cycles (`tarjan_scc`, iterative).
- **`report.rs`** — `render_report` builds the table as a `String` and `print_report` prints it, so tests assert on the text. JSON, short and terse formatters.
- **`mod.rs`** — Orchestration: walks files, leaves unsupported languages out of the graph and counts them, extracts and resolves imports. `run` hands the matrix to `matrix_output` (printed, refused wider than the terminal, given as JSON, or written with `--html`) or prints the table (`run_table`); `sort_entries` and `visible_entries` hold the `--sort-by` and `--cycles-only`/`--top` logic apart from printing.

### Shared: `src/html.rs`

Self-contained HTML pages: `render` puts a `Page` (title, styles, body, script) in the shared shell with `html/base.css` (theme tokens, light and dark) and its data as the JSON of `#data`; `script_json` writes that JSON so no `<` can close the element. No external assets.

### Shared: `src/report_helpers.rs`

Output conventions every command follows. `shown_count` gives how many entries an output lists: a table `--top` rows (`DEFAULT_TOP`), JSON every entry unless `--top` is given, summaries and CI annotations every entry; `total_label` says when a totals row covers more files than it shows; `output_results` hands a table every entry and how many to show. `set_path_base` (called by `main::run_command`) and `shown` print every path relative to the repository root, or to the analyzed directory outside one.

### Shared: `src/detection/`

Function detection for `cycom`, `cogcom`, `smells` and `init`. `code_lines` gives the code lines they read, without preprocessor directives and their `\` continuations in the C family. `mod.rs` walks brace-scoped code: a line is a declaration by marker, or by shape over the signature it spans (`shape::signature`, `shape::is_c_family_function`), unless it starts inside open parentheses (`shape::continuation_lines`) or is an annotation or attribute. `body.rs` reads the body by braces (`BodyScan`; braces of a type in the parameter list are not the body). `script.rs` holds what `FunctionSyntax::Script` (JavaScript, TypeScript) adds: functions bound to a name and regex literals. `indented.rs` reads indent-scoped bodies and leaves `quote` blocks out of them.

### Shared: `src/string_mask.rs`

`multi_line_string_mask` marks the interior lines of triple-quoted strings (`starts_in_string_mask` also marks the line that closes one, for readers of statements such as `deps/python.rs`);
`demote_multi_line_strings` turns those lines into `LineKind::Blank` so the
analyzers that select `LineKind::Code` skip them. Used by `hal`, `cycom`,
`cogcom`, and `smells` — prose or data inside a multi-line literal must not
count as control flow. `loc` is deliberately left alone: those lines are still
code for line counting. `code_view` is what those analyzers read: the same
demotion, and each raw string left as `""`, with the lines wholly inside one
turned into blanks. Code of a language that ignores case comes in
lower case (`case_fold`), and one without backslash escapes without its
backslashes.

### Shared: `src/case_fold.rs`

Languages that ignore case (`LanguageSpec::case_insensitive`: Fortran, Pascal)
are read in lower case by every analyzer that goes through `code_view`. In
Fortran the end of a construct written apart is joined (`end if` reads
`endif`), so it never reads as the construct it closes. Lines keep their
length, so `name_at` gives back a function name as written: `cycom`, `cogcom`
and `smells` report names that way.

### Shared: `src/unit_bodies.rs`

Functions closed by words, for `FunctionSyntax::Fortran` and
`FunctionSyntax::Pascal` (`detection::detect_function_bodies` hands them
over). Fortran: program units to the `end` that closes them, contained units
included, interface blocks skipped. Pascal: routines of the implementation to
the `end` of their `begin` block, after their local routines; the `interface`
section, class types, `forward` and `external` have no body. Unindented
sections and routines bound what conditional compilation leaves unbalanced,
and headers repeated under it with the routine's own name are one routine.
`cogcom` reads Fortran nesting from constructs (`fortran_statement`), Pascal
nesting from indentation, two columns a level.

### Shared: `src/test_blocks.rs`

`test_block_mask` marks the lines of each item under `#[cfg(test)]` of a Rust
file, attribute included: to where its braces balance, or to its `;` or `,`
when it has none. Comments and literals are read apart
(`deps::rust_literals`). `without_test_blocks` turns those lines into
`LineKind::Blank`, and `production_kinds` does so for a Rust file when tests
are excluded. `util::read_and_classify` applies it, so every analyzer that
reads through it measures production code; the code after a test module is
still measured. `loc` and the line counts of `report` keep those lines.

### Module structure: `src/smells/`

Code smell detection. Invoked via `km smells`.

- **`rules.rs`** — Long functions and TODO debt.
- **`params.rs`** — Long parameter lists: the list after the function's name, top-level commas only.
- **`magic.rs`** — Magic numbers (note: `'_'` is a valid digit separator, included in `is_numeric_char`). `Declarations` follows a declaration across the data it opens (enum members, Go `const (`, tables, `type` unions); named keys and parameter defaults name their value.
- **`commented.rs`** — Commented-out code, outside documentation comments (`documentation_lines`).
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

- **`analyzer.rs`** — Aggregates `git blame` data across all files to produce per-author totals: files owned, lines, languages; `set_last_commits` dates each author by their last commit (`GitRepo::last_commit_times`).
- **`report.rs`** — Table and JSON output formatters.
- **`mod.rs`** — Orchestration: opens git repo, runs blame per file, aggregates by author. Uses `util::parse_since` for `--since` flag.

### Module structure: `src/report/`

Comprehensive multi-metric report. Invoked via `km report`.

- **`data.rs`** / **`builder.rs`** — Data types and per-file metric collection (LOC, dups, indent, Halstead, cyclomatic, MI).
- **`markdown.rs`** — Human-readable table report.
- **`json.rs`** — JSON output formatter.
- **`mod.rs`** — Orchestration: walks files once, runs all analyzers, emits unified report.

### Module structure: `src/map/`

Enclosure diagram of the code (Tornhill): directories as circles holding their files, a file's area by its code lines, coloured by a metric. Invoked via `km map --html FILE` or `--format json`.

- **`metric.rs`** — `ColorBy` (`score`, `hotspots`, `churn`, `age`, `knowledge`, `cycles`, `llm` with the thresholds of `[map]`) and the `Shade` of each file: a heat from -1 (red) through 0 (grey) to 1 (green) and the text shown on hover, keyed by the path under the analysed directory. Reads `hotspots::collect`, `churn`/`age` classifiers over `GitRepo`, `knowledge::collect` and `deps::analyze`.
- **`pack.rs`** — Circle packing as `d3.pack`: siblings placed on a front chain (Wang et al.); **`enclose.rs`** the smallest enclosing circle (Welzl, shuffled with a fixed seed).
- **`layout.rs`** — The `file_tree::Tree` of the files laid out: children packed largest first, then by name, the root `SIZE` across; every node after its parent.
- **`callout.rs`** — The share of the lines on each side of the scale, and the directory where each side gathers (many such lines, a large share of the directory, a part of the code rather than nearly all), labelled with the `Words` of the metric.
- **`page.rs`** — The page of `crate::html`, with `map.js` (SVG, zoom by view box, tooltip, legend), `map_callouts.js` (the labels) and `map.css`.
- **`mod.rs`** — Orchestration: scores the files once (`score::measure`/`snapshot_of`) for their lines and the `score` colour, adds the shades, lays out and writes the page or the JSON.

### Shared: `src/kind.rs`

What each file is for: `Kind` (`Code`, `Docs`, `Infra`, `Data`), by language and path, and for YAML by its first lines (Kubernetes, CloudFormation, Ansible). `Selection` carries `--kind`, `[kinds] separate` and the globs `[kinds]` declares (`chose` tells whether `--kind` names a kind); it travels inside `walk::ExcludeFilter`, so every command reads it. `measures` decides what metrics of code read (without `--kind`, what they always read), `reads` what `loc` and `deps` read, `measures_path` the same for paths of history, and `kind_of_path` their kind.

### Module structure: `src/docs/`

Measures of documentation, the files of kind `docs`. `km docs refs`, `km docs links`, `km docs stale` and `km docs context` read Markdown documents whatever `--kind` says (`dispatch`, `main::on_kind`).

- **`markdown.rs`** — `scan` reads a document by paragraphs (`Block`: code spans and link texts go on across lines, each character keeps its line), skipping front matter, fenced and indented code and HTML comments. It gives the `Reference`s (links, images, reference definitions, `href`/`src`, code spans) and the anchors: headings named as GitHub names them (`slug`, `-1` for a repeated one) and `id`/`name` of HTML elements.
- **`refs.rs`** — `check` finds the dead references of a document against a `Repository`: the working tree (ignored files left out) with every trailing part of its paths, so `deps/graph.rs` names `src/deps/graph.rs`. A link is dead when its file or its heading is missing; an absolute link without extension is a route of a site and is not checked. A code span shaped like a path is dead when the history held it (`GitRepo::paths_in_history`, read only when a span names a path the tree lacks), or outside git when it starts at a name of the tree.
- **`report.rs`** — Table, JSON, one line, GitHub and CodeClimate annotations.
- **`links.rs`** — The links between documents: `links` gives the Markdown links and `[[wikilinks]]` (`Form::Note`) of a text; `Notes` resolves them by normalized path, or a wikilink by path from the root, else by note name without case or extension, the nearest of several (`deps::graph::nearest`). `Graph` holds the edges and broken links, fan-in and the groups (union-find); `is_entry_point` tells READMEs and indexes, which are no orphans. `deps/markdown.rs` uses `Notes` too, so `km deps --kind docs` reads the same graph (`Selection::chose`). **`links_report.rs`** prints `km docs links`.
- **`context.rs`** — `km docs context`: the `Role` of the files that steer agents (by name and path), approximate `tokens` (characters / 4), `rules` (lines of prose that ask), `repeated` lines across files (normalized, 30 characters or more), and `loads`: per directory and family (`claude`, `agents`, `gemini`), the instructions from the root down. **`context_report.rs`** runs it: walks with `walk::walk` (no language needed), reads git through `commits_by_file`, leaves links to an already read file out of the repetition, and prints with `report_helpers::table`.
- **`describes.rs`** — What a document describes, shared with `km impact`: `Rules` (`[docs] covers` and `ignore`, changelogs), the globs of `covers:` in a front matter (`declared_in`), and `names_of`/`names`: the path a reference names and whether it names a file, whole, by its end, or as a directory of two names or more above it.
- **`stale.rs`** — `km docs stale`: `Files` indexes the files of the tree by every end of their paths and by their directories; a document is behind when a file it covers or names has commits after its last one (`GitRepo::commits_by_file`, through `git log`). **`stale_report.rs`** prints it.
- **`mod.rs`** — Orchestration: the root of the repository, the documents of the walk, `--since-ref` (documents changed since the ref keep every dead reference, the rest only those that name a changed path, whole or by its end) and `--fail-on-dead`.

### Module structure: `src/infra/`

Measures of infrastructure code, the files of kind `infra`. `km infra smells` reads them whatever `--kind` says (`main::on_kind`, as `km docs` does).

- **`lines.rs`** — `Format` by language (`Hcl`, `Dockerfile`, `Yaml`, `Other`); `split` gives each line's code and comment (HCL `#`, `//`, `/* */`, heredocs as code; whole `#` lines in a Dockerfile; `#` outside quotes in YAML, where a quote opens only where a value starts). `assignments` reads `key: value`, `key = value`, `KEY=value` and `ENV`/`ARG`.
- **`secrets.rs`** — `names_secret` by the last word of a key (camel case and separators split), `is_literal` (variables, interpolations, references, tags, paths, numbers are not; an unquoted HCL value is an expression), `shaped`: known key prefixes and private keys.
- **`smells.rs`** — `Smell` (also the values of `--fail-on`) and `scan`: per line, the smells of assignments (secret, empty password, admin, image, action), shapes, `0.0.0.0/0`, `http://` to hosts that need TLS, MD5/SHA-1, suspicious comments, CI permissions; a Dockerfile adds unpinned `FROM` and a last `USER root`. One smell of each kind per line; secret values never enter `detail`.
- **`report.rs`** — Table (most severe first, through `report_helpers::table`), JSON in the shape of `km smells`, one line, GitHub and CodeClimate annotations by severity.
- **`mod.rs`** — Orchestration: the walk, `--since-ref` (`docs::changed_since`), `--fail-on`.

### Shared: `src/file_tree.rs`

Files as a tree of directories, rooted at the deepest directory holding every file, a directory holding only another merged with it. `deps/matrix_tree.rs` orders it for the dependency matrix; `map/layout.rs` packs it.

### Module structure: `src/score/`

Overall code health score (A++ to F--). Invoked via `km score`. Static metrics only (no git required); `--trend` and `km score diff` require a git repo.

- **`analyzer.rs`** — `Grade` enum (16 grades: A++ to F--), `DimensionScore`/`FileScore`/`ProjectScore` structs, `score_to_grade()`, `compute_project_score()`, `Grade::numeric_rank()` (used for gate comparisons), `Grade::parse()` (for `--fail-below` CLI arg), and 6 normalization functions. Halstead normalization uses effort-per-LOC.
- **`report.rs`** — Table and JSON output formatters.
- **`diff.rs`** — `ScoreDiff`/`ScoreDelta` types and `compute_diff()`. Asserts dimension count and names match before zipping to prevent silent model mismatches.
- **`diff_report.rs`** — Table and JSON formatters for diff output.
- **`changed.rs`** — `--gate-scope changed`: pairs `GitRepo::changes_since_in_workdir` with per-file scores from both snapshots (keyed by path relative to the walked directory, displayed repo-relative), and fails when a file ending below the ref's project score dropped more than the tolerance (default 0.5, `GateScope::default_tolerance`), or when absolute duplicated lines grow. Files above that score, new files and deleted files never fail.
- **`mod.rs`** — `ScoreGate` struct (`max_drop` from `--fail-if-worse` + `--gate-tolerance`, compared on unrounded scores; `scope` from `--gate-scope`; `fail_below`). `compute_snapshot()` keeps all per-file scores and duplicated lines for the changed-scope gate; `compute_score()` wraps it. `run()` for normal score. `run_diff()` for `--trend`/`km score diff`: computes before/after snapshots, prints report, then evaluates quality gates (gates always evaluated after output so CI logs are complete).
- **`scoring.rs`** / **`collector.rs`** / **`normalizer.rs`** — Dimension definitions, per-file metric extraction, and piecewise linear normalization curves. In a cognitive model the reading-effort dimension is an `Effort`: Halstead, or the Visual Studio MI for `ScoringModel::ExperimentalMi`.
- **`badge.rs`** — `--badge`: the score as a self-contained flat SVG (width estimated from the text, colour by the letter of the grade) or as the JSON of a shields.io endpoint badge, by the extension of the file.
- **`experimental.rs`** — `--experimental`: `measure` once with the experimental model, then `snapshot_of` it with both models, so the default score is printed beside the experimental one.
