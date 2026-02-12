# cm (code-metrics)

A fast command-line tool for code analysis, written in Rust. Counts lines of code by language (inspired by [cloc](https://github.com/AlDanial/cloc)), detects duplicate code blocks, and computes complexity metrics (Halstead, cyclomatic, indentation, maintainability index).

## Installation

```bash
cargo install --path .
```

This installs the `cm` binary.

## Commands

### `cm loc` -- Count lines of code

```bash
cm loc [path]
```

Run on the current directory:

```bash
cm loc
```

Run on a specific path:

```bash
cm loc src/
```

Options:

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Show summary stats (files read, unique, ignored, elapsed time) |
| `--json` | Output as JSON |

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

### `cm dups` -- Detect duplicate code

Finds duplicate code blocks across files using a sliding window approach. Applies the **Rule of Three**: duplicates appearing 3+ times are marked as **CRITICAL** (refactor recommended), while those appearing twice are **TOLERABLE**.

Test files and directories are excluded by default, since tests often contain intentional repetition.

```bash
cm dups [path]
```

Options:

| Flag | Description |
|------|-------------|
| `-r`, `--report` | Show detailed report with duplicate locations and code samples |
| `--show-all` | Show all duplicate groups (default: top 20) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--json` | Output as JSON |

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

By default, `cm dups` skips files matching common test conventions:

- **Directories**: `tests/`, `test/`, `__tests__/`, `spec/`
- **By extension**: `*_test.rs`, `*_test.go`, `test_*.py`, `*.test.js`, `*.spec.ts`, `*Test.java`, `*_test.cpp`, and more

Use `--include-tests` to analyze test files as well.

### `cm indent` -- Indentation complexity

Measures indentation-based complexity per file: standard deviation of indentation depths and maximum depth. Higher stddev suggests more complex control flow.

```bash
cm indent [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |

### `cm hal` -- Halstead complexity metrics

Computes [Halstead complexity metrics](https://en.wikipedia.org/wiki/Halstead_complexity_measures) per file by extracting operators and operands from source code.

```bash
cm hal [path]
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

Options:

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON (includes all metrics) |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
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

Rust, Python, JavaScript, TypeScript, Go, C, C++, C#, Java, Objective-C, PHP, Dart, Ruby, Kotlin, Swift, Shell (Bash/Zsh).

### `cm cycom` -- Cyclomatic complexity

Computes cyclomatic complexity per file and per function by counting decision points (`if`, `for`, `while`, `match`, `&&`, `||`, etc.).

```bash
cm cycom [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
| `--min-complexity N` | Minimum max-complexity to include a file (default: 1) |
| `--per-function` | Show per-function breakdown |

### `cm mi` -- Maintainability Index (Visual Studio variant)

Computes the [Maintainability Index](https://learn.microsoft.com/en-us/visualstudio/code-quality/code-metrics-maintainability-index-range-and-meaning) per file using the Visual Studio formula. MI is normalized to a 0–100 scale with no comment-weight term.

```bash
cm mi [path]
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
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
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
 Total (3 files)                         1157   13.2
```

### `cm miv` -- Maintainability Index (verifysoft variant)

Computes the [Maintainability Index](https://www.verifysoft.com/en_maintainability.html) per file. MI combines Halstead Volume, Cyclomatic Complexity, lines of code, and comment ratio into a single maintainability score.

This is the verifysoft.com variant, which includes a comment-weight term (MIcw) that rewards well-commented code.

```bash
cm miv [path]
```

#### Formula

```
MIwoc = 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)
MIcw  = 50 * sin(sqrt(2.46 * radians(PerCM)))
MI    = MIwoc + MIcw
```

Where V = Halstead Volume, G = cyclomatic complexity, LOC = code lines, PerCM = comment percentage (converted to radians).

#### Thresholds

| MI Score | Level | Meaning |
|----------|-------|---------|
| 85+ | good | Easy to maintain |
| 65–84 | moderate | Reasonable maintainability |
| <65 | difficult | Hard to maintain |

Options:

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
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
 Total (3 files)                         1011                  51.9
```

### `cm hotspots` -- Hotspot analysis

Finds hotspots: files that change frequently AND have high complexity. Based on Adam Thornhill's method ("Your Code as a Crime Scene").

```bash
cm hotspots [path]
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
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
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

### `cm knowledge` -- Code ownership analysis

Analyzes code ownership patterns via git blame (knowledge maps). Based on Adam Thornhill's method ("Your Code as a Crime Scene" chapters 8-9).

```bash
cm knowledge [path]
```

Identifies bus factor risk and knowledge concentration per file. Generated files (lock files, minified JS, etc.) are automatically excluded.

#### Risk levels

| Risk | Condition | Meaning |
|------|-----------|---------|
| CRITICAL | 1 person owns >80% | High bus factor risk |
| HIGH | 1 person owns 60-80% | Significant concentration |
| MEDIUM | 2-3 people own >80% combined | Moderate concentration |
| LOW | Well-distributed | Healthy ownership |

#### Knowledge loss detection

Use `--since` to define "recent activity". If the primary owner of a file has no commits in that period, the file is flagged with **knowledge loss** risk. Use `--risk-only` to show only those files.

Options:

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
| `--sort-by METRIC` | Sort by `concentration`, `diffusion`, or `risk` (default: `concentration`) |
| `--since DURATION` | Define recent activity window for knowledge loss (e.g. `6m`, `1y`, `30d`) |
| `--risk-only` | Show only files with knowledge loss risk |

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

### `cm tc` -- Temporal coupling analysis

Analyzes temporal coupling between files via git history. Based on Adam Thornhill's method ("Your Code as a Crime Scene" ch. 7): files that frequently change together in the same commits have implicit coupling, even without direct imports.

```bash
cm tc [path]
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
| `--json` | Output as JSON |
| `--top N` | Show only the top N file pairs (default: 20) |
| `--sort-by METRIC` | Sort by `strength` or `shared` (default: `strength`) |
| `--since DURATION` | Only consider commits since this time (e.g. `6m`, `1y`, `30d`) |
| `--min-degree N` | Minimum commits per file to be included (default: 3) |
| `--min-strength F` | Minimum coupling strength to show (e.g. `0.5` for strong only) |

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

## Features

- Respects `.gitignore` rules automatically
- Deduplicates files by content hash (identical files counted once)
- Detects languages by file extension, filename, or shebang line
- Supports nested block comments (Rust, Haskell, OCaml, etc.)
- Handles pragmas (e.g., Haskell `{-# LANGUAGE ... #-}`) as code
- Mixed lines (code + comment) are counted as code, matching `cloc` behavior

## Supported Languages

| Language | Extensions / Filenames |
|---|---|
| Bourne Again Shell | `.bash` |
| Bourne Shell | `.sh` |
| C | `.c`, `.h` |
| C# | `.cs` |
| C++ | `.cpp`, `.cxx`, `.cc`, `.hpp`, `.hxx` |
| Clojure | `.clj`, `.cljs`, `.cljc`, `.edn` |
| CSS | `.css` |
| Dart | `.dart` |
| Dockerfile | `Dockerfile` |
| DOS Batch | `.bat`, `.cmd` |
| Elixir | `.ex` |
| Elixir Script | `.exs` |
| Erlang | `.erl`, `.hrl` |
| F# | `.fs`, `.fsi`, `.fsx` |
| Go | `.go` |
| Gradle | `.gradle` |
| Groovy | `.groovy` |
| Haskell | `.hs` |
| HTML | `.html`, `.htm` |
| Java | `.java` |
| JavaScript | `.js`, `.mjs`, `.cjs` |
| JSON | `.json` |
| Julia | `.jl` |
| Kotlin | `.kt`, `.kts` |
| Lua | `.lua` |
| Makefile | `.mk`, `Makefile`, `makefile`, `GNUmakefile` |
| Markdown | `.md`, `.markdown` |
| Nim | `.nim` |
| Objective-C | `.m`, `.mm` |
| OCaml | `.ml`, `.mli` |
| Perl | `.pl`, `.pm` |
| PHP | `.php` |
| Properties | `.properties` |
| Python | `.py`, `.pyi` |
| R | `.r`, `.R` |
| Ruby | `.rb`, `Rakefile`, `Gemfile` |
| Rust | `.rs` |
| Scala | `.scala`, `.sc`, `.sbt` |
| SQL | `.sql` |
| Swift | `.swift` |
| Terraform | `.tf` |
| Text | `.txt` |
| TOML | `.toml` |
| TypeScript | `.ts`, `.mts`, `.cts` |
| XML | `.xml`, `.xsl`, `.xslt`, `.svg`, `.fsproj`, `.csproj`, `.vbproj`, `.vcxproj`, `.sln`, `.plist`, `.xaml` |
| YAML | `.yaml`, `.yml` |
| Zig | `.zig` |
| Zsh | `.zsh` |

## Development

```bash
cargo build              # build debug binary
cargo test               # run all tests
cargo clippy             # lint (zero warnings required)
cargo tarpaulin --out stdout  # coverage report
```

## License

See [Cargo.toml](Cargo.toml) for package details.
