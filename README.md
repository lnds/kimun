# Kimün (km)

A fast command-line tool for code analysis, written in Rust. Run `km score` on any project to get an overall health grade (A++ to F--) across five quality dimensions — cognitive complexity, duplication, indentation depth, Halstead effort, and file size — with a list of the files that need the most attention.

```
$ km score
Code Health Score
──────────────────────────────────────────────────────────────────
 Project Score:  A (91.2)
 Files Analyzed: 142
 Total LOC:      19,084
──────────────────────────────────────────────────────────────────
 Dimension                 Weight   Score   Grade
──────────────────────────────────────────────────────────────────
 Cognitive Complexity         32%    84.1   B+
 Duplication                  21%   100.0   A++
 Indentation Complexity       16%    93.2   A+
 Halstead Effort              21%    87.5   A-
 File Size                    10%   100.0   A++
──────────────────────────────────────────────────────────────────

 Files Needing Attention (worst per-file scores, excl. duplication)
──────────────────────────────────────────────────────────────────
 Score  Grade  File                       Issues
──────────────────────────────────────────────────────────────────
  57.7  D-     ./src/deps/analyzer.rs     Cognitive: 36
  61.0  D      ./src/cogcom/analyzer.rs   Cognitive: 35
  70.3  C      ./src/smells/rules.rs      Cognitive: 20
  71.6  C      ./src/hal/tokenizer.rs     Cognitive: 22
  71.8  C      ./src/loc/mod.rs           Cognitive: 19
  72.8  C      ./src/knowledge/mod.rs     Cognitive: 21
  73.8  C+     ./src/dups/mod.rs          Cognitive: 19
  75.0  C+     ./src/cli_help.rs          Cognitive: 17
  75.6  C+     ./src/test_blocks.rs       Cognitive: 17
  77.3  B-     ./src/walk.rs              Cognitive: 16
──────────────────────────────────────────────────────────────────
```

That is Kimün measuring its own source: 140 files and 18,963 lines of code, tests left out, in 0.11 seconds on an Apple M3 Max. Counting all 235 files of `src/`, tests included, takes `km loc` 0.04 seconds and `cloc` 2.4.

