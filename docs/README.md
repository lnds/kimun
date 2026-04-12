# Kimün (km)

> *Kimün* means "knowledge" or "wisdom" in Mapudungun, the language of the Mapuche people.

A fast command-line tool for code analysis, written in Rust.

Run `km score` on any project to get an overall health grade (A++ to F--) across five quality dimensions — cognitive complexity, duplication, indentation depth, Halstead effort, and file size.

## Installation

```bash
cargo install --path .
```

### Shell completions

```bash
# zsh
km completions zsh > ~/.zfunc/_km

# bash
km completions bash > /etc/bash_completion.d/km

# fish
km completions fish > ~/.config/fish/completions/km.fish
```

## Quick start

```bash
km score          # overall health grade for the current directory
km loc            # lines of code by language
km hotspots       # files that are both complex and frequently changed
km knowledge      # who owns what (bus factor)
```

Use the sidebar to browse all commands.
