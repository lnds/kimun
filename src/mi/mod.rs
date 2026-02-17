//! Maintainability Index computation (Visual Studio variant).
//!
//! Computes MI per file using the Visual Studio formula: no comment weight,
//! normalized to 0–100 scale, clamped at 0. Invoked via `cm mi`.
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
use std::path::Path;

use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;
use crate::util::read_and_classify;
use crate::walk;
use analyzer::compute_mi;
use report::{FileMIMetrics, print_json, print_report};

fn analyze_file(path: &Path, spec: &LanguageSpec) -> Result<Option<FileMIMetrics>, Box<dyn Error>> {
    let (_lines, kinds) = match read_and_classify(path, spec)? {
        Some(v) => v,
        None => return Ok(None),
    };
    let code_lines = kinds.iter().filter(|k| **k == LineKind::Code).count();

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
    let metrics = match compute_mi(volume, complexity, code_lines) {
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

    for (file_path, spec) in walk::source_files(path, exclude_tests) {
        match analyze_file(&file_path, spec) {
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
    use crate::loc::language::detect;
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
    fn analyze_file_returns_none_for_empty_code() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.rs");
        fs::write(&path, "// only a comment\n").unwrap();
        let spec = detect(&path).unwrap();
        let result = analyze_file(&path, spec).unwrap();
        assert!(
            result.is_none(),
            "file with no code lines should return None"
        );
    }

    #[test]
    fn analyze_file_produces_valid_mi() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.rs");
        fs::write(
            &path,
            "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
        )
        .unwrap();
        let spec = detect(&path).unwrap();
        let result = analyze_file(&path, spec)
            .unwrap()
            .expect("should produce MI");
        assert!(
            result.metrics.mi_score >= 0.0,
            "MI should be >= 0, got {}",
            result.metrics.mi_score
        );
        assert!(
            result.metrics.mi_score <= 100.0,
            "MI should be <= 100, got {}",
            result.metrics.mi_score
        );
        assert!(
            result.metrics.mi_score > 20.0,
            "simple code should be Green (>20), got {}",
            result.metrics.mi_score
        );
    }
}
