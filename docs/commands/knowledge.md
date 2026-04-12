# `km knowledge` — Code ownership analysis

Analyzes code ownership patterns via git blame (knowledge maps). Based on Adam Thornhill's method ("Your Code as a Crime Scene" chapters 8-9).

```bash
km knowledge [path]
```

Identifies bus factor risk and knowledge concentration per file. Generated files (lock files, minified JS, etc.) are automatically excluded.

## Risk levels

| Risk | Condition | Meaning |
|------|-----------|---------|
| CRITICAL | 1 person owns >80% | High bus factor risk |
| HIGH | 1 person owns 60-80% | Significant concentration |
| MEDIUM | 2-3 people own >80% combined | Moderate concentration |
| LOW | Well-distributed | Healthy ownership |

## Knowledge loss detection

Use `--since` to define "recent activity". If the primary owner of a file has no commits in that period, the file is flagged with **knowledge loss** risk. Use `--risk-only` to show only those files.

## Options

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
| `--sort-by METRIC` | Sort by `concentration`, `diffusion`, or `risk` (default: `concentration`) |
| `--since DURATION` | Define recent activity window for knowledge loss (e.g. `6m`, `1y`, `30d`) |
| `--risk-only` | Show only files with knowledge loss risk |
| `--summary` | Aggregate by author: files owned, lines, languages, worst risk |
| `--bus-factor` | Show project bus factor (minimum contributors covering 80% of code) |
| `--author NAME` | Show only files owned by this author (case-insensitive substring match) |

## Example output

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
