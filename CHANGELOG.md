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
