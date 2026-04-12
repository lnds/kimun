# `km indent` — Indentation complexity

Measures indentation-based complexity per file: standard deviation of indentation depths and maximum depth. Higher stddev suggests more complex control flow.

```bash
km indent [path]
```

## Options

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
