# `km hal` — Halstead complexity metrics

Computes [Halstead complexity metrics](https://en.wikipedia.org/wiki/Halstead_complexity_measures) per file by extracting operators and operands from source code.

```bash
km hal [path]
```

## Metrics

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

## Options

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON (includes all metrics) |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--top N` | Show only the top N files (default: 20) |
| `--sort-by METRIC` | Sort by `effort`, `volume`, or `bugs` (default: `effort`) |

## Example output

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

## Supported languages

Rust, Python, JavaScript, TypeScript, Go, C, C++, C#, Java, Objective-C, PHP, Dart, Ruby, Kotlin, Swift, Shell (Bash/Zsh).
