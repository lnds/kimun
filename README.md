# Kimün (km)

> *Kimün* means "knowledge" or "wisdom" in Mapudungun, the language of the Mapuche people.

A fast command-line tool for code analysis, written in Rust. Run `km score` on any project to get an overall health grade (A++ to F--) across five quality dimensions — cognitive complexity, duplication, indentation depth, Halstead effort, and file size — with a list of the files that need the most attention.

> Note: This repository is not the same as the Rust console note-taking app `kimun` by nico2sh. That project is available at https://github.com/nico2sh/kimun.

Beyond the aggregate score, Kimün provides 17 specialized commands:

- **Static metrics** — lines of code by language ([cloc](https://github.com/AlDanial/cloc)-compatible), duplicate detection (Rule of Three), Halstead complexity, cyclomatic complexity, cognitive complexity (SonarSource), indentation complexity, two Maintainability Index variants (Visual Studio and verifysoft), code smell detection, and a comprehensive multi-metric report.
- **Git-based analysis** — hotspot detection (change frequency × complexity, Thornhill method), code churn (pure change frequency), code ownership / knowledge maps via `git blame`, temporal coupling between files that change together, impact of a diff (diffusion and missing co-changes), per-author ownership summary, and file age classification (Active / Stale / Frozen).
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
| `--min-complexity N` | Skip files whose most complex function is below N; with `--per-function`, also hide functions below N (default: 1) |
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
| `--min-complexity N` | Skip files whose most complex function is below N; with `--per-function`, also hide functions below N (default: 1) |
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

- When the repository has the commits of the pull request, it is measured from them, as between two refs. A pull request merged by squash or rebase, whose head was never fetched, is measured from its base up to the commit that merged it.
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
- **Workspace roots**: a `Cargo.toml` with `[workspace]`, a `package.json` with `workspaces` (or next to a `pnpm-workspace.yaml`), an umbrella `mix.exs` with `apps_path`, a `go.work`. A change to the manifest or the lock file at such a root (`Cargo.lock`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `bun.lock`, `mix.lock`) reaches every project under it at distance 1, with scope `workspace`, and carries on from them. A workspace root is a project itself only when it declares one (`[package]` in Cargo, `app:` in mix); a `package.json` at a workspace root never is.
- **Inert files** reach nothing: a change to documentation (`.md`, `.mdx`, `.rst`, `.adoc`, `.txt`) breaks no build and no test. It does not count as a change to its project, nor as a file of unknown reach. `.kimun.toml` can declare more:

  ```toml
  [impact]
  inert = ["scripts/**", "notebooks/**"]   # globs, relative to the repository
  ```

- **Files outside every project** (CI workflows, shared configuration) are listed apart. Their reach is unknown, not zero.

- **`--affected`** prints the projects whose builds and tests the diff calls for, one per line, and nothing else: the changed and the reached ones. If any changed file is outside every project, it prints **all** projects and says why on stderr — skipping a test suite is worse than running one too many. It reads only the diff and the manifests, not the history.
- A repository with a single project gets a line saying this level does not apply, rather than "0 reached".
- `node_modules`, `deps`, `_build`, `target`, `vendor` and `testdata` are never searched for manifests, nor is a `fixtures` directory inside a test directory. An end-to-end suite with its own manifest (`test/e2e/package.json`) is a project.

Limits:

- `mix.exs` is code and is read as text. A dependency whose path is built at run time (`Path.expand(...)`, string interpolation, a generated list) cannot be attributed; the report names the manifest and how many it missed.
- Coupling across ecosystems is not visible: a web client and the service whose API it calls have no manifest dependency between them.
- A project nested in another (`assets/package.json` inside a Phoenix application) has no dependency to or from the one that contains it unless a manifest declares one.
- The graph is read from the working tree. The files of a project the diff deletes or moves away belong to no project any more: their reach is unknown, and the manifests still naming it are reported as not read.

#### Blast radius: source files

Inside the projects the change affects: which source files use what changed, and which of them no test exercises. This is the question "the tests of the module I changed pass; who else calls it?".

The first line is the answer in short: how many files call what changed, and how many of them have no test. `Radius` is the count of files the change concerns, directly and through them. `Upper bound` is what it would be without knowing which functions changed.

```
Structural radius — source files that use what changed
──────────────────────────────────────────────────────────────────────────────
 1 file calls what changed, 1 of them with no test
 Changed: lib/booking/insights.ex
 Functions: arrange
 Radius: 2 of 6 source files (33%): 2 at distance 1
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

- The dependency graph of `km deps` is read backwards from the changed source files.
- In Elixir the change is **narrowed to functions**: the lines the diff touches tell which functions changed, and a change to a private function is carried to the public ones that reach it through local calls. A file that uses the module is then one of three: it **calls** a function that changed, it **refers** to the module without calling it (a struct, an `import`, a `use`), or it only calls functions the change leaves alone, and is not listed. Each changed file is narrowed on its own. Outside the functions, a touched `alias` or `require` changes none (it only names what the touched functions use), and a touched module attribute changes the functions that read it. A `use`, an `import`, a `defstruct`, or an attribute no function reads may concern every function: that file is not narrowed, every use of it counts, and the report says which line it was. A new file is never narrowed.
- The **radius** starts at the files the change concerns and follows who uses them, file by file. Files that only pass through a dependent the change leaves alone are not counted. The **upper bound** is what the radius would be if every use of a changed file counted, whatever the function: in a codebase where everything goes through a few contexts it is most of the project, which is why the radius is the number to read.
- **Tests** says how the dependent is protected. A test rarely names everything it exercises — a controller or a live view is tested through its route, a helper through the views that use it — so protection comes in degrees:

  | `Tests` | Protection | When |
  |---------|------------|------|
  | a number | direct | That many test files refer to it, or sit at the same place in the source and test layout (`lib/a/b.ex` and `test/a/b_test.exs`) |
  | `named` | named | A test carries its name a directory apart (`live/page_live.ex` and `page_live_test.exs`), or the name of the directory it is in (`page_live/index.ex` and `page_live_test.exs`) |
  | `users` | users | It has no test of its own, but a file that uses it has one |
  | `none` | none | No test reaches it |

  Test support (`test/support/`), configuration and scripts are not tests: a factory refers to everything and would make everything look protected.
- A file that calls what changed and that no test reaches is **unprotected**: the change can break it without any test noticing. That is the warning. Dependents are listed from the least protected.


It is measured for the languages whose graph reflects usage: Elixir, JavaScript/TypeScript and Kaikai. For Rust, Python and Go the block says it is not available rather than report a radius drawn on declarations. Only the projects the change affects are read; the whole repository when the reach over projects is unknown.

Limits: a test at the same place, or named after a file, may not exercise the call that changed, and one that refers to a file may mock what it calls — the column says a test exists, not that it covers. `users` is weaker still: it says something that uses the file is tested. Function names are compared without arity. Modules named at run time (`apply/3`, configuration), generated by macros, or aliased by a Phoenix router `scope` are not seen.

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
| `structural.functions` | Public functions the change affects, over the files that tell them; `null` when none could be narrowed |
| `structural.narrowing[]` | Per changed file: `functions`, or `null` with the `reason` every use of it counts |
| `structural.direct[]` | Every file that uses a changed file: `file`, `exposure` (`calls`, `refers`, `elsewhere`), `protection` (`direct`, `named`, `users`, `none`), `calls`, `tests`, `tests_in_diff` |
| `structural.unprotected[]` | Files that call what changed and that no test reaches, not even through what uses them — where an integration test is missing |
| `structural.unknown_without_tests[]` | Files that use the changed module without a call that tells, and have no test |
| `structural.change_tests_a_dependent` | Whether a test in the change protects a file that uses what changed |
| `structural.radius` | `files`, `source_files` and `share` (0 to 1): the radius as a number |
| `structural.reach[]` | The files of the radius, by `distance` |
| `structural.upper_bound` | Files reached if every use of a changed file counted, whatever the function |
| `structural.unavailable[]` | Languages of changed files the source level is not measured for |
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

Supports Elixir (every module referred to in the code, with `alias` undone; see the notes below), Rust (`mod X;`, with any visibility qualifier: `pub`, `pub(crate)`, `pub(in path)`), Python (relative `from .X import`), JavaScript/TypeScript (relative `import`/`require`), Go (imports matching the module path from `go.mod`), and Kaikai (`import a.b.c`, including the `as` and `.{…}` forms). External dependencies (crates, npm packages, the Kaikai stdlib) are ignored.

Files in any other language are left out of the graph instead of being listed with zero dependencies. The table footer, the `unsupported` array of the JSON output and the `unsupported:N` field of the short format say how many files were skipped, so "not measured" is never shown as "no dependencies".

Elixir notes:

- Dependencies are between modules, and most need no import, so every module name in the code counts as a reference: calls, structs, `use`, `import`, `require`, `@behaviour`, `defimpl`. Comments, strings, heredocs and sigils are set aside first, so a doctest is not a dependency.
- `alias` is undone, including `as:`, grouped and multi-line forms, and `__MODULE__`. The statement itself is not a use.
- Nested `defmodule`s are named after the module that contains them, told by indentation as `mix format` leaves it.
- A module defined in several files (projects made from one template) resolves to the file nearest the one referring to it.
- Not seen: modules named at run time (`apply/3`, configuration), those generated by macros, and the alias a Phoenix router gives its `scope`.

Kaikai notes:

- `import a.b.c` names `a/b/c.kai` relative to a package root, not to the importing file. Each ancestor directory of the importing file is tried, nearest first, so run `km deps` on a directory that contains the package root.
- When no such file exists, `import a` can name a package directory `a/` that carries a `kai.toml`; the import then depends on every `.kai` file directly inside it.
- An import that resolves to no analysed file is external and adds no edge.
- The files of one Kaikai package merge, so a module can use a name declared in another file of its package without importing it. The import graph is therefore a lower bound on the real dependencies.

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
| `--fail-if-worse` | With `--trend`: exit with code 1 if the score dropped by more than `--gate-tolerance` |
| `--gate-tolerance POINTS` | Score drop `--fail-if-worse` allows before failing (default: `0.01` with `--gate-scope project`, `0.5` with `--gate-scope changed`). Compares unrounded scores |
| `--gate-scope {project,changed}` | What `--fail-if-worse` compares (default: `project`, the aggregate score). `changed` looks only at the files the diff touches: it fails if a modified or renamed file ends below the project score at the ref after dropping more than `--gate-tolerance`, or if the project's duplicated lines grow. Files above the project score, new files and deleted files never fail it, so removing healthy code cannot lower the verdict. The report lists every changed file with its before/after score |
| `--fail-below GRADE` | With `--trend`: exit with code 1 if the score is below `GRADE` (e.g. `B-`). Overridable via `.kimun.toml` |
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

[impact]
inert = ["scripts/**"]  # changed files that reach nothing, besides documentation
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
| Kaikai | `.kai` |
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

### Language-specific notes

- **Kaikai** — `#[...]` opens an attribute, not a `#` comment, so attributes count
  as code. Documentation attributes (`#[doc("...")]`, including the multi-line
  `#[doc("""...""")]` form) count as comments and are excluded from the
  complexity, Halstead, and smell analyses.
- **Interior lines of multi-line strings** (Kaikai and Python triple-quoted
  literals) count as code for `km loc`, but are excluded from cyclomatic,
  cognitive, Halstead, and smell analyses: prose and embedded data are not
  control flow.

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

## License

See [Cargo.toml](Cargo.toml) for package details.
