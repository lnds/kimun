# `km loc` — Count lines of code

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

## Options

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Show summary stats (files read, unique, ignored, elapsed time) |
| `--by-author` | Break down lines of code by git author (requires a git repository) |
| `--json` | Output as JSON |

## Example output

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
