# `km deps` — Dependency graph analysis

Analyzes internal module dependencies by parsing import/use/require statements. Builds a directed graph of file-level coupling and detects cycles using Tarjan's SCC algorithm.

```bash
km deps [path]
```

Supports Rust (`mod X;`), Python (relative `from .X import`), JavaScript/TypeScript (relative `import`/`require`), and Go (imports matching the module path from `go.mod`). External dependencies (crates, npm packages) are ignored.

## Options

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--cycles-only` | Show only files that participate in a dependency cycle |
| `--sort-by METRIC` | Sort by `fan-out` (default) or `fan-in` |
| `--top N` | Show only top N files (default: 20) |

## Example output

```
Dependency Graph
────────────────────────────────────────────────────────────────────────
 File                    Language Fan-In Fan-Out Cycle
────────────────────────────────────────────────────────────────────────
 main.rs                     Rust      0      26    no
 score/mod.rs                Rust      1       7    no
 report/mod.rs               Rust      1       5    no
────────────────────────────────────────────────────────────────────────
No dependency cycles detected.
```
