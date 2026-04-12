# Kimün (km)

> *Kimün* means "knowledge" or "wisdom" in Mapudungun, the language of the Mapuche people.

A fast command-line tool for code analysis, written in Rust. Run `km score` on any project to get an overall health grade (A++ to F--) across five quality dimensions — cognitive complexity, duplication, indentation depth, Halstead effort, and file size — with a list of the files that need the most attention.

Beyond the aggregate score, Kimün provides 18 specialized commands:

- **Static metrics** — lines of code by language ([cloc](https://github.com/AlDanial/cloc)-compatible), duplicate detection (Rule of Three), Halstead complexity, cyclomatic complexity, cognitive complexity (SonarSource), indentation complexity, two Maintainability Index variants (Visual Studio and verifysoft), code smell detection, dependency graph analysis, and a comprehensive multi-metric report.
- **Git-based analysis** — hotspot detection (change frequency × complexity, Thornhill method), code churn (pure change frequency), code ownership / knowledge maps via `git blame`, temporal coupling between files that change together, per-author ownership summary, and file age classification (Active / Stale / Frozen).
- **AI-powered analysis** — optional integration with Claude to run all tools and produce a narrative report.

## Installation

```bash
cargo install --path .
```

This installs the `km` binary.

### Shell completions

```bash
# zsh
km completions zsh > ~/.zfunc/_km
# add to ~/.zshrc if not already present:
#   fpath=(~/.zfunc $fpath)
#   autoload -Uz compinit && compinit

# bash
km completions bash > /etc/bash_completion.d/km

# fish
km completions fish > ~/.config/fish/completions/km.fish
```

## Commands

### Static metrics

| Command | Description |
|---------|-------------|
| [`km loc`](docs/commands/loc.md) | Count lines of code by language (cloc-compatible) |
| [`km dups`](docs/commands/dups.md) | Detect duplicate code blocks (Rule of Three) |
| [`km indent`](docs/commands/indent.md) | Indentation complexity (stddev of indent depth) |
| [`km hal`](docs/commands/hal.md) | Halstead complexity metrics |
| [`km cycom`](docs/commands/cycom.md) | Cyclomatic complexity per file and function |
| [`km cogcom`](docs/commands/cogcom.md) | Cognitive complexity (SonarSource method) |
| [`km mi`](docs/commands/mi.md) | Maintainability Index — Visual Studio variant (0–100) |
| [`km miv`](docs/commands/miv.md) | Maintainability Index — verifysoft variant (with comment weight) |
| [`km smells`](docs/commands/smells.md) | Code smell detection (long functions, magic numbers, TODO debt, etc.) |
| [`km deps`](docs/commands/deps.md) | Dependency graph and cycle detection (Tarjan SCC) |
| [`km report`](docs/commands/report.md) | Comprehensive multi-metric report in a single pass |
| [`km score`](docs/commands/score.md) | Overall code health grade (A++ to F--) |

### Git-based analysis

| Command | Description |
|---------|-------------|
| [`km hotspots`](docs/commands/hotspots.md) | Files that change often AND are complex (Thornhill method) |
| [`km churn`](docs/commands/churn.md) | Pure change frequency per file |
| [`km knowledge`](docs/commands/knowledge.md) | Code ownership and bus factor via git blame |
| [`km tc`](docs/commands/tc.md) | Temporal coupling — files that change together |
| [`km authors`](docs/commands/authors.md) | Per-author ownership summary |
| [`km age`](docs/commands/age.md) | File age: Active / Stale / Frozen |

## Features

- Respects `.gitignore` rules automatically
- Deduplicates files by content hash (identical files counted once)
- Detects languages by file extension, filename, or shebang line
- Supports nested block comments (Rust, Haskell, OCaml, etc.)
- Handles pragmas (e.g., Haskell `{-# LANGUAGE ... #-}`) as code
- Mixed lines (code + comment) are counted as code, matching `cloc` behavior

## Documentation

- [Supported languages](docs/languages.md)
- [Architecture](docs/architecture.md)
- [Contributing](docs/contributing.md)
- [References](docs/references.md)

## License

See [Cargo.toml](Cargo.toml) for package details.
