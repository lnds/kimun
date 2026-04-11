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

### Refactor

- consolidate duplicated function detection into shared module
- extract `indent_level` to `util.rs`, remove 3 duplicates

## 0.15.0 (2026-03-02)

### Feat

- add --exclude-ext, --exclude-dir, --exclude, --include-ext, and --list-excluded filters

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