**Website: [kimun.tools](https://kimun.tools/en/)** · [A two-minute video](https://kimun.tools/en/#video) of the grade, the metrics, what git remembers, the impact of a change and the gate for CI · Also [in Spanish](LEEME.md).

Beyond the aggregate score, Kimün provides 18 specialized commands:

- **Static metrics** — lines of code by language (classified as [cloc](https://github.com/AlDanial/cloc) classifies them), duplicate detection (Rule of Three), Halstead complexity, cyclomatic complexity, cognitive complexity (SonarSource), indentation complexity, two Maintainability Index variants (Visual Studio and verifysoft), code smell detection, dependencies between files and their cycles, and a comprehensive multi-metric report.
- **Git-based analysis** — hotspot detection (change frequency × complexity, Thornhill method), code churn (pure change frequency), code ownership / knowledge maps via `git blame`, temporal coupling between files that change together, impact of a diff (diffusion and missing co-changes), per-author ownership summary, and file age classification (Active / Stale / Frozen).
- **For coding agents** — every command that measures can answer in JSON, and `km ai` is an optional integration with Claude that runs the tools and writes a narrative report. No other command calls a model.

> Note: This repository is not the same as the Rust console note-taking app `kimun` by nico2sh. That project is available at https://github.com/nico2sh/kimun.

## Installation

```bash
cargo install kimun              # from crates.io
brew install lnds/kimun/kimun    # with Homebrew, on macOS and Linux
```

Either one installs the `km` binary. There are also ready-made binaries for macOS (Apple Silicon), Linux (x86_64 and arm64) and Windows (x86_64) on the [releases page](https://github.com/lnds/kimun/releases). From a checkout of this repository, `cargo install --path .` builds it from source.

### Shell completions

Generate and install a completion script for your shell:

```bash
# zsh
km completions zsh > ~/.zfunc/_km
# add to ~/.zshrc if not already present:
#   fpath=(~/.zfunc $fpath)
#   autoload -Uz compinit && compinit

# bash
km completions bash > /etc/bash_completion.d/km

# fish
km completions fish > ~/.config/fish/completions/km.fish
```

## Commands

Two conventions hold for every command:

- **`--top N`** limits the rows of a table (20 by default), and JSON only when given; the totals of a table, the `short` and `terse` summaries and the CI annotations (`github`, `codeclimate`) always cover every file. A totals row over more files than it shows says so: `Total (30 files, 20 shown)`.
- **Paths** are shown relative to the root of the git repository, or to the analyzed directory outside one, without a leading `./`, whatever directory km runs from and however the path argument is written, so the output of two commands can be joined.

### `km loc` -- Count lines of code

```bash
km loc [path]
```

Run on the current directory:

```bash
km loc
```

Run on a specific path:

```bash
km loc src/
```

Test files are left out by default, so the totals are lower than those of `cloc` on the same directory. With `--include-tests` both count the same files.

Options:

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Show summary stats (files read, unique, ignored, elapsed time) |
| `--by-author` | Break down lines of code by git author (requires a git repository) |
| `--include-tests` | Count test files and directories too (left out by default) |
| `--format {table,json,short,terse}` | Output format (default: table) |

Example output:

```
────────────────────────────────────────────────────────────────────
 Language                Files        Blank      Comment         Code
────────────────────────────────────────────────────────────────────
 Rust                        5          120           45          850
 TOML                        1            2            0           15
────────────────────────────────────────────────────────────────────
 SUM:                        6          122           45          865
────────────────────────────────────────────────────────────────────
```

### `km dups` -- Detect duplicate code

Finds duplicate code blocks across files using a sliding window approach. Applies the **Rule of Three**: duplicates appearing 3+ times are marked as **CRITICAL** (refactor recommended), while those appearing twice are **TOLERABLE**.

Test files and directories are excluded by default, since tests often contain intentional repetition.

The occurrences of a group never overlap within a file, so a run of repeated lines (a table of `0xFF`) is not a copy of itself. A block grows while every occurrence goes on matching, even when the next lines also appear elsewhere. The duplicated lines of a project are counted once each, however many groups hold them, so they never exceed the code lines.

```bash
km dups [path]
```

Options:

| Flag | Description |
|------|-------------|
| `-r`, `--report` | Show detailed report with duplicate locations and code samples |
| `--show-all` | Show all duplicate groups (default: top 20) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |
| `--max-duplicates N` | Exit with code 1 if duplicate groups exceed this limit (`--max-duplicates 0` fails on any duplicate) |
| `--max-dup-ratio PERCENT` | Exit with code 1 if the duplicated-lines ratio exceeds this percentage (e.g. `--max-dup-ratio 5.0`) |
| `--fail-on-increase REF` | Exit with code 1 if the current duplication ratio is higher than at the given git ref (e.g. `origin/main`). Prevents debt from growing silently in CI |
| `--format {table,json,short,terse}` | Output format (default: table) |

Example summary output:

```
────────────────────────────────────────────────────────────────────
 Duplication Analysis

 Total code lines:                                             3247
 Duplicated lines:                                              156
 Duplication:                                                  4.8%

 Duplicate groups:                                               12
 Files with duplicates:                                           8
 Largest duplicate:                                        18 lines

 Rule of Three Analysis:
   Critical duplicates (3+):     7 groups,    96 lines
   Tolerable duplicates (2x):    5 groups,    60 lines

 Assessment:                                                    Good
────────────────────────────────────────────────────────────────────
```

Example detailed output (`--report`):

```
────────────────────────────────────────────────────────────────────
 [1] CRITICAL: 18 lines, 3 occurrences (36 duplicated lines)

   src/parser.rs:45-62
   src/formatter.rs:120-137
   src/validator.rs:89-106

 Sample:
   fn process_tokens(input: &str) -> Vec<Token> {
       let mut tokens = Vec::new();
       for line in input.lines() {
       ...

────────────────────────────────────────────────────────────────────
 [2] TOLERABLE: 12 lines, 2 occurrences (12 duplicated lines)

   src/main.rs:100-111
   src/cli.rs:200-211

 Sample:
   match result {
       Ok(value) => {
       ...
────────────────────────────────────────────────────────────────────
```

#### Excluded test patterns

By default, `km dups` skips files matching common test conventions:

- **Directories**: `tests/`, `test/`, `__tests__/`, `spec/`, and .NET test projects, named after what they test (`App.Tests/`, `Orders.UnitTests/`, `Api.IntegrationTest/`)
- **Examples and benchmarks**: `examples/`, `example/`, `benches/`, `bench/`, `benchmarks/`, `benchsuite/`, code that is run to show or time the project rather than shipped with it
- **By extension**: `*_test.rs`, `*_test.go`, `test_*.py`, `*.test.js`, `*.spec.ts`, `*Test.java`, `*_test.cpp`, and more
- **Rust test modules**: a file declared under `#[cfg(test)]` without a body (`#[cfg(test)] mod testutil;`, with or without `#[path]`), and the modules below it

The same rules apply to every command. Use `--include-tests` to analyze them as well, and `--list-excluded` to see which files are left out and why.

#### Inline tests

Rust keeps tests inside the source file, under `#[cfg(test)]`. Every command that measures code leaves those items out along with the test files: `dups`, `indent`, `hal`, `cycom`, `cogcom`, `mi`, `miv`, `smells`, `hotspots`, `score` and the metrics of `report`. Only the item under the attribute is skipped, so the code after a test module is measured. `--include-tests` brings them back.

`km loc` and the line counts of `km report` still count them: they are lines of the file, as `cloc` counts them.

#### Generated files, data and prose

The commands that measure code (`dups`, `indent`, `hal`, `cycom`, `cogcom`, `mi`, `miv`, `smells`, `score`, `report`, `init`, `hotspots`, `churn`, `tc`) leave out:

- **Generated files**, by name (lock files, `*.min.js`, `*.bundle.js`, `*.pb.go`, `*_pb2.py`, `*.generated.*`) or by content:
  - a comment in the first 20 lines saying the file is generated and not to be edited (Go's `// Code generated ... DO NOT EDIT.`, protoc);
  - `@generated`, or C#'s `<auto-generated`;
  - the banner of Bison or Flex, or the helpers esbuild puts at the top of a bundle;
  - bundler output: a `sourceMappingURL` comment at the end of the file, or a `<name>.map` beside it.
- **Data and prose**: JSON, YAML, TOML, XML, Markdown, Text, Properties and PO files. Their indentation is structure, not nesting, and they change with every release without being code.

`km knowledge` and `km authors` leave out generated files. `km loc` and `km deps` read every file: lines are lines, and generated code is still imported.

### `km indent` -- Indentation complexity

Measures indentation-based complexity per file: standard deviation of indentation depths and maximum depth. Higher stddev suggests more complex control flow.

```bash
km indent [path]
```

Depth is read as structure, not layout. A line that continues a statement is at most one level deeper than the line it continues: arguments aligned under an opening parenthesis, a line after one ending in `::`, `&&`, `||`, `|>`, `|` or `\`, and a line starting with `.`, `|`, `&&` or `||`. Inside a block opened within parentheses (the body of a closure or of an Elixir `fn`), depths count again from that block. A file with fewer than 10 code lines is not graded (`-`): the spread of a handful of lines means nothing. Data and prose files are left out.

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |

### `km hal` -- Halstead complexity metrics

Computes [Halstead complexity metrics](https://en.wikipedia.org/wiki/Halstead_complexity_measures) per file by extracting operators and operands from source code.

```bash
km hal [path]
```

#### Metrics

| Symbol | Metric | Formula | Description |
|--------|--------|---------|-------------|
| n1 | Distinct operators | -- | Unique operators in the code |
| n2 | Distinct operands | -- | Unique operands in the code |
| N1 | Total operators | -- | Total operator occurrences |
| N2 | Total operands | -- | Total operand occurrences |
| n | Vocabulary | n1 + n2 | Size of the "alphabet" used |
| N | Length | N1 + N2 | Total number of tokens |
| V | Volume | N * log2(n) | Size of the implementation |
| D | Difficulty | (n1/2) * (N2/n2) | Error proneness |
| E | Effort | D * V | Mental effort to develop |
| B | Bugs | V / 3000 | Estimated delivered bugs |
| T | Time | E / 18 seconds | Estimated development time |

Higher effort, volume, and bugs indicate more complex and error-prone code.

In TypeScript, declarations that run nothing (`interface`, `type` aliases, `import type`, `export type { … }`, `declare`) are left out: a file of types alone has no Halstead metrics, so it does not reach `km miv` or the Halstead dimension of `km score`.

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--sort-by METRIC` | Sort by `effort`, `volume`, or `bugs` (default: `effort`) |

Example output:

```
Halstead Complexity Metrics
──────────────────────────────────────────────────────────────────────────────
 File                      n1   n2    N1    N2    Volume     Effort   Bugs
──────────────────────────────────────────────────────────────────────────────
 src/loc/counter.rs       139  116  3130  1169   34367.7   24070888  11.46
 src/main.rs               37   43   520   185    4457.0     354743   1.49
──────────────────────────────────────────────────────────────────────────────
 Total (2 files)                     3650  1354   38824.7   24425631  12.95
```

#### Supported languages

Rust, Python, JavaScript, JSX, TypeScript, Go, C, C++, C#, Java, Objective-C, PHP, Dart, Ruby, Kotlin, Swift, Shell (Bash/Zsh), Elixir, Kaikai, Fortran, Pascal (Delphi).

### `km cycom` -- Cyclomatic complexity

Computes cyclomatic complexity per file and per function by counting decision points (`if`, `for`, `while`, `match`, `&&`, `||`, etc.).

```bash
km cycom [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse,github,codeclimate}` | Output format (default: table). `github` emits GitHub Actions annotations; `codeclimate` (alias: `gitlab`) emits CodeClimate JSON for GitLab Code Quality |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--min-complexity N` | Skip files whose most complex function is below N; with `--per-function`, also hide functions below N (default: 1) |
| `--per-function` | Show per-function breakdown |

#### How functions are found

`cycom`, `cogcom` and `smells` share one function detector. A declaration is recognised by the marker of its language (`fn`, `def`, `func`, `function`, `defmacro`) or, in the C family, by its shape, over the whole signature even when it spans several lines:

- parameters on several lines, the name alone on the line above them (GNU style), the `{` on the next line;
- a line that starts inside parentheses left open (the continuation of a condition, a parameter with an attribute) is never a declaration, nor is an annotation (`@Override`, `[HttpGet]`) or an anonymous class (`new Adapter() {`);
- preprocessor directives and the lines a trailing `\` joins to them are not code: they neither declare functions nor add complexity;
- Go methods are named without their receiver;
- in JavaScript and TypeScript, functions bound to a name (`res.send = function`, `const f = (x) => {`, `name: Type = <T>(x) => {`) take that name, a function passed as an argument is `<anonymous>`, and the braces of a regular expression literal are not blocks;
- in Elixir, the code inside a `quote` block is not part of the function that holds it, and the functions it declares are measured on their own.

### `km cogcom` -- Cognitive complexity

Computes cognitive complexity per file and per function using the [SonarSource method](https://www.sonarsource.com/docs/CognitiveComplexity.pdf) (2017). Unlike cyclomatic complexity, cognitive complexity measures how difficult code is to *understand*, penalizing deeply nested structures and rewarding linear control flow.

```bash
km cogcom [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse,github,codeclimate}` | Output format (default: table). `github` emits GitHub Actions annotations; `codeclimate` (alias: `gitlab`) emits CodeClimate JSON for GitLab Code Quality |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--min-complexity N` | Skip files whose most complex function is below N; with `--per-function`, also hide functions below N (default: 1) |
| `--per-function` | Show per-function breakdown |
| `--sort-by METRIC` | Sort by `total`, `max`, or `avg` (default: `total`) |

In brace languages a function ends where its braces balance. A body introduced by `=` (`fn f(x) = expr`, as in Kaikai, Kotlin or Scala) has no closing brace of its own: it runs while its braces are open and the following lines are indented deeper than the declaration. `km cycom` finds functions the same way.

Keywords and braces are read in order along each line, so code written on one line scores as it does on several: `if (a) { return 1 } else { return 2 }` is 2, and `for (v of xs) { if (y) { return v } }` is 3. The block of a control keyword opens at the next `{`, on its line or, Allman style, on the next one; `else if` nests its block like `if` and `else`.

In Elixir, `with`, `unless` and `receive` are structural increments like `case`, and `and`, `or`, `&&` and `||` count as boolean sequences. For `km cycom`, each clause (`->`) of a `case`, `cond`, `receive` or multi-clause `fn` is a branch, as each `case` label of C, and `with`, `unless`, `and`, `or`, `&&` and `||` add one each.

### `km mi` -- Maintainability Index (Visual Studio variant)

Computes the [Maintainability Index](https://learn.microsoft.com/en-us/visualstudio/code-quality/code-metrics-maintainability-index-range-and-meaning) per file using the Visual Studio formula. MI is normalized to a 0–100 scale with no comment-weight term.

```bash
km mi [path]
```

#### Formula

```
MI = MAX(0, (171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)) * 100 / 171)
```

Where V = Halstead Volume, G = cyclomatic complexity, LOC = code lines.

#### Thresholds

| MI Score | Level | Meaning |
|----------|-------|---------|
| 20–100 | green | Good maintainability |
| 10–19 | yellow | Moderate maintainability |
| 0–9 | red | Low maintainability |

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--sort-by METRIC` | Sort by `mi` (ascending), `volume`, `complexity`, or `loc` (default: `mi`) |

Example output:

```
Maintainability Index (Visual Studio)
──────────────────────────────────────────────────────────────────────
 File                       Volume Cyclo   LOC     MI  Level
──────────────────────────────────────────────────────────────────────
 src/loc/counter.rs        32101.6   115   731    0.0  red
 src/main.rs               11189.6    16   241   17.5  yellow
 src/loc/report.rs          6257.0    13   185   22.2  green
──────────────────────────────────────────────────────────────────────
 Total (3 files)                         1157    7.2
```

The Total row is the MI of the files weighted by their code lines, with any value below 0 taken as 0, as the legacy model of `km score` takes it: a few small files do not outweigh the bulk of the code.

### `km miv` -- Maintainability Index (verifysoft variant)

Computes the [Maintainability Index](https://www.verifysoft.com/en_maintainability.html) per file. MI combines Halstead Volume, Cyclomatic Complexity, lines of code, and comment ratio into a single maintainability score.

This is the verifysoft.com variant, which includes a comment-weight term (MIcw) that rewards well-commented code.

```bash
km miv [path]
```

#### Formula

```
MIwoc = 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)
MIcw  = 50 * sin(sqrt(2.46 * radians(PerCM)))
MI    = MIwoc + MIcw
```

Where V = Halstead Volume, G = cyclomatic complexity, LOC = code lines, PerCM = comment percentage (converted to radians).

The scale has no lower bound, as verifysoft defines it. The index is computed once per file, and both logarithmic terms grow with the file's size, so a long file of trivial functions can fall below zero while a short file with one tangled function stays moderate; the `Total` row averages the files. For a scale clamped to 0–100 use `km mi`; to find tangled functions, `km cogcom`.

#### Thresholds

| MI Score | Level | Meaning |
|----------|-------|---------|
| 85+ | good | Easy to maintain |
| 65–84 | moderate | Reasonable maintainability |
| <65 | difficult | Hard to maintain |

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--sort-by METRIC` | Sort by `mi` (ascending), `volume`, `complexity`, or `loc` (default: `mi`) |

Example output:

```
Maintainability Index
────────────────────────────────────────────────────────────────────────────────
 File                       Volume Cyclo   LOC  Cmt%   MIwoc      MI  Level
────────────────────────────────────────────────────────────────────────────────
 src/loc/counter.rs        32101.6   115   731   3.6   -16.2     2.8  difficult
 src/main.rs                8686.7    14   204  14.6    34.5    68.2  moderate
 src/util.rs                2816.9    18    76   9.5    55.4    84.7  moderate
────────────────────────────────────────────────────────────────────────────────
 Total (3 files)                         1011                  22.2
```

The verifysoft MI has no lower bound: a large file can be far below 0 (ripgrep `defs.rs` is −206), and it weighs size more than complexity. That is the definition of the variant; `km mi` is the one bounded to 0–100. The Total row is weighted by code lines with values below 0 taken as 0, as in `km mi`.

### `km hotspots` -- Hotspot analysis

Finds hotspots: files that change frequently AND have high complexity. Based on Adam Thornhill's method ("Your Code as a Crime Scene").

```bash
km hotspots [path]
```

#### Formula

```
Score = Commits × Complexity
```

Files with high scores concentrate risk — they are both change-prone and complex, making them the highest-value refactoring targets.

By default, complexity is measured by **total indentation** (sum of logical indentation levels across all code lines), following Thornhill's original method from "Your Code as a Crime Scene". Use `--complexity cycom` for cyclomatic complexity instead.

Requires a git repository. Merge commits are excluded from the count.

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--sort-by METRIC` | Sort by `score`, `commits`, or `complexity` (default: `score`) |
| `--since DURATION` | Only consider commits since this time (e.g. `30d`, `6m`, `1y`) |
| `--complexity METRIC` | `indent` (default, Thornhill) or `cycom` (cyclomatic) |

Duration units: `d` (days), `m` (months, approx. 30 days), `y` (years, approx. 365 days).

Example output (default — indentation complexity):

```
Hotspots (Commits × Total Indent Complexity)
──────────────────────────────────────────────────────────────────────────────
 File                    Language Commits Total Indent      Score
──────────────────────────────────────────────────────────────────────────────
 src/main.rs                 Rust      18        613      11034
 src/loc/counter.rs          Rust       7       1490      10430
 src/dups/detector.rs        Rust       7       1288       9016
 src/dups/mod.rs             Rust       9        603       5427
 src/report/mod.rs           Rust       4        998       3992
──────────────────────────────────────────────────────────────────────────────

Score = Commits × Total Indentation (Thornhill method).
High-score files are change-prone and complex — prime refactoring targets.
```

Example output (`--complexity cycom`):

```
Hotspots (Commits × Cyclomatic Complexity)
──────────────────────────────────────────────────────────────────────────────
 File                     Language Commits Cyclomatic      Score
──────────────────────────────────────────────────────────────────────────────
 src/loc/counter.rs           Rust       7        115        805
 src/dups/mod.rs              Rust       9         44        396
 src/main.rs                  Rust      18         21        378
 src/cycom/analyzer.rs        Rust       4         92        368
 src/dups/detector.rs         Rust       7         46        322
──────────────────────────────────────────────────────────────────────────────

Score = Commits × Cyclomatic Complexity.
High-score files are change-prone and complex — prime refactoring targets.
```

### `km knowledge` -- Code ownership analysis

Analyzes code ownership patterns via git blame (knowledge maps). Based on Adam Thornhill's method ("Your Code as a Crime Scene" chapters 8-9).

```bash
km knowledge [path]
```

Identifies bus factor risk and knowledge concentration per file. Generated files (lock files, minified JS, files that say they are generated, see [Generated files, data and prose](#generated-files-data-and-prose)) are automatically excluded.

#### Risk levels

| Risk | Condition | Meaning |
|------|-----------|---------|
| CRITICAL | 1 person owns >80% | High bus factor risk |
| HIGH | 1 person owns 60-80% | Significant concentration |
| MEDIUM | 2-3 people own >80% combined | Moderate concentration |
| LOW | Well-distributed | Healthy ownership |

#### Knowledge loss detection

Use `--since` to define "recent activity". If the primary owner of a file has no commits in that period, the file is flagged with **knowledge loss** risk; when nobody committed in the period, every file is. Use `--risk-only` to show only those files; it needs `--since`. The closing count is of every file at risk, with how many `--top` shows.

Bot accounts (`dependabot[bot]`, `github-actions[bot]`) hold no knowledge: their lines are left out, and a file only bots wrote is not listed. Generated files and data or prose (JSON, YAML, Markdown, text) are left out too.

#### Who wrote each line

Ownership comes from blame of the file as committed at `HEAD`. When the `git` program is installed, kimun runs `git blame -w -M -C`: a change of whitespace keeps the author of the line, and so does code moved within a file or from another file the same commit changed. Revisions listed in `.git-blame-ignore-revs` (or in the file `blame.ignoreRevsFile` names) are skipped, so a mass reformat does not take the lines. Without `git`, libgit2 blames the file: it skips whitespace changes but neither follows moved code nor ignores revisions. `km authors` and `km loc --by-author` read blame the same way.

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--sort-by METRIC` | Sort by `concentration`, `diffusion`, or `risk` (default: `concentration`) |
| `--since DURATION` | Define recent activity window for knowledge loss (e.g. `6m`, `1y`, `30d`) |
| `--risk-only` | Show only files with knowledge loss risk |
| `--summary` | Aggregate by author: files owned, lines, languages, worst risk |
| `--bus-factor` | Show the project bus factor: how many contributors have to leave before most of the code has no author left |
| `--bus-factor-threshold PCT` | Share of the code, in lines, the bus factor counts to (default: 50) |
| `--author NAME` | Show only files owned by this author (case-insensitive substring match) |

Example output:

```
Knowledge Map — Code Ownership
──────────────────────────────────────────────────────────────────────────────
 File                       Language  Lines  Owner         Own%  Contrib  Risk
──────────────────────────────────────────────────────────────────────────────
 src/loc/counter.rs             Rust    731  E. Diaz        94%        2  CRITICAL
 src/main.rs                    Rust    241  E. Diaz        78%        3  HIGH
 src/walk.rs                    Rust    145  E. Diaz        55%        5  MEDIUM
──────────────────────────────────────────────────────────────────────────────

Files with knowledge loss risk (primary owner inactive): 1
  src/legacy.rs (Former Dev)
```

Use `--bus-factor` to compute how many contributors have to leave before most of the code has nobody who wrote it (the *truck factor* of Avelino et al., "A novel approach for estimating truck factors", ICPC 2016):

- The **authors of a file** are those blame gives at least 75% of the lines of its main author. A contributor with a few lines in a file is not one of its authors.
- Contributors leave one at a time, first the one who is an author of the most lines, until more than 50% of the code is in files with no author left. The bus factor is how many left. Files weigh by their lines, so a one-line file does not count as much as the core of the project, and a long tail of occasional contributors does not raise the number.
- With `--since`, authors with no commits in that window have already left: the code only they wrote is orphaned before anyone is counted. When that is already more than the threshold, the bus factor is 0.
- People are told apart by email, read through `.mailmap`, so one address under several names, or several addresses the mailmap joins, is one person. This also holds for `--summary`, `km authors` and `km loc --by-author`.

```
$ km knowledge --bus-factor --since 1y
Project Bus Factor: 2

 12% of the code is already in files whose authors have not committed in 1y.
 If these 2 contributors leave, 71% of the code is in files with no author left.
 Risk: HIGH — two people hold critical knowledge
 Authors of a file hold at least 75% of the lines of its main author; files weigh by lines; threshold 50%.

──────────────────────────────────────────────────────────────────────
 Rank  Author                Files     Lines    Share  Orphaned
──────────────────────────────────────────────────────────────────────
    1  E. Diaz                  41      8420   68.12%     48.3%
    2  A. Torres                 9      1490   12.06%     71.0%
    3  R. Soto                   6       940    7.61%
    4  M. Rojas (inactive)       3       610    4.94%
──────────────────────────────────────────────────────────────────────
```

`Orphaned` is the share of the code left with no author once that contributor and those above have left.

### `km tc` -- Temporal coupling analysis

Analyzes temporal coupling between files via git history. Based on Adam Thornhill's method ("Your Code as a Crime Scene" ch. 7): files that frequently change together in the same commits have implicit coupling, even without direct imports.

```bash
km tc [path]
```

#### Formula

```
Coupling strength = shared_commits / min(commits_a, commits_b)
```

#### Coupling levels

| Strength | Level | Meaning |
|----------|-------|---------|
| >= 0.5 | STRONG | Files change together most of the time |
| 0.3-0.5 | MODERATE | Noticeable co-change pattern |
| < 0.3 | WEAK | Occasional co-changes |

High coupling between unrelated modules suggests hidden dependencies or architectural issues — consider extracting shared abstractions.

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--top N` | Rows of the table (default: 20); JSON lists every pair unless `--top` is given |
| `--sort-by METRIC` | Sort by `strength` or `shared` (default: `strength`) |
| `--since DURATION` | Only consider commits since this time (e.g. `6m`, `1y`, `30d`) |
| `--min-degree N` | Minimum commits per file to be included (default: 3) |
| `--min-shared N` | Minimum commits two files must share to be a pair (default: 3) |
| `--max-changeset N` | Ignore commits touching more than N files as evidence of coupling (default: 30) |
| `--min-strength F` | Minimum coupling strength to show (e.g. `0.5` for strong only) |
| `--include-ext`, `--exclude-ext`, `--exclude-dir`, `--exclude` | Path filters, applied to the paths of history |

Only files that still exist in the working tree are paired, and only code: lock files, manifests, CI YAML and documentation are left out. A sweeping commit (more than `--max-changeset` files) relates files by accident and is skipped, as is a revert together with the commit it reverts (a message opening with `Revert "` that names the commit): together they changed nothing. Renames are not followed: a file is paired under the path it has now.

Example output:

```
Temporal Coupling — Files That Change Together
──────────────────────────────────────────────────────────────────────────────────
 File A                     File B                     Shared  Strength  Level
──────────────────────────────────────────────────────────────────────────────────
 src/auth/jwt.rs            src/auth/middleware.rs          12      0.86  STRONG
 lib/parser.rs              lib/validator.rs                 8      0.53  STRONG
 config/db.yaml             config/cache.yaml                6      0.35  MODERATE
──────────────────────────────────────────────────────────────────────────────────

12 coupled pairs found (3 shown). Showing pairs with >= 3 shared commits.
Strong coupling (>= 0.5) suggests hidden dependencies — consider extracting shared abstractions.
```

**Note:** File renames are not tracked across git history. Renamed files appear as separate entries.

### `km impact` -- Impact of a diff

Measures how far a change reaches, for a PR or for uncommitted work, before it is merged.

#### What the blast radius is

The **blast radius** of a change is the part of the system that can behave differently because of it, beyond the files it edits. A change to a function is not only that function: it is every piece of code that calls it, and what calls those. If the function's own tests pass and something that uses it breaks in production, the break was inside the radius and outside the tests.

`km impact` measures it at two levels, and reports what guards it:

| Level | The radius is | Read from |
|-------|---------------|-----------|
| Projects | The projects of the repository that depend on the changed ones, directly or through others | Manifests |
| Source files | The files that call the functions that changed, and the files that use those | The dependency graph of the code |

At each level the radius is a count over a total: `3 of 6 projects`, `2 of 618 source files`. It answers **how much of the system to worry about**. Next to it comes **what is unprotected**: the files inside the radius that no test exercises. A wide radius fully covered by tests is a change to make with care; one file in the radius with no test is where it will break without warning, and the report names it.

Two more measures describe the change itself rather than its reach: how spread it is (**diffusion**), and which files usually change with it and were left out (**logical radius**).


```bash
km impact --since-ref origin/main [path]         # the branch you are on
km impact --since-ref main --until-ref feature   # a branch, without checking it out
git diff main... | km impact --diff -               # a patch
km impact --pr 123                               # a GitHub pull request
```

#### What is measured

| Source | The change | History ends at | Manifests and files read from |
|--------|------------|-----------------|-------------------------------|
| `--since-ref REF` | From where `REF` and `HEAD` diverged to the working tree: committed, uncommitted and untracked changes, and deletions | That merge base | The working tree |
| `--since-ref A --until-ref B` | What `B` brings since it diverged from `A`, whatever is checked out | That merge base | The tree of `B` |
| `--diff FILE` | A patch in git format, from a file or from stdin (`-`) | `HEAD`, or where `--since-ref` and `HEAD` diverged when given | The working tree |
| `--pr NUMBER` | A GitHub pull request | As the mode it resolves to | As the mode it resolves to |

It is always the change over the whole repository: `path` only locates the repository and does not narrow the analysis. Generated files (lock files, minified assets) are left out of every measure.

**`--pr` requires the [GitHub CLI](https://cli.github.com) (`gh`) installed and authenticated.** kimun runs it as a program; it links no GitHub client and stores no token.

- When the repository has the commits of the pull request, it is measured from them, as between two refs. A pull request merged by squash or rebase, whose head was never fetched, is measured from the patch GitHub serves, read against the tree of the commit that merged it. Its base and that commit are not compared: the base may be many merges behind, and everything merged in between would be counted.
- Otherwise (a pull request from a fork, or one not fetched) its patch is taken from `gh pr diff` and measured like any other patch, with a note on stderr. `git fetch origin pull/NUMBER/head` makes its commits available.

A patch (`--diff`, or a pull request without local commits) is measured against the working tree, where it is not applied:

- it must be in git format with the `a/` and `b/` prefixes, as `git diff`, `git format-patch` and `gh pr diff` print it, without colors;
- for a branch use `git diff main...` (three dots): `git diff main` compares against the tip of `main`, and shows what `main` gained since as if the branch had undone it. To include uncommitted work, `--since-ref main` is the direct way;
- a project the patch creates is not known, so its files have unknown reach, and `--affected` lists every project;
- if the patch is already applied in `HEAD`, pass `--since-ref` so that its own commits are not counted as history.

#### Blast radius: projects

Which projects of the repository are reached by the diff. Meant for monorepos, where a change to a shared library reaches applications its author may not know.

A **project** is a directory with a manifest. A project **depends** on another when its manifest names it as a local dependency; dependencies on registries or other repositories are ignored. A changed file belongs to the nearest project above it. The radius is every project that depends on a changed one, directly or through others.

| Ecosystem | Manifest | Local dependencies read |
|-----------|----------|-------------------------|
| Rust | `Cargo.toml` | `path` dependencies, `workspace = true` resolved through `[workspace.dependencies]`, in `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` and their `[target.*]` forms |
| JavaScript / TypeScript | `package.json` | any dependency whose name is another package of the repository (npm, yarn and pnpm workspaces), plus `file:` and `link:` |
| Elixir | `mix.exs` | `path:` dependencies and `in_umbrella: true` |
| Go | `go.mod`, `go.work` | required modules that are another module of the repository, and `replace` with a directory |
| Python | `pyproject.toml` | any requirement whose name is another project of the repository (whatever the case, and whichever of `-`, `_`, `.`), and those given a `path` in `[tool.uv.sources]` or in the tables of Poetry; read from `[project]` (`dependencies`, `optional-dependencies`), `[dependency-groups]`, `[build-system] requires`, `[tool.poetry]` and the development groups of uv and PDM |

```
Blast radius — projects reached through their manifests
──────────────────────────────────────────────────────────────────────────────
 3 of 6 projects reached (50%), 2 direct
 Changed: libs/core

 Changed    Reaches         Distance  Scope  Via
 libs/core  apps/inventory         1
 libs/core  libs/locker            1
 libs/core  apps/parcels           2         libs/locker
──────────────────────────────────────────────────────────────────────────────
Changed files outside every project (reach unknown): Makefile
```

- **Scope**: a `dev` dependency (dev or test only) reaches the dependent, whose tests use the changed project, and stops there: the changed project is not part of what the dependent ships, so the dependents of the dependent are not reached. `build` and `optional` dependencies carry on like runtime ones.
- **Workspace roots**: a `Cargo.toml` with `[workspace]`, a `package.json` with `workspaces` (or next to a `pnpm-workspace.yaml`), an umbrella `mix.exs` with `apps_path`, a `go.work`, a `pyproject.toml` with `[tool.uv.workspace]`. A change to the manifest or the lock file at such a root (`Cargo.lock`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `bun.lock`, `mix.lock`, `uv.lock`) reaches every project under it at distance 1, with scope `workspace`, and carries on from them. A workspace root is a project itself only when it declares one (`[package]` in Cargo, `app:` in mix, `[project]` in Python); a `package.json` at a workspace root never is.
- **Inert files** reach nothing: a change to documentation (`.md`, `.mdx`, `.rst`, `.adoc`, `.txt`) breaks no build and no test. It does not count as a change to its project, nor as a file of unknown reach. `.kimun.toml` can declare more:

  ```toml
  [impact]
  inert = ["scripts/**", "notebooks/**"]   # globs, relative to the repository
  ```

- **Files outside every project** (CI workflows, shared configuration) are listed apart. Their reach is unknown, not zero.

- **`--affected`** prints the projects whose builds and tests the diff calls for, one per line, and nothing else: the changed and the reached ones. If any changed file is outside every project, it prints **all** projects and says why on stderr — skipping a test suite is worse than running one too many. It reads only the diff and the manifests, not the history.
- A repository with a single project gets a line saying this level does not apply, rather than "0 reached".
- `node_modules`, `vendor` and `testdata` are never searched for manifests, nor are `deps`, `_build` and `target` (unless they sit under `src`, `lib` or `app`, where they are part of the project), nor is a `fixtures` directory inside a test directory. An end-to-end suite with its own manifest (`test/e2e/package.json`) is a project.

Limits:

- `mix.exs` is code and is read as text. A dependency whose path is built at run time (`Path.expand(...)`, string interpolation, a generated list) cannot be attributed; the report names the manifest and how many it missed.
- Of Python only `pyproject.toml` is read. A project declared in `setup.py` (which is code) or `setup.cfg` is not seen, nor is a local dependency written in `requirements.txt` (`-e ../lib`). A `pyproject.toml` that only configures tools, with neither `[project]` nor `[tool.poetry]`, is not a project. Extras (`optional-dependencies`) are `optional`; dependency groups are `dev`.
- Coupling across ecosystems is not visible: a web client and the service whose API it calls have no manifest dependency between them.
- A project nested in another (`assets/package.json` inside a Phoenix application) has no dependency to or from the one that contains it unless a manifest declares one.
- The graph is read from the working tree. The files of a project the diff deletes or moves away belong to no project any more: their reach is unknown, and the manifests still naming it are reported as not read.

#### Blast radius: source files

Inside the projects the change affects: which source files use what changed, and which of them no test exercises. This is the question "the tests of the module I changed pass; who else calls it?".

The first line is the answer in short: how many files call what changed, and how many of them have no test. `Radius` is the count of files that call what changed, and of those that use them. `Upper bound` is what it would be without knowing which functions changed.

```
Structural radius — source files that use what changed
──────────────────────────────────────────────────────────────────────────────
 1 file calls what changed, 1 of them with no test
 Changed: lib/booking/insights.ex
 Functions: arrange
 Radius: 1 of 6 source files (17%): 1 at distance 1
 Not in the radius: 1 that refer to the module without calling it
 Upper bound, whatever the function: 4 files (67%)

 Tests  Dependent
  none  lib/booking_web/controllers/insight_controller.ex
            calls Insights.arrange
  none  lib/booking/export.ex
            refers to the module without calling it
──────────────────────────────────────────────────────────────────────────────
No test reaches 1 of the files that call what changed; an integration test is probably missing:
  lib/booking_web/controllers/insight_controller.ex
No test in the change exercises a file that uses what changed.
1 more with no test use the module without a call that tells whether the change concerns them.
```

How it is measured:

- The dependency graph of `km deps` is read backwards from the changed source files. Every import counts for the radius, `type` and `deferred` ones included: a change to a type breaks those that check against it, and a deferred import runs. A test that imports a file only for its types runs none of it, so it does not protect it.
- In Elixir the change is **narrowed to functions**: the lines the diff touches tell which functions changed, and a change to a private function is carried to the public ones that reach it through local calls. A file that uses the module is then one of three: it **calls** a function that changed, it **refers** to the module without calling it (a struct, an `import`, a `use` with nothing to read), or it only calls functions the change leaves alone, and is not listed. A file that imports the whole module may reach a function through a call the reading does not see, so it refers to the module even when the calls seen are left alone. Each changed file is narrowed on its own. Outside the functions, a touched `alias` or `require` changes none (it only names what the touched functions use), and a touched module attribute changes the functions that read it. A `use`, an `import`, a `defstruct`, or an attribute no function reads may concern every function: that file is not narrowed, every use of it counts, and the report says which line it was. A new file is never narrowed.
- In Python the change is narrowed to the **top-level names** of the module: its functions, classes and assignments. A touched method changes its class; a decorator belongs to what it decorates; a name that uses a changed one changes too, private or not, since nothing keeps another file from importing it. A file that uses the module calls what changed when it takes one of those names: `from m import name`, `m.name` on a module it imports (under its alias too), or, after `from m import *`, a name it goes on to mention. A module handed over as a value may give any of its names. One imported with nothing read on it only refers to the module. Imports, the docstring, an `if` or a `try` that only chooses imports, and the `if __name__ == "__main__":` block change no name. Any other code at the left margin runs when the module is imported and may concern every name: the file is not narrowed. Not seen: names reached at run time (`getattr`, a registry filled by decorators, `mock.patch("m.name")`), and which callers use the method that changed. A name a package re-exports is followed through its `__init__.py`, one distance further.
- The **radius** starts at the files that call what changed and follows who uses them, file by file. A file that only refers to the module, without a call that tells, is listed but does not carry the radius on: a schema is named by half a project, and following all of that says nothing. Files that only pass through a dependent the change leaves alone are not counted either. Past the first step the radius is still by file, not by function, so it stays an estimate from above. The **upper bound** is what the radius would be if every use of a changed file counted, whatever the function: in a codebase where everything goes through a few contexts it is most of the project, which is why the radius is the number to read.
- **Tests** says how the dependent is protected. A test rarely names everything it exercises — a controller or a live view is tested through its route, a helper through the views that use it — so protection comes in degrees:

  | `Tests` | Protection | When |
  |---------|------------|------|
  | a number | direct | That many test files refer to it, request a route it serves, or sit at the same place in the source and test layout (`lib/a/b.ex` and `test/a/b_test.exs`) |
  | `named` | named | A test carries its name a directory apart (`live/page_live.ex` and `page_live_test.exs`), or the name of the directory it is in (`page_live/index.ex` and `page_live_test.exs`). A test at the place of a source file is the test of that file and names no other: `app_test.exs` beside `app.ex` says nothing of the files in `app/` |
  | `users` | users | It has no test of its own, but a file that uses it has one |
  | `none` | none | No test reaches it |

  Test support (`test/support/`), configuration and scripts are not tests: a factory refers to everything and would make everything look protected.
- A file that calls what changed and that no test reaches is **unprotected**: the change can break it without any test noticing. That is the warning. Dependents are listed from the least protected.
- **Entry points** are told apart. A command-line task or a script is run, not used: no test of something else passes through it, and few have one of their own. With no test they get a line of their own instead of the warning. They are the files under `mix/tasks/`, `management/commands/`, `bin/` and `scripts/` that no source file uses, plus what `.kimun.toml` declares:

  ```toml
  [impact]
  entry_points = ["**/endpoint.ex"]   # globs, relative to the repository
  ```

- In Elixir, a test that requests a path protects the module that serves it. The routes are read from the Phoenix router of the project of the test, with the path and the alias of every `scope` around them; `:id` matches any one segment, and a segment the test writes at run time (`#{order.id}`) matches only a parameter of the route: a request for one order does not protect `/orders/new`.
- In Elixir, what a framework relates by convention counts as a use. A Phoenix controller uses the views named after it (`PageController` and `PageJSON`, `PageHTML`, `PageView`), so the test of the controller protects them. A module uses the components its templates render, whether written in it as `~H` or kept in a `.html.heex` file beside it (`page/index.ex` and `page/index.html.heex`) or in a directory named after it (`page_html.ex` and `page_html/home.html.heex`).


It is measured for the languages whose graph reflects usage: C, C++, C#, Elixir, Go, Java, JavaScript/TypeScript, Kaikai, Python, Ruby and Rust. In Go the unit is the package: a change to any file of one reaches every file that imports it. In C and C++ a change to a source reaches the files that include its header. Only the projects the change affects are read; the whole repository when the reach over projects is unknown.

Limits: a test at the same place, or named after a file, may not exercise the call that changed, and one that refers to a file may mock what it calls — the column says a test exists, not that it covers. `users` is weaker still: it says something that uses the file is tested. Function names are compared without arity. Modules named at run time (`apply/3`, configuration) or generated by macros are not seen. A request is read when its path is written in the call (`live(conn, ~p"/orders")`), not when it comes from a variable or a helper, and a route when it is declared on one line.

#### Diffusion

How spread the change is.
 Kamei et al. found diffusion among the strongest predictors of a defect-inducing change.

| Measure | Meaning |
|---------|---------|
| Files changed | Files added, modified, renamed or deleted |
| Directories | Distinct directories holding a changed file |
| Subsystems | Distinct top-level directories (files at the root form one more) |
| Lines added / deleted | Binary files count no lines |
| Entropy | Shannon entropy of the modified lines over the files, divided by its maximum: `0` when one file holds every modified line, `1` when all the files hold the same amount |

#### Logical radius

Files that usually change with the files of the diff and are **not** in it — a change that may have been forgotten. It catches coupling the code does not declare: tests, configuration, migrations.

```
Confidence = shared_commits / commits of the changed file
```

A file is reported when some changed file reaches `--min-confidence` with at least `--min-shared` shared commits. Confidence is directional, unlike the strength of `km tc`: a file that changed three times, always with one that changed a hundred times, has strength 1.0 but is needed in 3% of the changes to the other.

- History ends at the merge base: the commits of the diff are never evidence for themselves.
- Commits touching more than `--max-changeset` files are ignored, and the report says how many: a reformat or a rename across the project relates its files to each other by accident.
- A changed file with no history (new, or outside `--since`) predicts nothing. It is listed apart, so its silence is not read as "no impact".
- Files that no longer exist are not reported.
- Test files are always part of the analysis: a test that usually changes with the code is a change worth expecting.

Options:

| Flag | Description |
|------|-------------|
| `--since-ref REF` | Git ref to diff against, e.g. `origin/main`, `HEAD`. Required unless `--diff` or `--pr` is given |
| `--until-ref REF` | Measure up to this ref instead of the working tree (needs `--since-ref`) |
| `--diff FILE` | Measure a patch in git format; `-` reads stdin |
| `--pr NUMBER` | Measure a GitHub pull request; requires `gh` installed and authenticated |
| `--since DURATION` | Only consider history since this time (e.g. `6m`, `1y`, `30d`) |
| `--min-confidence F` | Minimum confidence to report a missing file (default: `0.5`) |
| `--min-shared N` | Minimum shared commits to report a missing file (default: `3`) |
| `--max-changeset N` | Ignore commits touching more than N files as evidence (default: `30`) |
| `--affected` | Print only the changed and reached projects, one per line |
| `--top N` | Show only the top N missing files (default: 20) |
| `--format {table,json,short,terse}` | Output format (default: table) |

Example output:

```
Change Impact — diff against main

Blast radius — projects reached through their manifests
──────────────────────────────────────────────────────────────────────────────
 Single project (.): no other project to reach; this level does not apply.
──────────────────────────────────────────────────────────────────────────────

Diffusion
  Files changed           5
  Directories             3
  Subsystems              2
  Lines added           120
  Lines deleted          30
  Entropy              0.82  (0 = one file holds the change, 1 = evenly spread)

Logical radius — files that usually change with this diff and are not in it
──────────────────────────────────────────────────────────────────────────────
 Missing file      Confidence   Shared  Changes with
──────────────────────────────────────────────────────────────────────────────
 src/tc/report.rs        0.80     8/10  src/tc/mod.rs (+1 more)
 README.md               0.50     6/12  src/cli.rs
──────────────────────────────────────────────────────────────────────────────
No history before the diff (new or never committed): src/impact/mod.rs
Generated files ignored: 1
```

When the rows are too wide for a table (long paths), each missing file is listed on a line of its own with its evidence below.

#### JSON output, for tools and LLMs

`--format json` carries everything the table shows, and the lists the table cuts short. An agent reviewing a change can read it in this order:

```bash
km impact --since-ref origin/main --format json
```

| Field | Meaning |
|-------|---------|
| `source` | The change measured: `diff against main`, `PR #12`, `patch from stdin` |
| `structural.functions` | Functions the change affects (public ones in Elixir, top-level names in Python), over the files that tell them; `null` when none could be narrowed |
| `structural.narrowing[]` | Per changed file: `functions`, or `null` with the `reason` every use of it counts |
| `structural.direct[]` | Every file that uses a changed file: `file`, `exposure` (`calls`, `refers`, `elsewhere`), `protection` (`direct`, `named`, `users`, `none`), `calls`, `tests`, `tests_in_diff`, `entry_point` |
| `structural.unprotected[]` | Files that call what changed and that no test reaches, not even through what uses them — where an integration test is missing |
| `structural.untested_entry_points[]` | Command-line tasks and scripts that call what changed and have no test; not counted as unprotected |
| `structural.unknown_without_tests[]` | Files that use the changed module without a call that tells, and have no test |
| `structural.change_tests_a_dependent` | Whether a test in the change protects a file that uses what changed |
| `structural.radius` | `files`, `source_files` and `share` (0 to 1): the radius as a number |
| `structural.reach[]` | The files of the radius, by `distance` |
| `structural.upper_bound` | Files reached if every use of a changed file counted, whatever the function |
| `projects.changed[]`, `projects.reached[]` | Projects holding a changed file, and those reached, each with `origin`, `distance`, `via`, `scope` |
| `projects.affected[]` | Projects whose builds and tests the change calls for (what `--affected` prints) |
| `projects.outside[]` | Changed files that belong to no project: their reach is unknown |
| `projects.inert[]` | Changed files that reach nothing: documentation, and what `.kimun.toml` declares inert |
| `diffusion` | Files, directories, subsystems, lines and entropy of the change |
| `logical_radius.missing[]` | Files that usually change with the change and are not in it, with every trigger |

`km ai` exposes the command to an LLM as the tool `km_impact`, and the skill installed by `km ai skill` documents it.

`Shared` reads as shared commits over the commits of the changed file.
 `(+1 more)` means another changed file predicts the same missing file; `--format json` lists every one. `--format terse` prints the number of missing files.

**Note:** File renames are not tracked across git history. A file renamed in the diff itself keeps the history of its old path.

### `km churn` -- Code churn analysis

Measures pure change frequency per file from git history (commit count only, no complexity weight). Identifies the most frequently modified files — high churn without a corresponding quality improvement is a maintenance signal.

The rate is commits per month from the file's first commit to now, or from the start of the `--since` window to now, at least one month. A burst of commits long ago is spread over the time since, so it does not outrank a file that keeps changing. HIGH is above 4 a month, MEDIUM from 1, LOW below.

```bash
km churn [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--sort-by METRIC` | Sort by `commits` (default), `rate` (commits/month), or `file` |
| `--since DURATION` | Only consider commits since this time (e.g. `6m`, `1y`, `30d`) |
| `--format {table,json,short,terse}` | Output format (default: table) |

Example output:

```
Code Churn — Change Frequency
──────────────────────────────────────────────────────────────────────────────
 File                     Language  Commits   Rate/mo   First Seen   Last Seen
──────────────────────────────────────────────────────────────────────────────
 src/main.rs                  Rust       18      3.2    2025-01-10  2026-03-28
 src/loc/counter.rs           Rust        7      1.3    2025-01-10  2026-02-14
 src/dups/detector.rs         Rust        7      1.2    2025-02-01  2026-02-20
──────────────────────────────────────────────────────────────────────────────
```

### `km smells` -- Code smell detection

Detects common code quality issues per file using text-based heuristics (no AST required). Only languages with complexity marker support are analyzed (same set as `km cycom`: Rust, Python, JS/TS, C/C++, Go, etc.).

```bash
km smells [path]
```

#### Smell types

| Smell | Description |
|-------|-------------|
| `long_function` | Function body exceeds `--max-lines` (default: 50) |
| `long_params` | Function has more than `--max-params` parameters (default: 4) |
| `todo_debt` | TODO, FIXME, HACK, XXX, or BUG in comment lines |
| `magic_number` | Bare numeric literals in code, other than 0, 1, 2, -1 and values that already have a name |
| `commented_code` | Two or more consecutive comment lines containing code-like patterns, outside documentation |

- **`long_params`** counts the parameters of the list that follows the function's name (a Go receiver, or the call a function expression is passed to, is not it), separated by its top-level commas: commas inside type arguments (`Node<K, V>`), patterns (`%{...}`, `<<...>>`), literals and nested calls separate nothing.
- **`magic_number`** leaves out declarations, with any modifiers before the keyword (`private static final`, `public const`), assignments to upper-case names (`MAX_RETRIES = 5`), and the data a declaration opens: the members of an `enum`, a Go `const (...)` block, the elements of a table, the alternatives of a `type` union. A literal that is the value of a key (`connect: 3`, `"MAX": 4093`) or a parameter default is named too. The body of a function declared with such a keyword (`export function`, `static int f() {`) is still read.
- **`commented_code`** leaves out documentation, where code is an example: `///` and `//!` lines, `/** */` and `/*! */` blocks (JSDoc, Javadoc, Rust doctests, C# XML doc), and a Go doc comment with a tab-indented example right above a declaration.

Options:

| Flag | Description |
|------|-------------|
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--max-lines N` | Maximum function body lines before flagging (default: 50) |
| `--max-params N` | Maximum parameter count before flagging (default: 4) |
| `--files FILE` | Analyze only these specific files (repeatable). Useful for scripting |
| `--since-ref REF` | Analyze only files changed since this git ref (e.g. `origin/main`, `HEAD~1`). Ideal for CI |
| `--format {table,json,short,terse,github,codeclimate}` | Output format (default: table). `github` emits GitHub Actions annotations; `codeclimate` (alias: `gitlab`) emits CodeClimate JSON for GitLab Code Quality |

The table breaks each file's smell count down by type, with one column per smell kind (`magic`, `long`, `param`, `todo`, `comm`) and a per-column total in the footer.

Example output:

```
Code Smells
──────────────────────────────────────────────────────────────────────
 File                            Total  magic  long  param  todo  comm
──────────────────────────────────────────────────────────────────────
 src/loc/counter.rs                 12      7     1      0     4     0
 src/main.rs                         6      2     0      0     4     0
 src/dups/detector.rs                3      0     2      1     0     0
──────────────────────────────────────────────────────────────────────
 Total (3 files)                    21      9     3      1     8     0
```

### `km deps` -- Dependency graph analysis

Analyzes internal module dependencies by parsing import/use/require statements. Builds a directed graph of file-level coupling and detects cycles using Tarjan's SCC algorithm.

```bash
km deps [path]
```

Supports Elixir (every module referred to in the code, with `alias` undone; see the notes below), Rust (every path that names a module of the workspace: `use`, `crate::`, `super::`, a child module; see the notes below), Python (`import` and `from … import`, absolute and relative; see the notes below), JavaScript/TypeScript (`import`, `export … from`, `require` and `import()`, relative or through the aliases of `tsconfig.json` and the packages of the workspace; see the notes below), Go (imports of the packages of the repository, by the module path of each `go.mod`; see the notes below), C and C++ (`#include`, as a compiler finds it; see the notes below), Java (the classes a file names, through its package and its imports; see the notes below), C# (the types a file names, through its namespaces and its `using` directives; see the notes below), Ruby (`require_relative`, `require` against the load path, and the constants a file names, as Rails autoloads them; see the notes below), and Kaikai (`import a.b.c`, including the `as` and `.{…}` forms). External dependencies (crates, npm packages, system headers, the Kaikai stdlib) are ignored.

Files in any other language are left out of the graph instead of being listed with zero dependencies. The table footer, the `unsupported` array of the JSON output and the `unsupported:N` field of the short format say how many files were skipped, so "not measured" is never shown as "no dependencies".

Some imports do not load a module when the importer loads: a TypeScript `import type` (and `import { type A }` when every name is a type, and `export type … from`) and a Python import under `if TYPE_CHECKING:` take only types, and a Python import inside a function or a dynamic `import()` runs later. They stay in the graph, marked `type` or `deferred`, and count in fan-in and fan-out, since they are still coupling, but they are left out of cycle detection: a cycle is a set of files that need each other to load. The table footer counts them (`Imports left out of cycles: …`), and the JSON gives `type_fan_out` and `deferred_fan_out` per file, which are part of `fan_out`, and `type_edges` and `deferred_edges` in total.

An import that has the form of a project file but resolves to none is reported, so that a missing edge is not read as no dependency: a quoted `#include` that is no header of the system (often a file generated at build time), a relative JavaScript or TypeScript specifier of a script, a relative Python import, a Go import within a module of the repository. External packages are never listed. The table footer counts them (`Unresolved internal imports: N in M files`), the JSON lists them per file in `unresolved` and counts them in `unresolved_count`, and the short format gives `unresolved:N`. Rust, Elixir, Java, C# and Kaikai name modules and types rather than files, where nothing tells an internal name from an external one, so they report none.

The JSON lists the edges in `edges`, each as `{"from", "to", "kind"}` with `kind` one of `load`, `compile`, `deferred` or `type`: those that leave the files listed, so `--top` and `--cycles-only` narrow them as they narrow `files`.

Elixir notes:

- Dependencies are between modules, and most need no import, so every module name in the code counts as a reference: calls, structs, `use`, `import`, `require`, `@behaviour`, `defimpl`. Comments, strings, heredocs and sigils are set aside first, so a doctest is not a dependency.
- `alias` is undone, including `as:`, grouped and multi-line forms, and `__MODULE__`. The statement itself is not a use.
- Nested `defmodule`s are named after the module that contains them, told by indentation as `mix format` leaves it.
- A module defined in several files (projects made from one template) resolves to the file nearest the one referring to it.
- Comments and literals hold no references, but the code a string interpolates does: `"Total: #{Orders.total(order)}"` uses `Orders`.
- A Phoenix controller uses the views named after it (`PageController` and `PageJSON`, `PageHTML`, `PageView`), and a module uses the components its `~H` templates render.
- `mix.exs`, `.formatter.exs` and the scripts of a `config` directory configure the build: they stay in the graph with no dependencies, and the modules they name (`groups_for_modules`, `config :app, App.Endpoint`) or define (`App.MixProject`) make no edges.
- `use Foo, :name` runs `Foo.name` and `use Foo` runs `Foo.__using__`. When `Foo` is a module of the project, the code of that function, and of the functions it splices in with `unquote(helper())`, is read as written in the user: its aliases, imports and references are the user's. A live view that does `use MyAppWeb, :live_view` uses `MyAppWeb.CoreComponents` and the `Layouts` it names, as the compiler sees it. The quote is read whole, whichever branch the macro takes.
- `import Foo, only: [bar: 1]` calls `Foo.bar`. Without `only:`, a call with no module (`bar(x)`, `&bar/1`, `<.bar>` in a template) that the file does not define calls the function of that name of each module of the project it imports.
- A use the file needs to compile is marked `compile`: a `use`, and a call to a macro (`defmacro`, `defguard`) of the project, as the compiler expands it into the caller. What a `quote` block holds is compiled where the macro is called, so it makes no `compile` use of the file that writes it; what a `use` injects is compiled in the user. An `import` or `require` alone recompiles the file only when the exports of the module change (`mix xref` calls that `export`), so it is a plain use. A `compile` use counts in cycles; the table footer counts them (`Compile-time uses …`), and the JSON gives `compile_fan_out` per file and `compile_edges` in total. A function called in the body of a module, outside any function (`plug Plug.Static, only: MyAppWeb.static_paths()`), also runs at compile time, and is not told.
- Not seen: modules named at run time (`apply/3`, configuration), those generated by macros, and those a Phoenix router names under the alias of a `scope`. Functions are told by name, not by arity.

Rust notes:

- A dependency is a path: `use crate::git::GitRepo`, a qualified `super::analyzer::run(x)`, a call on a child module (`report::print(x)`). Groups, `as`, `self`, globs and multi-line `use` are read. `mod x;` only says where a module lives and adds no edge.
- Each crate is a tree of modules grown from its root (`src/lib.rs`, `src/main.rs`, a file of `src/bin`, `tests`, `examples` or `benches`, `build.rs`) by its `mod` declarations, `#[path]` included. A path leads to the file of the deepest module it names: the item may be defined there or only re-exported. A module declared twice under opposite `#[cfg]`s, with a `#[path]` each, leads to both files.
- A crate re-exported under a name (`pub extern crate grep_printer as printer`, `pub use grep_printer as printer`) is followed: `grep::printer::Stats` leads to the library of `grep-printer`, and the facade uses it.
- Other targets of a package reach its library by name (`my_app::orders` from `tests/` or `main.rs`), and so do the other crates of the workspace. The name comes from `[package] name`, with `-` read as `_`.
- A type uses the files that hold its `impl` blocks, since what they define is reached through the type.
- For `km impact`, a file that holds `#[test]` functions has a test of its own; a file of `tests/` that holds them, or a `tests.rs` module, is a test. What a package runs (`src/main.rs`, `src/bin`, `examples`, `benches`, `build.rs`) is an entry point.
- Comments, strings and `$crate` in a macro name nothing.
- Not seen: what a macro generates or names, `include!`, a library with a `[lib] path` or `name` of its own, a dependency renamed in `Cargo.toml`, and modules behind `cfg`, which all count. A use only from an inline test module still makes the file a dependent. A test that runs the binary (`assert_cmd`, `CARGO_BIN_EXE_*`) names no file, so it protects none. Two packages of the same name in one repository are not told apart.

Python notes:

- Every `import a.b` and `from a.b import c` is read, wherever it is written: in a function, under `if TYPE_CHECKING:`, over several lines in parentheses or with a backslash. One written in a docstring or a comment is not an import. Where it is written is read by indentation: under `if TYPE_CHECKING:` (or `if typing.TYPE_CHECKING:`) it is `type`, in a function (`def`, `async def`, a method) it is `deferred`, anywhere else it loads, a class body and the `else:` of `if TYPE_CHECKING:` included. Neither of the first two makes a cycle.
- A dotted path names `a/b.py` or the package `a/b/__init__.py`; a stub (`.pyi`) stands for a module with no source. Only the deepest module is used: `import a.b.c` adds no edge to `a/__init__.py`.
- In `from a.b import c`, `c` is the submodule `a/b/c.py` when that file exists, and a name `a.b` defines otherwise.
- A relative import starts at the package of the importing file, one level up per extra dot.
- An absolute import is looked up from each directory above the importing file that is not itself a package (it holds no `__init__.py`), nearest first, and from the `src` directory under each. That covers a project run from its root, a `src` layout with `tests/` beside it, and several projects in one repository. Then come the roots the nearest `pyproject.toml` above declares: where the build backend finds the packages (`where` and `package-dir` of setuptools, `packages` of Poetry, `sources` and `packages` of Hatch, `python-source` of maturin, `package-dir` of PDM, `module-root` of uv) and what pytest adds (`pythonpath`). What is found under none of them is external: the standard library, installed packages.
- For `km impact`, `test_*.py`, `*_test.py` and `tests.py` are tests, and `conftest.py` is test support, which protects nothing; `__main__.py`, `setup.py` and `manage.py` are entry points, and so is what sits under `management/commands`.
- Not seen: modules named at run time (`importlib`, `__import__`, Django settings and `INSTALLED_APPS`), roots added at run time (`sys.path`, `PYTHONPATH`, a `.pth` file) or declared in `setup.py`, `setup.cfg` or `pytest.ini`, and a namespace package spread over several roots. A name a package re-exports leads to its `__init__.py`, and from there to the module that defines it.

Go notes:

- An import names a package, and a package is a directory: the import depends on every `.go` file directly inside it, not on the ones in its subdirectories, which are other packages. Which file holds what was used is not read, so fan-out counts files: one import of a package of 17 files adds 17.
- Imports are read from the code: one written in a comment or a string, such as the example program of a package comment, is not an import. A package cannot import itself, so an import of the file's own directory adds no edge, unless the file is an external test package.
- Test files (`_test.go`) are not part of what a package offers, and neither are the files the toolchain ignores (`_x.go`, `.x.go`). A test in an external test package (`package store_test`) that imports its own package depends on the files beside it.
- Every `go.mod` under the analysed directory is read, so the modules of a repository with several resolve each from its own directory, with or without a `go.work`. An import of the module path itself is the package at the root of the module.
- The files of one package use each other without importing, so there are no edges between them. The import graph is therefore a lower bound on the real dependencies.
- Not seen: build constraints (`//go:build`, file names such as `x_linux.go`), so a file left out of a build still counts, and packages under `vendor/`.
- An import under the path of a module of the repository that names no directory with a package is reported as unresolved.

JavaScript/TypeScript notes:

- Only the import forms are read: `import … from`, a bare `import '…'`, `export … from`, `require('…')`, `import('…')` and `import x = require('…')`, several to a line or over several lines. Any other string is no import, so a path given to `fs.readFile` or `path.resolve` is not a dependency.
- A relative specifier resolves from the importing file, trying the extensions of JavaScript and TypeScript and the `index` of a directory. A `.js` that does not exist names its TypeScript source (`./a.js` is `a.ts`), and a query such as Vite's `?raw` names the same file.
- Any other specifier goes first through the `paths` of the nearest `tsconfig.json` or `jsconfig.json` above the importing file, the most specific pattern winning, then through its `baseUrl`. `extends` is followed within the repository, and a directory whose main config has no aliases takes those of the others beside it (`tsconfig.app.json`). Comments and trailing commas are allowed.
- Then a specifier may name a package of the repository, by the `name` of its `package.json`: `@acme/ui` leads to its entry point (`source`, `module`, `main`, `types`, `exports`, read from `src` when they point to a build directory, or else `src/index`), and `@acme/ui/button` to that file of the package or of its `src`. Packages under `node_modules` are not read. What resolves to none of these is external.
- A barrel (`index.ts`) is a file like any other: its importers depend on it, and it on what it re-exports.
- `import type`, `export type … from`, and an import or re-export whose names are all marked `type` (`import { type A, type B }`) take only types: the compiler erases them, so they are `type` and make no cycle. One name that is a value, or a default import beside them, makes the import load. `import()` is `deferred`: the module loads when the call runs.
- A relative specifier that resolves to no file is reported as unresolved when it names a script: no extension or one of JavaScript or TypeScript. A stylesheet or an image is no file of the graph.
- Not seen: the aliases of a bundler config (`resolve.alias` of Vite or webpack), `imports` of `package.json` (`#internal`), specifiers built at run time, and `.vue` and `.svelte` files.

C and C++ notes:

- The include directories of the build (`-I`, CMake's `include_directories`) are not read. A quoted `#include "a/b.h"` is looked for beside the includer first; then, quoted or not, in each directory above it and in the `include` directory of each; then wherever a file of the repository ends with that path, when one is nearest to the includer.
- `#include <stdio.h>` and any other bare name in angle brackets that is in none of those directories is a system header: no edge, even if the repository holds a file of that name somewhere else.
- A file that includes a header uses what its source defines too: `foo.h` and the `foo.c` beside it, or `include/x/foo.h` and `src/x/foo.c`. `km deps` shows only the includes the code writes, so a source and a header that include each other's counterpart make no cycle; `km impact` follows both, so a change to `foo.c` reaches whoever includes `foo.h`.
- Preprocessor conditions (`#ifdef`) are not evaluated: every include counts.
- A quoted include that resolves to no file of the repository is reported as unresolved, unless it names a header of the C library or of POSIX (`"stdio.h"`, `"sys/types.h"`), which a quoted include may also reach. An include in angle brackets is never reported.

Java notes:

- A class is known by its package and its name, and lives in the file of its top-level class, wherever the source root is (`src/main/java`, `src`, a module of a multi-module build).
- A file uses the classes it names: those of its own package and of each import on demand (`import a.b.*`) by their simple name, each class it imports by itself, and any class written with its package (`a.b.C`). A nested class (`a.b.Outer.Inner`) and a static import lead to the file of the outer class. An import is a use even when nothing names the class.
- When several files declare the same class (two flavors of one library, a generated copy), the one nearest to the file that names it is taken.
- The JDK and libraries declare no file of the repository: their classes are external.
- A field, a variable or a method that has a class's name is not told from it.

C# notes:

- A `using` names a namespace, not a file. A type is known by its namespace (block-scoped or file-scoped) and its name, and a file uses the types it names.
- A file sees by their simple name the types of each namespace it is in and of those around it (`App.Web` sees `App`), of the global namespace, of each `using`, and of each `global using` written in its directory or above it. An alias (`using M = App.Models;`) names a type or a namespace, `using static` uses the type it names, and `[Audited]` names the class `AuditedAttribute`.
- A partial type is declared by each of its files, and a use of it uses them all. When two projects declare a type of the same name, the one nearest to the file that names it is taken.
- Types of .NET and of NuGet packages declare no file of the repository: external. The implicit usings of the SDK bring nothing of the repository.
- An extension method (`this` before the first parameter, in a static class) is called on a value without naming its class. A call after a dot (`t.IsValueType()`) uses the file that declares an extension method of that name, taking as many arguments, in a namespace the caller sees. The receiver's type is not known: a call several files could answer is ambiguous and adds no edge, and an instance method that shares the name and the arity of a visible extension is taken for it. Private extension methods are left out.
- A field, a variable or a method that has a type's name is not told from it.

Ruby notes:

- `require_relative` resolves from the requiring file, and so does a path built from `__dir__` or `__FILE__` (`File.expand_path('../x', __FILE__)`, `File.join(__dir__, 'x')`). `require`, `load` and `autoload` look the path up under the `lib`, `test` and `spec` directories of the repository and the autoload roots of Rails (`app/*`, `app/*/concerns`), the nearest first; a path found nowhere is a gem or the standard library.
- Most Rails code has no `require`: a file uses the files that define the constants it names. Constants are always looked up, in any project, since once loaded a constant is global whatever loaded it (`require`, Zeitwerk, Bundler). A reference is looked up whole from the namespaces it is written in, innermost first (`Item` inside `module Shop` is `Shop::Item` before `Item`).
- Classes and modules nest by indentation, as formatted code has them. A module opened only to hold other definitions is a namespace, not a definition: `module Shop` around `class Cart` does not make every reference to `Shop` a use of that file. A constant assigned in a class or module (`VERSION = "1"`, `Point = Struct.new(:x)`) is a definition.
- A constant defined in several files (a reopened class): the file named after it as Zeitwerk names files (`Shop::LineItem` in `shop/line_item.rb`) is the definition. When none is, the nearest definitions to the reference count, all of them when they sit in one directory; spread over directories with none nearer, the reference is ambiguous and left out (`km impact` counts it). A constant no file defines is found by its path under the autoload roots.
- Shipping code never uses what a test file or a test directory defines; tests see both.
- The core classes of Ruby (`String`, `Hash`, `Object`…) are never a use, even when a file of the repository reopens them.
- Comments, strings, heredocs, regular expressions and `%` literals name nothing; the code a string interpolates does.
- Not seen: `const_get`, `send` and other metaprogramming; methods added by reopening a class in a file not named after it, when a reference resolves to the file named after the class; ERB and other templates; custom autoload paths (`config.autoload_paths`) and collapsed directories; request specs that reach a controller through its route.

Kaikai notes:

- `import a.b.c` names `a/b/c.kai` relative to a package root, not to the importing file. Each ancestor directory of the importing file is tried, nearest first, so run `km deps` on a directory that contains the package root.
- When no such file exists, `import a` can name a package directory `a/` that carries a `kai.toml`, which the compiler searches: the import depends on `a/a.kai`, or, when the package has no file of its name, on every `.kai` file directly inside it.
- An import that resolves to no analysed file is external and adds no edge.
- Each file is a module of its own. The files of a package do not share names: they import each other like any other module (`import store`, `import notes.store`), so their only edges are their imports, and a change to one reaches the files of its package that import it. Unlike Go, where the files of a package see each other without importing and an import reaches them all.

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--cycles-only` | Show only files that participate in a dependency cycle |
| `--sort-by METRIC` | Sort by `fan-out` (default) or `fan-in` |
| `--top N` | Rows of the table (default: 20); JSON lists every file unless `--top` is given |
| `--matrix` | Print the dependency structure matrix instead of the table |
| `--depth N` | Levels of directories the matrix shows expanded (default: 1) |
| `--focus DIR` | Expand this directory of the matrix, and the ones above it, alone |
| `--html FILE` | Write the matrix as an interactive HTML page to this file |

Example output:

```
Dependency Graph
────────────────────────────────────────────────────────────────────────
 File                 Language Fan-In Fan-Out Cycle
────────────────────────────────────────────────────────────────────────
 report_helpers.rs        Rust     26       1    no
 util.rs                  Rust     25       2   yes
 walk.rs                  Rust     24       1    no
────────────────────────────────────────────────────────────────────────

Dependency cycles: 14
  Cycle 1 (3 files):
    cogcom/analyzer.rs
    cogcom/detection.rs
    cogcom/report.rs
```

Dependency structure matrix:

`--matrix` shows which file depends on which as a dependency structure matrix (DSM), as IntelliJ IDEA draws it. Rows and columns are the same directories and files, in the same order; a column carries only the number of its row, so the matrix stays narrow. A cell counts the files of its column that use files of its row. A directory shown collapsed (`▸`) counts for every file under it; one shown expanded (`▾`) heads the rows of what it holds. The root is the deepest directory holding every file, and a directory that holds only another one is shown with it (`lib/core/`).

The children of each directory are ordered so that a sibling comes before the siblings it uses: in a layered system every mark falls below the diagonal. Siblings that use each other form a cycle and stay together, those that use more of it than it uses them first; the rest keep the order of their names. Only uses that load or compile order the rows: `type` and `deferred` ones, which make no cycle, are drawn dimmed.

A mark above the diagonal goes against the layering. The list under the matrix gives each one with the uses behind it, five at most, and why it is there: in a cycle between files (`!` after the count, red on a terminal), against the layering when only the directories form a cycle (yellow), or type-only or deferred (dimmed). There are no colours when the output is not a terminal or `NO_COLOR` is set.

`--depth N` expands the directories less than N levels below the root; `--focus DIR` expands that directory alone and the ones above it, with the path written as km shows it. `--cycles-only` keeps the files in cycles. `--top` does not apply to the matrix. With `--format json` it is given for tools and models: `rows` in their order (`number`, `path`, a directory ending in `/`, `level`), `groups` with the directories shown expanded, `cells` with `row`, `column`, `count`, `mark` (`cycle`, `layering`, `weak` or `plain`) and the `uses` behind each (`from`, `to`, `kind`, `cycle`), and `above_diagonal` as `[row, column]` pairs. A matrix wider than the terminal is not printed: a lower `--depth` or `--focus` narrows it.

`--html FILE` writes the matrix as one HTML file that needs nothing else, in light and dark themes. It opens as the text would show, and in it a directory expands and collapses with a click on its triangle (or all at once), a click on a name marks what it uses and what uses it in two colours, and a click on a cell lists the uses behind it. The headers stay in place when the matrix scrolls, a cell is shaded by its count, and a name shows its whole path on hover. The terminal width does not limit it.

```
$ km deps --matrix --focus shop/core
Dependency Structure Matrix
A cell counts the files of its column that use files of its row; ! marks a cycle.

                1  2  3  4  5
 1 main.py      ·
 2 ▸ billing/   1  ·
 3 ▸ web/       1  1  ·
   ▾ core/
 4   db.py            1  · 1!
 5   models.py     1  2 1!  ·

Above the diagonal: 1 cell
  shop/core/models.py → shop/core/db.py: 1, in a cycle
    shop/core/models.py → shop/core/db.py
```

### `km authors` -- Per-author ownership summary

Summarizes code ownership across the project by author. Aggregates `git blame` data to answer "who knows what?" at the team level — complementing `km knowledge` (per-file view) with a team-level view. Blame is read as `km knowledge` reads it (see [Who wrote each line](#who-wrote-each-line)). "Last Active" is the date of the author's last commit, whatever it touched.

```bash
km authors [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--since DURATION` | Only consider activity since this time (e.g. `6m`, `1y`, `30d`) |
| `--format {table,json,short,terse}` | Output format (default: table) |

Example output:

```
──────────────────────────────────────────────────────────────────────
 Author              Owned      Lines  Languages    Last Active
──────────────────────────────────────────────────────────────────────
 E. Diaz                38       8432  Rust, TOML   2026-03-15
 R. Ramirez              4        312  Rust         2026-02-10
──────────────────────────────────────────────────────────────────────
```

### `km age` -- File age analysis

Classifies source files as **Active**, **Stale**, or **Frozen** based on how long ago they were last modified in git history. Helps identify neglected or abandoned code.

```bash
km age [path]
```

#### Status classification

| Status | Condition | Meaning |
|--------|-----------|---------|
| ACTIVE | Modified within `--active-days` days (default: 90) | Regularly touched |
| STALE | Between `--active-days` and `--frozen-days` (default: 365) | Neglected |
| FROZEN | Not modified for more than `--frozen-days` days | Potentially abandoned |

Options:

| Flag | Description |
|------|-------------|
| `--active-days N` | Days threshold for Active status (default: 90) |
| `--frozen-days N` | Days threshold for Frozen status (default: 365) |
| `--sort-by METRIC` | Sort by `date` (oldest first, default), `status`, or `file` |
| `--status FILTER` | Show only files with this status: `active`, `stale`, or `frozen` |
| `--format {table,json,short,terse}` | Output format (default: table) |

Example output:

```
──────────────────────────────────────────────────────────────────────────────
 File                    Language     Last Modified  Days  Status
──────────────────────────────────────────────────────────────────────────────
 src/legacy/parser.rs    Rust           2023-01-15   840  FROZEN
 src/util.rs             Rust           2024-09-20   197  STALE
 src/main.rs             Rust           2026-03-01    34  ACTIVE
──────────────────────────────────────────────────────────────────────────────

  ACTIVE     12  (modified < 90 days)
  STALE       8  (90 days – 365 days)
  FROZEN      3  (not modified > 365 days)
```

### `km score` -- Code health score

Computes an overall code health score for the project, grading it from A++ (exceptional) to F-- (severe issues). Uses only static metrics (no git required).

There are three scoring models: `cogcom`, the default, built on Cognitive Complexity (5 dimensions); `cogcom-0.35`, the same dimensions with the weights it had up to v0.35; and `legacy`, built on the Maintainability Index and Cyclomatic Complexity (6 dimensions).

Non-code files (Markdown, TOML, JSON, etc.) are automatically excluded. Inline test blocks (`#[cfg(test)]`) are excluded from every dimension, see [Inline tests](#inline-tests).

```bash
km score [path]
km score --model cogcom-0.35 [path]  # the weights up to v0.35
km score --model legacy [path]    # MI + cyclomatic model
```

#### Dimensions and weights (default: cogcom)

| Dimension | Weight | What it measures |
|-----------|--------|-----------------|
| Cognitive Complexity | 32% | SonarSource method, penalizes nesting |
| Duplication | 21% | Project-wide duplicate code % |
| Indentation Complexity | 16% | Stddev of indentation depth |
| Halstead Effort | 21% | Mental effort per LOC |
| File Size | 10% | Full score up to 500 LOC, falling to 0 at 4,000 |

Up to v0.35 File Size weighed 15%, and the others 30%, 20%, 15% and 20%. It weighs less now because size already drives the other dimensions: a large file has more code to be complex, and the project score is a mean weighted by lines of code. Studies of modules find that defects grow less than proportionally with size, so splitting a file does not by itself make it less fault-prone; what size measures well is the cost of reading and changing it. The five points went to the other dimensions in proportion to their weights. `--model cogcom-0.35` scores with the former weights, to compare with earlier results or to keep a CI gate where it was.

#### Experimental model (--experimental)

`km score --experimental` scores with the Maintainability Index (Visual Studio variant, 0–100) in place of Halstead Effort, at its weight (21%), and prints the score of the default model next to it, from the same measurement. The MI weighs Halstead volume, cyclomatic complexity and size together; a file scores 0 at MI 0, 40 at 5, 60 at 10, 80 at 20 (where Visual Studio turns green), 90 at 30, 97 at 45 and 100 from 60. Unlike the other dimensions, the MI of the project is the plain mean of its files, not weighted by lines: the MI already falls with the size of a file, and weighting it would count size twice. It is for trying the dimension out, not for gates: it cannot be combined with `--model` or `--trend`.

On the projects it was tried on, the experimental score stays within about 1.5 points of the default one, in the same order: the MI follows Halstead closely, from which it is largely computed. A project whose worst code sits in a few large files among many small ones scores higher than with the default model (jq: F 51.3 against D 60.5), since each file counts once.

```
 Experimental: Maintainability Index (Visual Studio) in place of Halstead Effort.
 Default model: B+ (83.2)  →  experimental: B (81.8), -1.5
```

`--format json` adds a `baseline` object with the score and grade of the default model; `--format short` adds a `baseline` line.

#### Dimensions and weights (--model legacy)

| Dimension | Weight | What it measures |
|-----------|--------|-----------------|
| Maintainability Index | 30% | Verifysoft MI, normalized to 0-100 |
| Cyclomatic Complexity | 20% | Max complexity per file |
| Duplication | 15% | Project-wide duplicate code % |
| Indentation Complexity | 15% | Stddev of indentation depth |
| Halstead Effort | 15% | Mental effort per LOC |
| File Size | 5% | Full score up to 500 LOC, falling to 0 at 4,000 |

Each dimension is aggregated as a LOC-weighted mean across all files (except Duplication which is a single project-level value). The project score is the weighted sum of all dimension scores.

#### Grade scale

| Grade | Score range | Grade | Score range |
|-------|------------|-------|------------|
| A++ | 97-100 | C+ | 73-76 |
| A+ | 93-96 | C | 70-72 |
| A | 90-92 | C- | 67-69 |
| A- | 87-89 | D+ | 63-66 |
| B+ | 83-86 | D | 60-62 |
| B | 80-82 | D- | 57-59 |
| B- | 77-79 | F | 50-56 |
| | | F- | 40-49 |
| | | F-- | 0-39 |

Options:

| Flag | Description |
|------|-------------|
| `--model MODEL` | Scoring model: `cogcom` (default), `cogcom-0.35` (the weights up to v0.35) or `legacy` (MI + cyclomatic) |
| `--experimental` | Score with the Maintainability Index in place of Halstead Effort, against the default model |
| `--trend [REF]` | Compare current score against a git ref (default: `HEAD`). Shows change: `B- → B (+2.3)`. Useful for PR review: `--trend origin/main` |
| `--fail-if-worse` | With `--trend`: exit with code 1 if the score dropped by more than `--gate-tolerance` |
| `--gate-tolerance POINTS` | Score drop `--fail-if-worse` allows before failing (default: `0.01` with `--gate-scope project`, `0.5` with `--gate-scope changed`). Compares unrounded scores |
| `--gate-scope {project,changed}` | What `--fail-if-worse` compares (default: `project`, the aggregate score). `changed` looks only at the files the diff touches: it fails if a modified or renamed file ends below the project score at the ref after dropping more than `--gate-tolerance`, or if the project's duplicated lines grow. Files above the project score, new files and deleted files never fail it, so removing healthy code cannot lower the verdict. The report lists every changed file with its before/after score |
| `--fail-below GRADE` | With `--trend`: exit with code 1 if the score is below `GRADE` (e.g. `B-`). Overridable via `.kimun.toml` |
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include tests, examples and benchmarks in analysis (excluded by default) |
| `--bottom N` | Number of worst files to show in "needs attention" (default: 10) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |

Example output:

```
Code Health Score
──────────────────────────────────────────────────────────────────
 Project Score:  A (91.2)
 Files Analyzed: 142
 Total LOC:      19,084
──────────────────────────────────────────────────────────────────
 Dimension                 Weight   Score   Grade
──────────────────────────────────────────────────────────────────
 Cognitive Complexity         32%    84.1   B+
 Duplication                  21%   100.0   A++
 Indentation Complexity       16%    93.2   A+
 Halstead Effort              21%    87.5   A-
 File Size                    10%   100.0   A++
──────────────────────────────────────────────────────────────────

 Files Needing Attention (worst per-file scores, excl. duplication)
──────────────────────────────────────────────────────────────────
 Score  Grade  File                       Issues
──────────────────────────────────────────────────────────────────
  57.7  D-     ./src/deps/analyzer.rs     Cognitive: 36
  61.0  D      ./src/cogcom/analyzer.rs   Cognitive: 35
  70.3  C      ./src/smells/rules.rs      Cognitive: 20
  71.6  C      ./src/hal/tokenizer.rs     Cognitive: 22
  71.8  C      ./src/loc/mod.rs           Cognitive: 19
  72.8  C      ./src/knowledge/mod.rs     Cognitive: 21
  73.8  C+     ./src/dups/mod.rs          Cognitive: 19
  75.0  C+     ./src/cli_help.rs          Cognitive: 17
  75.6  C+     ./src/test_blocks.rs       Cognitive: 17
  77.3  B-     ./src/walk.rs              Cognitive: 16
──────────────────────────────────────────────────────────────────
```

#### `km score diff` — Compare score against a git ref

Extracts the file tree at the given ref, computes the score for both snapshots, and shows a delta table per dimension. Useful for reviewing how commits impact code quality.

```bash
km score diff                          # compare vs HEAD (uncommitted changes)
km score diff --git-ref HEAD~1         # compare vs previous commit
km score diff --git-ref main           # compare vs main branch
km score diff --format json            # machine-readable output
```

Options:

| Flag | Description |
|------|-------------|
| `--git-ref REF` | Git ref to compare against (default: `HEAD`) |
| `--model MODEL` | Scoring model: `cogcom` (default), `cogcom-0.35` or `legacy` |
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--bottom N` | Number of worst files to show (default: 10) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |

### `km report` -- Comprehensive metrics report

Generates a multi-section report combining all static code metrics in a single pass: lines of code, duplicates, indentation, Halstead, cyclomatic complexity, cognitive complexity, and maintainability index.

```bash
km report [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--top N` | Show only the top N files per section (default: 20) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |
| `--full` | Show all files instead of truncating to top N |
| `--format {table,json,short,terse}` | Output format (default: table) |

## Project configuration (`.kimun.toml`)

Run `km init` to analyze your project and generate a calibrated `.kimun.toml` in one step:

```
$ km init
Analyzing project... done.

Current state:
  avg function length: 38 lines  →  suggested max_lines = 45
  avg param count:     3.2       →  suggested max_params = 4
  dup ratio:           4.1%      →  suggested max_dup_ratio = 5.0
  health score:        B+        →  suggested fail_below = B

Write .kimun.toml with these values? [Y/n]
```

Use `--yes` to skip the prompt. Add `-y` in CI to write the file non-interactively.

Alternatively, place a `.kimun.toml` file manually in the root of your repository to set project-level defaults for thresholds and quality gates. `km` searches for the file at the git repository root, falling back to the current directory.

CLI flags always take precedence over `.kimun.toml`, which in turn takes precedence over built-in defaults.

```toml
[smells]
max_lines  = 30    # flag functions longer than N body lines (default: 50)
max_params = 3     # flag functions with more than N parameters (default: 4)

[dups]
min_lines      = 8     # minimum block size for duplication detection (default: 6)
                       # also applies to `km report` and `km score`
max_duplicates = 10    # CI gate: fail if duplicate groups exceed N
max_dup_ratio  = 5.0   # CI gate: fail if duplicated-lines ratio exceeds this %

[score]
model      = "cogcom"  # scoring model: cogcom (default), cogcom-0.35 or legacy
fail_below = "B-"      # CI gate: fail if health score is below this grade

[age]
active_days = 60    # files modified within N days are Active (default: 90)
frozen_days = 180   # files not modified for more than N days are Frozen (default: 365)

[tc]
min_degree   = 5    # minimum commits per file to include in coupling analysis (default: 3)
min_strength = 0.5  # only show pairs with coupling strength >= this value

[hotspots]
complexity = "cogcom"  # complexity metric: indent (default), cycom, or cogcom

[impact]
inert = ["scripts/**"]  # changed files that reach nothing, besides documentation
entry_points = ["**/endpoint.ex"]  # files run rather than used, besides tasks and scripts
```

All sections and fields are optional — omit any you don't need. A fully documented template is available at [`.kimun.toml.example`](.kimun.toml.example).

## Features

- Respects `.gitignore` rules automatically
- Deduplicates files by content hash (identical files counted once)
- Detects languages by file extension, filename, or shebang line
- Supports nested block comments (Rust, Haskell, OCaml, etc.)
- Handles pragmas (e.g., Haskell `{-# LANGUAGE ... #-}`) as code
- Mixed lines (code + comment) are counted as code, matching `cloc` behavior
- Reads sources in any 8-bit encoding (Latin-1, Windows-1252) whole: bytes that are not UTF-8 are replaced, not taken for the end of the file

## Supported Languages

| Language | Extensions / Filenames |
|---|---|
| Bourne Again Shell | `.bash` |
| Bourne Shell | `.sh` |
| C | `.c`, `.h` |
| C# | `.cs` |
| C++ | `.cpp`, `.cxx`, `.cc`, `.hpp`, `.hxx`, `.hh` |
| Clojure | `.clj`, `.cljs`, `.cljc`, `.edn` |
| CMake | `.cmake`, `CMakeLists.txt` |
| CSS | `.css` |
| Dart | `.dart` |
| Dockerfile | `Dockerfile` |
| DOS Batch | `.bat`, `.cmd` |
| Elixir | `.ex` |
| EEx | `.eex` |
| Elixir Script | `.exs` |
| Erlang | `.erl`, `.hrl` |
| F# | `.fs`, `.fsi`, `.fsx` |
| Fortran 77 | `.f`, `.for`, `.ftn`, `.f77`, `.pfo` (and upper case) |
| Fortran 90 | `.f90`, `.F90` |
| Fortran 95 | `.f95`, `.F95` |
| Fortran 2003 | `.f03`, `.F03` |
| Fortran 2008 | `.f08`, `.F08` |
| Go | `.go` |
| Gradle | `.gradle` |
| Groovy | `.groovy` |
| Haskell | `.hs` |
| HTML | `.html`, `.htm` |
| HTML EEx | `.heex` |
| Java | `.java` |
| JavaScript | `.js`, `.mjs`, `.cjs` |
| jq | `.jq` |
| JSON | `.json` |
| JSX | `.jsx` |
| Julia | `.jl` |
| Kaikai | `.kai` |
| Kotlin | `.kt`, `.kts` |
| lex | `.l`, `.lex` |
| Lua | `.lua` |
| Makefile | `.mk`, `Makefile`, `makefile`, `GNUmakefile` |
| Markdown | `.md`, `.markdown` |
| Nim | `.nim` |
| Objective-C | `.m`, `.mm` |
| OCaml | `.ml`, `.mli` |
| Pascal | `.pas`, `.dpr`, `.dpk`, `.lpr`, `.pp`, `.pascal` |
| Perl | `.pl`, `.pm` |
| PHP | `.php` |
| Properties | `.properties` |
| Python | `.py`, `.pyi` |
| R | `.r`, `.R` |
| reStructuredText | `.rst`, `.rest` |
| Ruby | `.rb`, `Rakefile`, `Gemfile` |
| Rust | `.rs` |
| Scala | `.scala`, `.sc`, `.sbt` |
| SQL | `.sql` |
| Swift | `.swift` |
| Terraform | `.tf` |
| Text | `.txt` |
| TOML | `.toml` |
| TypeScript | `.ts`, `.tsx`, `.mts`, `.cts` |
| yacc | `.y`, `.yacc` |
| XML | `.xml`, `.xsl`, `.xslt`, `.svg`, `.fsproj`, `.csproj`, `.vbproj`, `.vcxproj`, `.sln`, `.plist`, `.xaml` |
| YAML | `.yaml`, `.yml` |
| Zig | `.zig` |
| Zsh | `.zsh` |

### Language-specific notes

- **Kaikai** — `#[...]` opens an attribute, not a `#` comment, so attributes count
  as code. Documentation attributes (`#[doc("...")]`, including the multi-line
  `#[doc("""...""")]` form) count as comments and are excluded from the
  complexity, Halstead, and smell analyses.
- **Elixir** — the heredocs of `@moduledoc` and `@doc` (`"""`, `'''`, and the
  `~S"""` and `~s"""` sigils) count as comments, as cloc counts them, and are
  excluded from the complexity, Halstead, and smell analyses. A one-line
  `@doc "..."` and any other heredoc count as code.
- **Python** — a triple-quoted string, a docstring above all, counts as comment,
  as cloc counts it: its lines are comment unless code shares them
  (`SQL = """` is code, the lines after it are comment).
- **C and C++ headers** — a `.h` is C++ when its text says so (a line opening
  with `namespace`, `template`, `class`, an access specifier, `using`,
  `constexpr`, or any `std::`), and C otherwise. cloc reports every header as
  `C/C++ Header`; kimun keeps them with the language that analyzes them.
- **reStructuredText** — a line opening with `..` (a directive or a comment)
  counts as comment; cloc also counts as comment the lines indented under it.
- **HTML EEx** — `<%!-- --%>` and lines opening with `<% #` are comments, as
  cloc counts them. **EEx** — `<%# %>` is a comment.
- **Fortran** — free form (`.f90` and later) comments with `!`; fixed form
  (`.f`, `.for`, `.f77`) also makes a comment of a line with `C`, `c` or `*` in
  its first column. OpenMP and OpenACC directives (`!$omp`, `C$OMP`, `!$acc`)
  and conditional compilation (`!$ `) count as code, as the directives they
  are; cloc counts them as comments, and in free form it also takes a line that
  starts with `c` in the first column (`contains`, `call`) for a comment. A
  backslash escapes nothing in a string. Fortran ignores case, and so do the
  complexity, Halstead and smell analyses, while the reports name functions as
  they are written. Functions are the program units (`subroutine`,
  `function`, `program`) to the `end` that closes them, the units they contain
  included; those of an `interface` block declare no function. Cognitive
  nesting is read from the constructs (`if … then`, `do`, `select`, `where`)
  and the `end` of each, not from indentation, which fixed form lacks.
- **Pascal (Delphi, Free Pascal)** — three kinds of comment: `//`,
  `{ … }` and `(* … *)`, the last two closed only by their own delimiter.
  `{$…}` is a compiler directive and counts as code. Strings are
  single-quoted, with no backslash escapes; a delimiter inside one opens no
  comment (cloc does not tell them apart, so it often counts code as comment
  after a `'{'` or a `COUNT(*)`). Pascal ignores case, as Fortran does.
  Functions are the routines of the implementation (`procedure`, `function`,
  `constructor`, `destructor`, `operator`, class methods), from their header to
  the `end` of their block, local routines included; the declarations of the
  `interface` section and of class types, and `forward` or `external` ones,
  have none. Headers repeated under conditional compilation are one routine.
  Cognitive nesting is read from indentation, two columns a level.
- **Interior lines of multi-line strings** (Kaikai triple-quoted literals,
  Elixir heredocs) count as code for `km loc`, but are excluded from
  cyclomatic, cognitive, Halstead, and smell analyses: prose and embedded data
  are not control flow.
- **Rust raw strings** (`r"..."`, `r#"..."#` and the forms with more `#`, with a
  `b` or `c` prefix too) end only at a quote followed by as many `#` as opened
  them, and span lines. What they hold counts as code for `km loc`, whatever it
  looks like, and is left out of the cyclomatic, cognitive, Halstead, and smell
  analyses.

## Development

```bash
cargo build              # build debug binary
cargo test               # run all tests
cargo clippy             # lint (zero warnings required)
cargo tarpaulin --out stdout  # coverage report
```

## References

The metrics and methodologies implemented in Kimün are based on the following sources:

### Books

- **Adam Thornhill**, *Your Code as a Crime Scene* (Pragmatic Bookshelf, 2015). Basis for hotspot analysis (ch. 4–5), temporal coupling (ch. 7), knowledge maps / code ownership (ch. 8–9), and indentation-based complexity as a proxy for code quality.
- **Adam Thornhill**, *Software Design X-Rays* (Pragmatic Bookshelf, 2018). Extends the crime scene metaphor with additional behavioral code analysis techniques.

### Papers and standards

- **Maurice H. Halstead**, *Elements of Software Science* (Elsevier, 1977). Defines the operator/operand metrics: vocabulary, volume, difficulty, effort, estimated bugs, and development time.
- **Thomas J. McCabe**, "A Complexity Measure", *IEEE Transactions on Software Engineering*, SE-2(4), December 1976, pp. 308–320. Introduces cyclomatic complexity as a measure of independent paths through a program's control flow graph.
- **Paul Oman & Jack Hagemeister**, "Metrics for Assessing a Software System's Maintainability", *Proceedings of the International Conference on Software Maintenance (ICSM)*, 1992. Original Maintainability Index formula combining Halstead Volume, cyclomatic complexity, and lines of code.
- **Microsoft**, [Code Metrics — Maintainability Index range and meaning](https://learn.microsoft.com/en-us/visualstudio/code-quality/code-metrics-maintainability-index-range-and-meaning). Visual Studio variant: normalized to 0–100 scale, no comment-weight term.
- **Verifysoft**, [Maintainability Index](https://www.verifysoft.com/en_maintainability.html). Extended MI formula with a comment-weight component (MIcw) that rewards well-commented code.
- **Yasutaka Kamei et al.**, "A Large-Scale Empirical Study of Just-in-Time Quality Assurance" (IEEE TSE 39(6), 2013). Basis for the diffusion measures of `km impact`.
- **Thomas Zimmermann, Andreas Zeller, Peter Weissgerber, Stephan Diehl**, "Mining Version Histories to Guide Software Changes" (IEEE TSE 31(6), 2005). Basis for the logical radius of `km impact`.

## The name

<p align="center">
  <img src="assets/madu.png" alt="Madu, the mascot of Kimün" width="220">
</p>

*Kimün* means "knowledge" or "wisdom" in Mapudungun, the language of the Mapuche people.

The mascot is **Madu**, a pudú who is a *machi*: the one who holds the knowledge of her people. She is the sister of Kalku, the mascot of [kalku](https://github.com/lnds/kalku). She explains Kimün in [a two-minute video](https://kimun.tools/en/#video):

<p align="center">
  <a href="https://kimun.tools/en/#video"><img src="assets/video-en.jpg" alt="Play the video: Madu, the mascot, beside the words 'kimün = knowledge'" width="720"></a>
</p>

## License

MIT. See [LICENSE](LICENSE).
