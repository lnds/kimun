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

- implement cm report command
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
