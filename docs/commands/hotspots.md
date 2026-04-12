# `km hotspots` — Hotspot analysis

Finds hotspots: files that change frequently AND have high complexity. Based on Adam Thornhill's method ("Your Code as a Crime Scene").

```bash
km hotspots [path]
```

## Formula

```
Score = Commits × Complexity
```

Files with high scores concentrate risk — they are both change-prone and complex, making them the highest-value refactoring targets.

By default, complexity is measured by **total indentation** (sum of logical indentation levels across all code lines), following Thornhill's original method from "Your Code as a Crime Scene". Use `--complexity cycom` for cyclomatic complexity instead.

Requires a git repository. Merge commits are excluded from the count.

## Options

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
| `--sort-by METRIC` | Sort by `score`, `commits`, or `complexity` (default: `score`) |
| `--since DURATION` | Only consider commits since this time (e.g. `30d`, `6m`, `1y`) |
| `--complexity METRIC` | `indent` (default, Thornhill) or `cycom` (cyclomatic) |

Duration units: `d` (days), `m` (months, approx. 30 days), `y` (years, approx. 365 days).

## Example output

Default — indentation complexity:

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

With `--complexity cycom`:

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
