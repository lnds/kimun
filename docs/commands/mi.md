# `km mi` — Maintainability Index (Visual Studio variant)

Computes the [Maintainability Index](https://learn.microsoft.com/en-us/visualstudio/code-quality/code-metrics-maintainability-index-range-and-meaning) per file using the Visual Studio formula. MI is normalized to a 0–100 scale with no comment-weight term.

```bash
km mi [path]
```

## Formula

```
MI = MAX(0, (171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)) * 100 / 171)
```

Where V = Halstead Volume, G = cyclomatic complexity, LOC = code lines.

## Thresholds

| MI Score | Level | Meaning |
|----------|-------|---------|
| 20–100 | green | Good maintainability |
| 10–19 | yellow | Moderate maintainability |
| 0–9 | red | Low maintainability |

## Options

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
| `--sort-by METRIC` | Sort by `mi` (ascending), `volume`, `complexity`, or `loc` (default: `mi`) |

## Example output

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
