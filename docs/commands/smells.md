# `km smells` — Code smell detection

Detects common code quality issues per file using text-based heuristics (no AST required). Only languages with complexity marker support are analyzed (same set as `km cycom`: Rust, Python, JS/TS, C/C++, Go, etc.).

```bash
km smells [path]
```

## Smell types

| Smell | Description |
|-------|-------------|
| `long_function` | Function body exceeds `--max-lines` (default: 50) |
| `long_params` | Function has more than `--max-params` parameters (default: 4) |
| `todo_debt` | TODO, FIXME, HACK, XXX, or BUG in comment lines |
| `magic_number` | Bare numeric literals in code (excluding 0, 1, 2, -1 and `const`/`let` declarations) |
| `commented_code` | Two or more consecutive comment lines containing code-like patterns |

## Options

| Flag | Description |
|------|-------------|
| `--top N` | Show only the top N files by smell count (default: 20) |
| `--max-lines N` | Maximum function body lines before flagging (default: 50) |
| `--max-params N` | Maximum parameter count before flagging (default: 4) |
| `--files FILE` | Analyze only these specific files (repeatable). Useful for scripting |
| `--since-ref REF` | Analyze only files changed since this git ref (e.g. `origin/main`, `HEAD~1`). Ideal for CI |
| `--json` | Output as JSON |

## Example output

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
