# `km authors` — Per-author ownership summary

Summarizes code ownership across the project by author. Aggregates `git blame` data to answer "who knows what?" at the team level — complementing `km knowledge` (per-file view) with a team-level view.

```bash
km authors [path]
```

## Options

| Flag | Description |
|------|-------------|
| `--since DURATION` | Only consider activity since this time (e.g. `6m`, `1y`, `30d`) |
| `--json` | Output as JSON |

## Example output

```
──────────────────────────────────────────────────────────────────────
 Author              Owned      Lines  Languages    Last Active
──────────────────────────────────────────────────────────────────────
 E. Diaz                38       8432  Rust, TOML   2026-03-15
 R. Ramirez              4        312  Rust         2026-02-10
──────────────────────────────────────────────────────────────────────
```
