mod analyzer;
mod report;

use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use ignore::WalkBuilder;

use crate::loc::counter::classify_reader;
use crate::loc::language::{LanguageSpec, detect, detect_by_shebang};
use analyzer::analyze;
use report::{FileIndentMetrics, print_json, print_report};

const TAB_WIDTH: usize = 4;

/// Test directory names to exclude by default.
const TEST_DIRS: &[&str] = &["tests", "test", "__tests__", "spec"];

/// Check whether a file matches a test naming pattern based on its extension.
fn is_test_file(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    let Some(dot) = file_name.rfind('.') else {
        return false;
    };
    let ext = &file_name[dot + 1..];
    let base = &file_name[..dot];

    match ext {
        "rs" | "go" | "exs" | "dart" => base.ends_with("_test"),
        "py" => base.starts_with("test_") || base.ends_with("_test"),
        "rb" => base.ends_with("_test") || base.ends_with("_spec"),
        "php" => base.ends_with("Test") || base.ends_with("_test"),
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" => {
            base.ends_with(".test") || base.ends_with(".spec")
        }
        "java" | "kt" | "kts" => base.ends_with("Test") || base.ends_with("Tests"),
        "cs" => base.ends_with("Test") || base.ends_with("Tests"),
        "swift" => base.ends_with("Test") || base.ends_with("Tests"),
        "scala" => base.ends_with("Test") || base.ends_with("Spec"),
        "c" => base.ends_with("_test") || base.starts_with("test_") || base.ends_with("_unittest"),
        "cc" | "cpp" | "cxx" => {
            base.ends_with("_test")
                || base.starts_with("test_")
                || base.ends_with("_unittest")
                || base.ends_with("Test")
        }
        "hs" => base.ends_with("Test") || base.ends_with("Spec"),
        _ => false,
    }
}

fn try_detect_shebang(path: &Path) -> Option<&'static LanguageSpec> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).ok()?;
    detect_by_shebang(&first_line)
}

fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
) -> Result<Option<FileIndentMetrics>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Binary detection: reject files with null bytes in first 512 bytes.
    let mut header = [0u8; 512];
    let n = reader.read(&mut header)?;
    if header[..n].contains(&0) {
        return Ok(None);
    }
    reader.seek(SeekFrom::Start(0))?;

    // Read content once, reuse for classification and indentation analysis.
    let content = std::io::read_to_string(reader)?;
    let lines: Vec<String> = content.lines().map(String::from).collect();
    let kinds = classify_reader(BufReader::new(Cursor::new(&content)), spec);

    let metrics = match analyze(&lines, &kinds, TAB_WIDTH) {
        Some(m) => m,
        None => return Ok(None),
    };

    Ok(Some(FileIndentMetrics {
        path: path.to_path_buf(),
        code_lines: metrics.code_lines,
        stddev: metrics.stddev,
        max_depth: metrics.max_depth,
    }))
}

pub fn run(path: &Path, json: bool, include_tests: bool) -> Result<(), Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut results: Vec<FileIndentMetrics> = Vec::new();

    let walker = WalkBuilder::new(path)
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
        .build();

    for entry in walker {
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

        match analyze_file(file_path, spec) {
            Ok(Some(fc)) => results.push(fc),
            Ok(None) => {}
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }

    // Sort by stddev descending
    results.sort_by(|a, b| b.stddev.total_cmp(&a.stddev));

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
        run(dir.path(), false, false).unwrap();
    }

    #[test]
    fn run_on_rust_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n    if x > 0 {\n        println!(\"hi\");\n    }\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false).unwrap();
    }

    #[test]
    fn run_json_output() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        run(dir.path(), true, false).unwrap();
    }

    #[test]
    fn run_skips_binary() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
        run(dir.path(), false, false).unwrap();
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
        // No source files outside tests/ → prints "No recognized source files"
        run(dir.path(), false, false).unwrap();
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
        run(dir.path(), false, true).unwrap();
    }

    #[test]
    fn run_excludes_test_files_by_name() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("parser_test.rs"),
            "fn test() {\n    assert!(true);\n}\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("parser.rs"),
            "fn parse() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        // parser_test.rs excluded, only parser.rs analyzed
        run(dir.path(), false, false).unwrap();
    }

    #[test]
    fn run_sorts_by_stddev_descending() {
        let dir = tempfile::tempdir().unwrap();
        // Flat file → low stddev
        fs::write(
            dir.path().join("flat.rs"),
            "fn a() {}\nfn b() {}\nfn c() {}\n",
        )
        .unwrap();
        // Nested file → high stddev
        fs::write(
            dir.path().join("nested.rs"),
            "fn main() {\n    if true {\n        if true {\n            if true {\n                x();\n            }\n        }\n    }\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false).unwrap();
    }

    #[test]
    fn run_multiple_languages() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    println!(\"hi\");\n}\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("app.py"),
            "def main():\n    print(\"hi\")\n",
        )
        .unwrap();
        run(dir.path(), false, true).unwrap();
    }
}
