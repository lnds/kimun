# cm (code-metrics)

A fast command-line tool for counting lines of code by language, written in Rust. Inspired by [cloc](https://github.com/AlDanial/cloc).

## Installation

```bash
cargo install --path .
```

This installs the `cm` binary.

## Usage

```bash
cm loc [path]
```

Run on the current directory:

```bash
cm loc
```

Run on a specific path:

```bash
cm loc src/
```

Example output:

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

## Features

- Respects `.gitignore` rules automatically
- Deduplicates files by content hash (identical files counted once)
- Detects languages by file extension, filename, or shebang line
- Supports nested block comments (Rust, Haskell, OCaml, etc.)
- Handles pragmas (e.g., Haskell `{-# LANGUAGE ... #-}`) as code
- Mixed lines (code + comment) are counted as code, matching `cloc` behavior

## Supported Languages

| Language | Extensions / Filenames |
|---|---|
| Bourne Again Shell | `.bash` |
| Bourne Shell | `.sh` |
| C | `.c`, `.h` |
| C# | `.cs` |
| C++ | `.cpp`, `.cxx`, `.cc`, `.hpp`, `.hxx` |
| Clojure | `.clj`, `.cljs`, `.cljc`, `.edn` |
| CSS | `.css` |
| Dart | `.dart` |
| Dockerfile | `Dockerfile` |
| DOS Batch | `.bat`, `.cmd` |
| Elixir | `.ex` |
| Elixir Script | `.exs` |
| Erlang | `.erl`, `.hrl` |
| F# | `.fs`, `.fsi`, `.fsx` |
| Go | `.go` |
| Gradle | `.gradle` |
| Groovy | `.groovy` |
| Haskell | `.hs` |
| HTML | `.html`, `.htm` |
| Java | `.java` |
| JavaScript | `.js`, `.mjs`, `.cjs` |
| JSON | `.json` |
| Julia | `.jl` |
| Kotlin | `.kt`, `.kts` |
| Lua | `.lua` |
| Makefile | `.mk`, `Makefile`, `makefile`, `GNUmakefile` |
| Markdown | `.md`, `.markdown` |
| Nim | `.nim` |
| Objective-C | `.m`, `.mm` |
| OCaml | `.ml`, `.mli` |
| Perl | `.pl`, `.pm` |
| PHP | `.php` |
| Properties | `.properties` |
| Python | `.py`, `.pyi` |
| R | `.r`, `.R` |
| Ruby | `.rb`, `Rakefile`, `Gemfile` |
| Rust | `.rs` |
| Scala | `.scala`, `.sc`, `.sbt` |
| SQL | `.sql` |
| Swift | `.swift` |
| Terraform | `.tf` |
| Text | `.txt` |
| TOML | `.toml` |
| TypeScript | `.ts`, `.mts`, `.cts` |
| XML | `.xml`, `.xsl`, `.xslt`, `.svg`, `.fsproj`, `.csproj`, `.vbproj`, `.vcxproj`, `.sln`, `.plist`, `.xaml` |
| YAML | `.yaml`, `.yml` |
| Zig | `.zig` |
| Zsh | `.zsh` |

## Development

```bash
cargo build              # build debug binary
cargo test               # run all tests
cargo clippy             # lint (zero warnings required)
cargo tarpaulin --out stdout  # coverage report
```

## License

See [Cargo.toml](Cargo.toml) for package details.
