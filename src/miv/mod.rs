//! Maintainability Index computation (verifysoft variant with comment weight).
//!
//! Combines Halstead Volume, Cyclomatic Complexity, LOC, and comment ratio
//! into a single maintainability score per file. Invoked via `cm miv`.
//!
//! This module directly calls `hal::analyze_file` and `cycom::analyze_file`
//! (pub(crate) functions). This creates tight coupling but avoids duplicating
//! file I/O and parsing logic. Changes to hal/cycom `analyze_file` signatures
//! must be coordinated with this module.
//!
//! Each file is read three times: once for LOC classification, once for
//! Halstead metrics (via `hal::analyze_file`), once for cyclomatic complexity
//! (via `cycom::analyze_file`). This is suboptimal but acceptable given the
//! existing per-module architecture where each analyzer owns its file I/O.

pub(crate) mod analyzer;
pub(crate) mod report;

use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use crate::loc::counter::{LineKind, classify_reader};
use crate::loc::language::{LanguageSpec, detect};
use crate::util::is_binary_reader;
use crate::walk;
use analyzer::compute_mi;
use report::{FileMIMetrics, print_json, print_report};

fn analyze_file(path: &Path, spec: &LanguageSpec) -> Result<Option<FileMIMetrics>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if is_binary_reader(&mut reader)? {
        return Ok(None);
    }

    let content = std::io::read_to_string(reader)?;
    let kinds = classify_reader(BufReader::new(Cursor::new(&content)), spec);

    let code_lines = kinds.iter().filter(|k| **k == LineKind::Code).count();
    let comment_lines = kinds.iter().filter(|k| **k == LineKind::Comment).count();

    let volume = match crate::hal::analyze_file(path, spec)? {
        Some(h) => h.metrics.volume,
        None => return Ok(None),
    };

    let complexity = match crate::cycom::analyze_file(path, spec)? {
        Some(c) => c.total_complexity,
        None => return Ok(None),
    };

    // compute_mi returns None only if code_lines==0, volume<=0, or complexity==0.
    // These should not occur when hal/cycom returned valid results, but guard anyway.
    let metrics = match compute_mi(volume, complexity, code_lines, comment_lines) {
        Some(m) => m,
        None => return Ok(None),
    };

    Ok(Some(FileMIMetrics {
        path: path.to_path_buf(),
        language: spec.name.to_string(),
        metrics,
    }))
}

pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    sort_by: &str,
) -> Result<(), Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut results: Vec<FileMIMetrics> = Vec::new();

    for entry in walk::walk(path, exclude_tests) {
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

        if exclude_tests && walk::is_test_file(file_path) {
            continue;
        }

        let spec = match detect(file_path) {
            Some(s) => s,
            None => match walk::try_detect_shebang(file_path) {
                Some(s) => s,
                None => continue,
            },
        };

        match analyze_file(file_path, spec) {
            Ok(Some(m)) => results.push(m),
            Ok(None) => {}
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }

    // Sort: mi ascending (worst first), volume/complexity/loc descending
    match sort_by {
        "volume" => results.sort_by(|a, b| {
            b.metrics
                .halstead_volume
                .total_cmp(&a.metrics.halstead_volume)
        }),
        "complexity" => {
            results.sort_by(|a, b| {
                b.metrics
                    .cyclomatic_complexity
                    .cmp(&a.metrics.cyclomatic_complexity)
            });
        }
        "loc" => results.sort_by(|a, b| b.metrics.loc.cmp(&a.metrics.loc)),
        _ => results.sort_by(|a, b| a.metrics.mi_score.total_cmp(&b.metrics.mi_score)),
    }

    results.truncate(top);

    if json {
        print_json(&results)?;
    } else {
        print_report(&results);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn run_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), false, false, 20, "mi").unwrap();
    }

    #[test]
    fn run_on_rust_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, "mi").unwrap();
    }

    #[test]
    fn run_on_python_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("app.py"),
            "def main():\n    x = 1\n    if x > 0:\n        print(x)\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, "mi").unwrap();
    }

    #[test]
    fn run_json_output() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        run(dir.path(), true, false, 20, "mi").unwrap();
    }

    #[test]
    fn run_skips_binary() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
        run(dir.path(), false, false, 20, "mi").unwrap();
    }

    #[test]
    fn run_excludes_tests_by_default() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("tests")).unwrap();
        fs::write(
            dir.path().join("tests/integration.rs"),
            "fn test() {\n    assert!(true);\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, "mi").unwrap();
    }

    #[test]
    fn run_includes_tests_with_flag() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("tests")).unwrap();
        fs::write(
            dir.path().join("tests/integration.rs"),
            "fn test() {\n    assert!(true);\n}\n",
        )
        .unwrap();
        run(dir.path(), false, true, 20, "mi").unwrap();
    }

    #[test]
    fn run_sort_by_volume() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, "volume").unwrap();
    }

    #[test]
    fn run_sort_by_complexity() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, "complexity").unwrap();
    }

    #[test]
    fn run_sort_by_loc() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, "loc").unwrap();
    }

    #[test]
    fn analyze_file_produces_valid_mi() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.rs");
        fs::write(
            &path,
            "// a comment\nfn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
        )
        .unwrap();
        let spec = detect(&path).unwrap();
        let result = analyze_file(&path, spec)
            .unwrap()
            .expect("should produce MI");
        assert!(
            result.metrics.mi_score.is_finite(),
            "MI score should be finite, got {}",
            result.metrics.mi_score
        );
        assert!(
            result.metrics.halstead_volume > 0.0,
            "volume should be positive"
        );
        assert!(
            result.metrics.cyclomatic_complexity > 0,
            "complexity should be positive"
        );
        assert!(result.metrics.loc > 0, "LOC should be positive");
        assert_eq!(
            result.metrics.comment_lines, 1,
            "should detect 1 comment line"
        );
        assert!(
            result.metrics.mi_score > 80.0,
            "simple code should have high MI, got {}",
            result.metrics.mi_score
        );
    }
}
