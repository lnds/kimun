mod analyzer;
mod markers;
pub(crate) mod report;

use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use crate::loc::counter::{LineKind, classify_reader};
use crate::loc::language::{LanguageSpec, detect};
use crate::util::is_binary_reader;
use crate::walk;
use analyzer::analyze;
use markers::markers_for;
use report::{FileCycomMetrics, print_json, print_per_function, print_report};

/// Analyze pre-read content (avoids re-reading the file).
pub(crate) fn analyze_content(
    lines: &[String],
    kinds: &[LineKind],
    spec: &LanguageSpec,
) -> Option<analyzer::FileComplexity> {
    let cm = markers_for(spec.name)?;
    analyze(lines, kinds, cm)
}

pub(crate) fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
) -> Result<Option<FileCycomMetrics>, Box<dyn Error>> {
    let cm = match markers_for(spec.name) {
        Some(m) => m,
        None => return Ok(None),
    };

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if is_binary_reader(&mut reader)? {
        return Ok(None);
    }

    let content = std::io::read_to_string(reader)?;
    let lines: Vec<String> = content.lines().map(String::from).collect();
    let kinds = classify_reader(BufReader::new(Cursor::new(&content)), spec);

    let fc = match analyze(&lines, &kinds, cm) {
        Some(fc) => fc,
        None => return Ok(None),
    };

    Ok(Some(FileCycomMetrics {
        path: path.to_path_buf(),
        language: spec.name.to_string(),
        function_count: fc.functions.len(),
        avg_complexity: fc.avg_complexity,
        max_complexity: fc.max_complexity,
        total_complexity: fc.total_complexity,
        level: fc.level,
        functions: fc.functions,
    }))
}

pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    min_complexity: usize,
    top: usize,
    per_function: bool,
    sort_by: &str,
) -> Result<(), Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut results: Vec<FileCycomMetrics> = Vec::new();

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
            Ok(Some(fc)) => results.push(fc),
            Ok(None) => {}
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }

    // Filter by min_complexity
    if min_complexity > 1 {
        results.retain(|f| f.max_complexity >= min_complexity);
    }

    // Sort by chosen metric descending
    match sort_by {
        "max" => results.sort_by(|a, b| b.max_complexity.cmp(&a.max_complexity)),
        "avg" => results.sort_by(|a, b| {
            b.avg_complexity
                .partial_cmp(&a.avg_complexity)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => results.sort_by(|a, b| b.total_complexity.cmp(&a.total_complexity)),
    }

    // Limit to top N
    results.truncate(top);

    if json {
        print_json(&results)?;
    } else if per_function {
        print_per_function(&results);
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
        run(dir.path(), false, false, 1, 20, false, "total").unwrap();
    }

    #[test]
    fn run_on_rust_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 1, 20, false, "total").unwrap();
    }

    #[test]
    fn run_on_python_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("app.py"),
            "def main():\n    if True:\n        print(\"hi\")\n",
        )
        .unwrap();
        run(dir.path(), false, false, 1, 20, false, "total").unwrap();
    }

    #[test]
    fn run_skips_binary() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
        run(dir.path(), false, false, 1, 20, false, "total").unwrap();
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
        run(dir.path(), false, false, 1, 20, false, "total").unwrap();
    }

    #[test]
    fn run_json_output() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        run(dir.path(), true, false, 1, 20, false, "total").unwrap();
    }

    #[test]
    fn run_per_function_output() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\nfn baz() {\n    quux();\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 1, 20, true, "total").unwrap();
    }

    #[test]
    fn run_min_complexity_filter() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("simple.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        // min_complexity=5 should filter out simple functions
        run(dir.path(), false, false, 5, 20, false, "total").unwrap();
    }

    #[test]
    fn run_top_limit() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("a.rs"),
            "fn a() {\n    if x { bar(); }\n}\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("b.rs"),
            "fn b() {\n    if y { baz(); }\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 1, 1, false, "total").unwrap();
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
        run(dir.path(), false, true, 1, 20, false, "total").unwrap();
    }

    #[test]
    fn run_skips_non_code_languages() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.json"), "{\"key\": \"value\"}\n").unwrap();
        fs::write(dir.path().join("style.css"), "body { color: red; }\n").unwrap();
        // Should produce no results (JSON/CSS have no markers)
        run(dir.path(), false, false, 1, 20, false, "total").unwrap();
    }
}
