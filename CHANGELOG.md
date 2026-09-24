## v0.26.0 (2026-09-24)

### Feat

- **score**: add --gate-scope changed to gate on the files a diff touches
  - `km score --trend REF --fail-if-worse --gate-scope changed` looks only at the files the diff touches. It fails when a modified or renamed file ends below the project score at `REF` after dropping more than `--gate-tolerance`, or when the project's duplicated lines grow.
  - Files above the project score, new files and deleted files never fail it, so deleting healthy code no longer fails the gate.
  - The report and JSON list every changed file with its before/after score, and the error names each file responsible.
  - The default scope stays `project` (the aggregate score), so existing pipelines are unchanged.
- **score**: add --gate-tolerance to set the drop --fail-if-worse allows
  - `--gate-tolerance POINTS` sets how many points the score may drop before `--fail-if-worse` fails. Defaults: `0.01` with `--gate-scope project`, `0.5` with `--gate-scope changed`.

### Fix

- **score**: gate on unrounded scores and show the overall at two decimals
  - `--fail-if-worse` rounded scores before comparing, so a drop under 0.01 could fail when it crossed a rounding boundary. It now compares the unrounded scores.
  - The `--trend` report (table, `short` and `terse`) now shows the overall score with two decimals. The gate error shows the drop with four decimals and names the tolerance.

## v0.25.1 (2026-09-23)

### Fix

- filter --per-function rows by --min-complexity
- **kaikai**: treat #[doc] as comment and skip multi-line string interiors

### Refactor

- **loc**: factor out string scanning and collapse the lang_spec arms

## v0.25.0 (2026-08-14)

### Feat

- **shell**: decompose Bourne/Bash/Zsh files into per-function metrics

### Fix

- **smells**: match debt markers as whole words; keep BUG case-sensitive
- **smells**: don't flag magic numbers in shell constant declarations

## v0.24.0 (2026-06-04)

### Feat

- **smells**: break down per-file table by smell type
- add km init to generate calibrated .kimun.toml

### Fix

- don't fail score gate on sub-0.01 rounding-noise regression
- don't treat function-like #define macros as functions

## v0.23.0 (2026-05-28)

### Feat

- add Kaikai complexity and Halstead metrics support

### Fix

- link advapi32 on Windows for libgit2-sys

## v0.22.0 (2026-05-28)

### Feat

- add Kaikai language support (.kai)

## v0.21.0 (2026-04-19)

### BREAKING CHANGE

- `--json` flag removed. Use `--format json` instead.
The per-command `--format` option on cycom/cogcom/smells is replaced by
the common `--format` flag (e.g. `--format github` for CI annotations).

### Feat

