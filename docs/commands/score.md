# `km score` — Code health score

Computes an overall code health score for the project, grading it from A++ (exceptional) to F-- (severe issues). Uses only static metrics (no git required).

> **Breaking change in v0.14:** The default scoring model changed from MI + Cyclomatic Complexity (6 dimensions) to Cognitive Complexity (5 dimensions). Use `--model legacy` to restore v0.13 behavior.

Non-code files (Markdown, TOML, JSON, etc.) are automatically excluded. Inline test blocks (`#[cfg(test)]`) are excluded from duplication analysis.

```bash
km score [path]
km score --model legacy [path]    # v0.13 scoring model
```

## Dimensions and weights (default: cogcom)

| Dimension | Weight | What it measures |
|-----------|--------|-----------------|
| Cognitive Complexity | 30% | SonarSource method, penalizes nesting |
| Duplication | 20% | Project-wide duplicate code % |
| Indentation Complexity | 15% | Stddev of indentation depth |
| Halstead Effort | 20% | Mental effort per LOC |
| File Size | 15% | Optimal range 50-300 LOC |

## Dimensions and weights (--model legacy)

| Dimension | Weight | What it measures |
|-----------|--------|-----------------|
| Maintainability Index | 30% | Verifysoft MI, normalized to 0-100 |
| Cyclomatic Complexity | 20% | Max complexity per file |
| Duplication | 15% | Project-wide duplicate code % |
| Indentation Complexity | 15% | Stddev of indentation depth |
| Halstead Effort | 15% | Mental effort per LOC |
| File Size | 5% | Optimal range 50-300 LOC |

Each dimension is aggregated as a LOC-weighted mean across all files (except Duplication which is a single project-level value). The project score is the weighted sum of all dimension scores.

## Grade scale

| Grade | Score range | Grade | Score range |
|-------|------------|-------|------------|
| A++ | 97-100 | C+ | 73-76 |
| A+ | 93-96 | C | 70-72 |
| A | 90-92 | C- | 67-69 |
| A- | 87-89 | D+ | 63-66 |
| B+ | 83-86 | D | 60-62 |
| B | 80-82 | D- | 57-59 |
| B- | 77-79 | F | 50-56 |
| | | F- | 40-49 |
| | | F-- | 0-39 |

## Options

| Flag | Description |
|------|-------------|
| `--model MODEL` | Scoring model: `cogcom` (default, v0.14+) or `legacy` (MI + cyclomatic, v0.13) |
| `--trend [REF]` | Compare current score against a git ref (default: `HEAD`). Shows change: `B- → B (+2.3)`. Useful for PR review: `--trend origin/main` |
| `--json` | Output as JSON |
| `--include-tests` | Include test files in analysis (excluded by default) |
| `--bottom N` | Number of worst files to show in "needs attention" (default: 10) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |

## Example output

```
Code Health Score
──────────────────────────────────────────────────────────────────
 Project Score:  B+ (84.3)
 Files Analyzed: 42
 Total LOC:      8,432
──────────────────────────────────────────────────────────────────
 Dimension                 Weight   Score   Grade
──────────────────────────────────────────────────────────────────
 Cognitive Complexity         30%    85.6   B+
 Duplication                  20%    91.3   A
 Indentation Complexity       15%    79.8   B-
 Halstead Effort              20%    85.1   B+
 File Size                    15%    89.2   A-
──────────────────────────────────────────────────────────────────

 Files Needing Attention (worst scores)
──────────────────────────────────────────────────────────────────
 Score  Grade  File                       Issues
──────────────────────────────────────────────────────────────────
  54.2  F      src/legacy/parser.rs       Cognitive: 42, Indent: 3.2
  63.7  D+     src/utils/helpers.rs       Effort: 15200, Indent: 2.4
  68.9  C-     src/core/engine.rs         Size: 1243 LOC
──────────────────────────────────────────────────────────────────
```

## `km score diff` — Compare score against a git ref

Extracts the file tree at the given ref, computes the score for both snapshots, and shows a delta table per dimension. Useful for reviewing how commits impact code quality.

```bash
km score diff                          # compare vs HEAD (uncommitted changes)
km score diff --git-ref HEAD~1         # compare vs previous commit
km score diff --git-ref main           # compare vs main branch
km score diff --json                   # machine-readable output
```

### Options

| Flag | Description |
|------|-------------|
| `--git-ref REF` | Git ref to compare against (default: `HEAD`) |
| `--model MODEL` | Scoring model: `cogcom` (default) or `legacy` |
| `--json` | Output as JSON |
| `--bottom N` | Number of worst files to show (default: 10) |
| `--min-lines N` | Minimum lines for a duplicate block (default: 6) |
