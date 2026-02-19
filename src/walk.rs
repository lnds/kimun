//! Filesystem walking with `.gitignore` support and test exclusion.
//!
//! Provides directory traversal that respects `.gitignore` rules, skips
//! `.git` directories, filters test directories/files when requested,
//! and detects source file languages by extension or shebang line.
//! Uses the `ignore` crate for efficient `.gitignore`-aware traversal.
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::loc::language::{LanguageSpec, detect, detect_by_shebang};

/// Test directory names to exclude when `--exclude-tests` is active.
pub const TEST_DIRS: &[&str] = &["tests", "test", "__tests__", "spec"];

/// Mapping from file extension to test-file naming patterns.
/// Used to identify and exclude test files based on naming conventions.
struct TestPattern {
    exts: &'static [&'static str],
    suffixes: &'static [&'static str],
    prefixes: &'static [&'static str],
}

/// Data-driven test file detection patterns, grouped by naming convention.
const TEST_PATTERNS: &[TestPattern] = &[
    // snake_case _test suffix: Rust, Go, Elixir, Dart
    TestPattern {
        exts: &["rs", "go", "exs", "dart"],
        suffixes: &["_test"],
        prefixes: &[],
    },
    // Python: test_ prefix or _test suffix
    TestPattern {
        exts: &["py"],
        suffixes: &["_test"],
        prefixes: &["test_"],
    },
    // Ruby: _test or _spec suffix
    TestPattern {
        exts: &["rb"],
        suffixes: &["_test", "_spec"],
        prefixes: &[],
    },
    // PHP: PascalCase Test or snake_case _test
    TestPattern {
        exts: &["php"],
        suffixes: &["Test", "_test"],
        prefixes: &[],
    },
    // JS/TS family: .test. or .spec. double extension
    TestPattern {
        exts: &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"],
        suffixes: &[".test", ".spec"],
        prefixes: &[],
    },
    // JVM + C# + Swift: PascalCase Test/Tests suffix
    TestPattern {
        exts: &["java", "kt", "kts", "cs", "swift"],
        suffixes: &["Test", "Tests"],
        prefixes: &[],
    },
    // Scala: PascalCase Test or Spec suffix
    TestPattern {
        exts: &["scala"],
        suffixes: &["Test", "Spec"],
        prefixes: &[],
    },
    // C: snake_case only (no PascalCase Test)
    TestPattern {
        exts: &["c"],
        suffixes: &["_test", "_unittest"],
        prefixes: &["test_"],
    },
    // C++: snake_case + PascalCase Test
    TestPattern {
        exts: &["cc", "cpp", "cxx"],
        suffixes: &["_test", "_unittest", "Test"],
        prefixes: &["test_"],
    },
    // Haskell: PascalCase Test or Spec
    TestPattern {
        exts: &["hs"],
        suffixes: &["Test", "Spec"],
        prefixes: &[],
    },
];

/// Check whether a file matches a test naming pattern based on its extension.
pub fn is_test_file(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    let Some(dot) = file_name.rfind('.') else {
        return false;
    };
    let ext = &file_name[dot + 1..];
    let base = &file_name[..dot];

    for pattern in TEST_PATTERNS {
        if !pattern.exts.contains(&ext) {
            continue;
        }
        if pattern.suffixes.iter().any(|s| base.ends_with(s)) {
            return true;
        }
        if pattern.prefixes.iter().any(|p| base.starts_with(p)) {
            return true;
        }
    }
    false
}

/// Try to detect a language by reading the shebang line of a file.
pub fn try_detect_shebang(path: &Path) -> Option<&'static LanguageSpec> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).ok()?;
    detect_by_shebang(&first_line)
}

/// Walk the directory tree and return all recognized source files with their
/// detected language spec. Handles errors, filters non-files, excludes test
/// files when requested, and detects language by extension or shebang.
pub fn source_files(path: &Path, exclude_tests: bool) -> Vec<(PathBuf, &'static LanguageSpec)> {
    let mut result = Vec::new();
    for entry in walk(path, exclude_tests) {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("warning: {err}");
                continue;
            }
        };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let file_path = entry.path();
        if exclude_tests && is_test_file(file_path) {
            continue;
        }
        let spec = match detect(file_path) {
            Some(s) => s,
            None => match try_detect_shebang(file_path) {
                Some(s) => s,
                None => continue,
            },
        };
        result.push((file_path.to_path_buf(), spec));
    }
    result
}

/// Walk source files, analyze each with `f`, and collect successful results.
/// Handles the common Ok(Some)/Ok(None)/Err pattern used across modules.
pub fn collect_analysis<T>(
    path: &Path,
    exclude_tests: bool,
    f: impl Fn(&Path, &LanguageSpec) -> Result<Option<T>, Box<dyn std::error::Error>>,
) -> Vec<T> {
    let mut results = Vec::new();
    for (file_path, spec) in source_files(path, exclude_tests) {
        match f(&file_path, spec) {
            Ok(Some(m)) => results.push(m),
            Ok(None) => {}
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }
    results
}

/// Build a directory walker that respects `.gitignore`, skips `.git`,
/// and optionally excludes test directories.
pub fn walk(path: &Path, exclude_tests: bool) -> ignore::Walk {
    WalkBuilder::new(path)
        .hidden(false)
        .follow_links(false)
        .filter_entry(move |entry| {
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                if entry.file_name() == ".git" {
                    return false;
                }
                if exclude_tests
                    && let Some(name) = entry.file_name().to_str()
                    && TEST_DIRS.contains(&name)
                {
                    return false;
                }
            }
            true
        })
        .build()
}
