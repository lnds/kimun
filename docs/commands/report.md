# `km report` — Comprehensive metrics report

Generates a multi-section report combining all static code metrics in a single pass: lines of code, duplicates, indentation, Halstead, cyclomatic complexity, cognitive complexity, and maintainability index.

```bash
km report [path]
```

## Options

| Flag | Description |
|------|-------------|
| `--top N` | Show only the top N files per section (default: 20) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |
| `--full` | Show all files instead of truncating to top N |
| `--json` | Output as JSON |