- add project-level configuration via .kimun.toml
- add --format codeclimate (alias --format gitlab) for GitLab Code Quality (#31)
- consolidate --json/--short/--terse into --format flag

### Fix

- replace sort_by with sort_by_key (clippy::unnecessary_sort_by) (#32)
- **score**: address review issues in quality gate format tests
- **score**: use two decimal places in quality gate failure message
- address code review findings from --format consolidation

### Refactor

- **main**: extract complex match arms to dedicated dispatch functions

## v0.20.0 (2026-04-05)

### Feat

- **dups**: add --fail-on-increase gate to prevent duplication debt growth
- **knowledge**: add --bus-factor flag to km knowledge
- **knowledge**: add --author flag to filter files by primary owner

### Refactor

- reduce cognitive complexity to recover A- score

## v0.19.0 (2026-04-05)

### Feat

- add km deps — dependency graph analysis with cycle detection
- add shell completion support via km completions <shell>
- add --format github|json to cycom, cogcom, and smells
- **score**: add --fail-if-worse and --fail-below gates to --trend

## v0.18.0 (2026-04-05)

### Feat

- **dups**: add --max-duplicates and --max-dup-ratio CI quality gates
- **churn**: add km churn command for pure change-frequency analysis
- **score**: add --trend flag to compare score against a git ref
- **knowledge**: add --summary flag for per-author ownership aggregation
- **smells**: add --files and --since-ref for PR-scoped analysis

### Fix

- three bugs found in post-merge quality review
- **report**: unify Unicode-safe column widths across all report formatters

## v0.17.0 (2026-04-04)

### Feat

- **loc**: add --by-author flag to break down lines by git author
- add `km smells` command for code smell detection
- add km authors command for per-author ownership summary
- **age**: add km age command to classify files by last git modification
- **ai**: add permissions command and --with-permissions flag for skill install

### Fix

- **loc**: fix table alignment and replace magic numbers with named constants

### Refactor

- extract print_per_function_breakdown to report_helpers

## v0.16.0 (2026-04-02)

### Feat

- **loc**: add HTML EEx (.heex) and PO File (.po, .pot) language support

## v0.15.1 (2026-03-03)

### Fix

- install project-level skill at git repo root, not cwd

### Feat

- add --exclude-ext, --exclude-dir, --exclude, --include-ext, and --list-excluded filters

### Refactor

- consolidate duplicated function detection into shared module
- extract `indent_level` to `util.rs`, remove 3 duplicates

## v0.14.0 (2026-03-01)

### Feat

- add `--model legacy` flag to `km score` for backward compatibility
- add cognitive complexity (SonarSource 2017), replace MI+cyclomatic in score
- add `km score diff` and auto-update homebrew on release

## v0.13.3 (2026-02-25)

## 0.12.3 (2026-02-19)

### Refactor

- improve code health score from C- to A (68→90)

## 0.12.2 (2026-02-18)

### Fix

- address nitpicks from code review

## 0.12.1 (2026-02-18)

### Fix

- address serious issues from second code review
- deduplicate files by content hash in km report (consistent with km loc)
- address blocking issues from second code review
- critical bugs found in code review

### Refactor

- address design issues from code review
- split large modules and add docs to reach A-
- improve test structure to move to B+

## 0.12.0 (2026-02-16)

### Feat

- install claude skill
- ai claude provider

### Fix

- dups now strips #[cfg(test)] blocks when excluding tests, matching score behavior

### Refactor

- read_and_classify and piecewise
- remove dups

## 0.11.0 (2026-02-16)

### Feat

- general score (grade)

## 0.10.0 (2026-02-16)

### Feat

- add version flag to show version
- add tc command (temporal coupling)

## 0.9.0 (2026-02-10)

### Feat

- add knowledge command (thornhill metric)

### Fix

- improve hotspots after code review

## 0.8.0 (2026-02-09)

### Feat

- add hotspots (tornhill)
- add git module with file frequency and temporal coupling APIs (git2)
- add --full flag to report command

### Refactor

- 1 pass to calculate mi and miv

## 0.7.0 (2026-02-09)

### Feat

- implement km report command
- full report

## 0.6.1 (2026-02-09)

### Refactor

- improve mi/miv sort stability, docs, and test coverage

## 0.6.0 (2026-02-09)

### Feat

- implements mi visual studio version
- implements maintainability index verisoft

## 0.5.0 (2026-02-09)

### Feat

- add hal command for Halstead complexity metrics

## 0.4.0 (2026-02-09)

### Feat

- add cycom command for cyclomatic complexity analysis

### Fix

- false positive for keywords inside block

### Refactor

- fix cycom false positives and split complex functions

## 0.3.0 (2026-02-08)

### Feat

- **indent**: use logical indentation levels and add complexity labels
- **indent**: add qualitative complexity labels based on indentation stddev
- add indent command for indentation complexity analysis

### Refactor

- shared code

## 0.2.0 (2026-02-07)

### Feat

- **dups**: add --include-tests flag
- **dups**: apply Rule of Three to classify duplicate severity
- **dups**: add --json flag to dups command for machine-readable output
- add --json flag to loc command for machine-readable output
- add dups command for duplicate code detection
- add --verbose/-v flag to loc command

### Fix

- include hidden files in directory walk and filter out .git directory

## 0.1.0 (2026-02-07)

### Feat

- loc command to count lines of code by programming language

### Fix

- improve count for perl, haskell, dos and bash
