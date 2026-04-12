# `km dups` — Detect duplicate code

Finds duplicate code blocks across files using a sliding window approach. Applies the **Rule of Three**: duplicates appearing 3+ times are marked as **CRITICAL** (refactor recommended), while those appearing twice are **TOLERABLE**.

Test files and directories are excluded by default, since tests often contain intentional repetition.

```bash
km dups [path]
```

## Options

| Flag | Description |
|------|-------------|
| `-r`, `--report` | Show detailed report with duplicate locations and code samples |
| `--show-all` | Show all duplicate groups (default: top 20) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--max-duplicates N` | Exit with code 1 if duplicate groups exceed this limit (`--max-duplicates 0` fails on any duplicate) |
| `--max-dup-ratio PERCENT` | Exit with code 1 if the duplicated-lines ratio exceeds this percentage (e.g. `--max-dup-ratio 5.0`) |
| `--fail-on-increase REF` | Exit with code 1 if the current duplication ratio is higher than at the given git ref (e.g. `origin/main`). Prevents debt from growing silently in CI |
| `--json` | Output as JSON |

## Example output

Summary:

```
────────────────────────────────────────────────────────────────────
 Duplication Analysis

 Total code lines:                                             3247
 Duplicated lines:                                              156
 Duplication:                                                  4.8%

 Duplicate groups:                                               12
 Files with duplicates:                                           8
 Largest duplicate:                                        18 lines

 Rule of Three Analysis:
   Critical duplicates (3+):     7 groups,    96 lines
   Tolerable duplicates (2x):    5 groups,    60 lines

 Assessment:                                                    Good
────────────────────────────────────────────────────────────────────
```

Detailed (`--report`):

```
────────────────────────────────────────────────────────────────────
 [1] CRITICAL: 18 lines, 3 occurrences (36 duplicated lines)

   src/parser.rs:45-62
   src/formatter.rs:120-137
   src/validator.rs:89-106

 Sample:
   fn process_tokens(input: &str) -> Vec<Token> {
       let mut tokens = Vec::new();
       for line in input.lines() {
       ...

────────────────────────────────────────────────────────────────────
 [2] TOLERABLE: 12 lines, 2 occurrences (12 duplicated lines)

   src/main.rs:100-111
   src/cli.rs:200-211

 Sample:
   match result {
       Ok(value) => {
       ...
────────────────────────────────────────────────────────────────────
```

## Excluded test patterns

By default, `km dups` skips files matching common test conventions:

- **Directories**: `tests/`, `test/`, `__tests__/`, `spec/`
- **By extension**: `*_test.rs`, `*_test.go`, `test_*.py`, `*.test.js`, `*.spec.ts`, `*Test.java`, `*_test.cpp`, and more

Use `--include-tests` to analyze test files as well.
