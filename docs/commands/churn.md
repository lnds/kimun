# `km churn` — Code churn analysis

Measures pure change frequency per file from git history (commit count only, no complexity weight). Identifies the most frequently modified files — high churn without a corresponding quality improvement is a maintenance signal.

```bash
km churn [path]
```

## Options

| Flag | Description |
|------|-------------|
| `--top N` | Show only the top N files (default: 20) |
| `--sort-by METRIC` | Sort by `commits` (default), `rate` (commits/month), or `file` |
| `--since DURATION` | Only consider commits since this time (e.g. `6m`, `1y`, `30d`) |
| `--json` | Output as JSON |

## Example output

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
