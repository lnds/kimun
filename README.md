# Kimün (km)

> *Kimün* means "knowledge" or "wisdom" in Mapudungun, the language of the Mapuche people.

A fast command-line tool for code analysis, written in Rust. Run `km score` on any project to get an overall health grade (A++ to F--) across five quality dimensions — cognitive complexity, duplication, indentation depth, Halstead effort, and file size — with a list of the files that need the most attention.

Beyond the aggregate score, Kimün provides 17 specialized commands:

- **Static metrics** — lines of code by language ([cloc](https://github.com/AlDanial/cloc)-compatible), duplicate detection (Rule of Three), Halstead complexity, cyclomatic complexity, cognitive complexity (SonarSource), indentation complexity, two Maintainability Index variants (Visual Studio and verifysoft), code smell detection, and a comprehensive multi-metric report.
- **Git-based analysis** — hotspot detection (change frequency × complexity, Thornhill method), code churn (pure change frequency), code ownership / knowledge maps via `git blame`, temporal coupling between files that change together, per-author ownership summary, and file age classification (Active / Stale / Frozen).
- **AI-powered analysis** — optional integration with Claude to run all tools and produce a narrative report.

## Installation

```bash
cargo install --path .
```

This installs the `km` binary.

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

Options:

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Show summary stats (files read, unique, ignored, elapsed time) |
| `--by-author` | Break down lines of code by git author (requires a git repository) |
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

```bash
km dups [path]
```

Options:

| Flag | Description |
|------|-------------|
| `-r`, `--report` | Show detailed report with duplicate locations and code samples |
| `--show-all` | Show all duplicate groups (default: top 20) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |
| `--include-tests` | Include test files in analysis (excluded by default) |
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

- **Directories**: `tests/`, `test/`, `__tests__/`, `spec/`
- **By extension**: `*_test.rs`, `*_test.go`, `test_*.py`, `*.test.js`, `*.spec.ts`, `*Test.java`, `*_test.cpp`, and more

Use `--include-tests` to analyze test files as well.

### `km indent` -- Indentation complexity

Measures indentation-based complexity per file: standard deviation of indentation depths and maximum depth. Higher stddev suggests more complex control flow.

```bash
km indent [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include test files in analysis (excluded by default) |

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

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
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

### `km cycom` -- Cyclomatic complexity

Computes cyclomatic complexity per file and per function by counting decision points (`if`, `for`, `while`, `match`, `&&`, `||`, etc.).

```bash
km cycom [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse,github,codeclimate}` | Output format (default: table). `github` emits GitHub Actions annotations; `codeclimate` (alias: `gitlab`) emits CodeClimate JSON for GitLab Code Quality |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
| `--min-complexity N` | Minimum max-complexity to include a file (default: 1) |
| `--per-function` | Show per-function breakdown |

### `km cogcom` -- Cognitive complexity

Computes cognitive complexity per file and per function using the [SonarSource method](https://www.sonarsource.com/docs/CognitiveComplexity.pdf) (2017). Unlike cyclomatic complexity, cognitive complexity measures how difficult code is to *understand*, penalizing deeply nested structures and rewarding linear control flow.

```bash
km cogcom [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse,github,codeclimate}` | Output format (default: table). `github` emits GitHub Actions annotations; `codeclimate` (alias: `gitlab`) emits CodeClimate JSON for GitLab Code Quality |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
| `--min-complexity N` | Minimum max-complexity to include a file (default: 1) |
| `--per-function` | Show per-function breakdown |
| `--sort-by METRIC` | Sort by `total`, `max`, or `avg` (default: `total`) |

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

### `km knowledge` -- Code ownership analysis

Analyzes code ownership patterns via git blame (knowledge maps). Based on Adam Thornhill's method ("Your Code as a Crime Scene" chapters 8-9).

```bash
km knowledge [path]
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
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
| `--sort-by METRIC` | Sort by `concentration`, `diffusion`, or `risk` (default: `concentration`) |
| `--since DURATION` | Define recent activity window for knowledge loss (e.g. `6m`, `1y`, `30d`) |
| `--risk-only` | Show only files with knowledge loss risk |
| `--summary` | Aggregate by author: files owned, lines, languages, worst risk |
| `--bus-factor` | Show project bus factor (minimum contributors covering 80% of code) |
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

Use `--bus-factor` to compute how many contributors you can afford to lose:

```
$ km knowledge --bus-factor
Project Bus Factor: 2

 Losing 2 key contributors would put 80% of the project's knowledge at risk.
 Risk: HIGH — two people hold critical knowledge

──────────────────────────────────────────────
 Rank  Author        Lines    Share  Cumulative
──────────────────────────────────────────────
    1  E. Diaz        8420   68.12%     68.12%
    2  A. Torres      1490   12.06%     80.18%  ← 80% threshold
    3  R. Soto         940    7.61%     87.79%
──────────────────────────────────────────────
```

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

### `km churn` -- Code churn analysis

Measures pure change frequency per file from git history (commit count only, no complexity weight). Identifies the most frequently modified files — high churn without a corresponding quality improvement is a maintenance signal.

```bash
km churn [path]
```

Options:

| Flag | Description |
|------|-------------|
| `--top N` | Show only the top N files (default: 20) |
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
| `magic_number` | Bare numeric literals in code (excluding 0, 1, 2, -1 and `const`/`let` declarations) |
| `commented_code` | Two or more consecutive comment lines containing code-like patterns |

Options:

| Flag | Description |
|------|-------------|
| `--top N` | Show only the top N files by smell count (default: 20) |
| `--max-lines N` | Maximum function body lines before flagging (default: 50) |
| `--max-params N` | Maximum parameter count before flagging (default: 4) |
| `--files FILE` | Analyze only these specific files (repeatable). Useful for scripting |
| `--since-ref REF` | Analyze only files changed since this git ref (e.g. `origin/main`, `HEAD~1`). Ideal for CI |
| `--format {table,json,short,terse,github,codeclimate}` | Output format (default: table). `github` emits GitHub Actions annotations; `codeclimate` (alias: `gitlab`) emits CodeClimate JSON for GitLab Code Quality |

Example output:

```
Code Smells
──────────────────────────────────────────────────────────────────────────────
 File                            Smells  Top Smell
──────────────────────────────────────────────────────────────────────────────
 src/loc/counter.rs                  12  magic_number (7)
 src/main.rs                          6  todo_debt (4)
 src/dups/detector.rs                 3  long_function (2)
──────────────────────────────────────────────────────────────────────────────
 Total (3 files)                     21
```

### `km deps` -- Dependency graph analysis

Analyzes internal module dependencies by parsing import/use/require statements. Builds a directed graph of file-level coupling and detects cycles using Tarjan's SCC algorithm.

```bash
km deps [path]
```

Supports Rust (`mod X;`), Python (relative `from .X import`), JavaScript/TypeScript (relative `import`/`require`), and Go (imports matching the module path from `go.mod`). External dependencies (crates, npm packages) are ignored.

| Flag | Description |
|------|-------------|
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--cycles-only` | Show only files that participate in a dependency cycle |
| `--sort-by METRIC` | Sort by `fan-out` (default) or `fan-in` |
| `--top N` | Show only top N files (default: 20) |

Example output:

```
Dependency Graph
────────────────────────────────────────────────────────────────────────
 File                    Language Fan-In Fan-Out Cycle
────────────────────────────────────────────────────────────────────────
 main.rs                     Rust      0      26    no
 score/mod.rs                Rust      1       7    no
 report/mod.rs               Rust      1       5    no
────────────────────────────────────────────────────────────────────────
No dependency cycles detected.
```

### `km authors` -- Per-author ownership summary

Summarizes code ownership across the project by author. Aggregates `git blame` data to answer "who knows what?" at the team level — complementing `km knowledge` (per-file view) with a team-level view.

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

> **Breaking change in v0.14:** The default scoring model changed from MI + Cyclomatic Complexity (6 dimensions) to Cognitive Complexity (5 dimensions). Use `--model legacy` to restore v0.13 behavior.

Non-code files (Markdown, TOML, JSON, etc.) are automatically excluded. Inline test blocks (`#[cfg(test)]`) are excluded from duplication analysis.

```bash
km score [path]
km score --model legacy [path]    # v0.13 scoring model
```

#### Dimensions and weights (default: cogcom)

| Dimension | Weight | What it measures |
|-----------|--------|-----------------|
| Cognitive Complexity | 30% | SonarSource method, penalizes nesting |
| Duplication | 20% | Project-wide duplicate code % |
| Indentation Complexity | 15% | Stddev of indentation depth |
| Halstead Effort | 20% | Mental effort per LOC |
| File Size | 15% | Optimal range 50-300 LOC |

#### Dimensions and weights (--model legacy)

| Dimension | Weight | What it measures |
|-----------|--------|-----------------|
| Maintainability Index | 30% | Verifysoft MI, normalized to 0-100 |
| Cyclomatic Complexity | 20% | Max complexity per file |
| Duplication | 15% | Project-wide duplicate code % |
| Indentation Complexity | 15% | Stddev of indentation depth |
| Halstead Effort | 15% | Mental effort per LOC |
| File Size | 5% | Optimal range 50-300 LOC |

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
| `--model MODEL` | Scoring model: `cogcom` (default, v0.14+) or `legacy` (MI + cyclomatic, v0.13) |
| `--trend [REF]` | Compare current score against a git ref (default: `HEAD`). Shows change: `B- → B (+2.3)`. Useful for PR review: `--trend origin/main` |
| `--format {table,json,short,terse}` | Output format (default: table) |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--bottom N` | Number of worst files to show in "needs attention" (default: 10) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |

Example output:

```
Code Health Score
──────────────────────────────────────────────────────────────────
 Project Score:  B+ (84.3)
 Files Analyzed: 42
 Total LOC:      8,432
──────────────────────────────────────────────────────────────────
 Dimension                 Weight   Score   Grade
──────────────────────────────────────────────────────────────────
 Cognitive Complexity         30%    85.6   B+
 Duplication                  20%    91.3   A
 Indentation Complexity       15%    79.8   B-
 Halstead Effort              20%    85.1   B+
 File Size                    15%    89.2   A-
──────────────────────────────────────────────────────────────────

 Files Needing Attention (worst scores)
──────────────────────────────────────────────────────────────────
 Score  Grade  File                       Issues
──────────────────────────────────────────────────────────────────
  54.2  F      src/legacy/parser.rs       Cognitive: 42, Indent: 3.2
  63.7  D+     src/utils/helpers.rs       Effort: 15200, Indent: 2.4
  68.9  C-     src/core/engine.rs         Size: 1243 LOC
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
| `--model MODEL` | Scoring model: `cogcom` (default) or `legacy` |
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

Place a `.kimun.toml` file in the root of your repository to set project-level defaults for thresholds and quality gates. `km` searches for the file at the git repository root, falling back to the current directory.

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
model      = "cogcom"  # scoring model: cogcom (default) or legacy
fail_below = "B-"      # CI gate: fail if health score is below this grade

[age]
active_days = 60    # files modified within N days are Active (default: 90)
frozen_days = 180   # files not modified for more than N days are Frozen (default: 365)

[tc]
min_degree   = 5    # minimum commits per file to include in coupling analysis (default: 3)
min_strength = 0.5  # only show pairs with coupling strength >= this value

[hotspots]
complexity = "cogcom"  # complexity metric: indent (default), cycom, or cogcom
```

All sections and fields are optional — omit any you don't need. A fully documented template is available at [`.kimun.toml.example`](.kimun.toml.example).

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

## License

See [Cargo.toml](Cargo.toml) for package details.
