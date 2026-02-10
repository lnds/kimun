mod analyzer;
pub(crate) mod report;
mod tokenizer;

use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use crate::loc::counter::{LineKind, classify_reader};
use crate::loc::language::{LanguageSpec, detect};
use crate::util::is_binary_reader;
use crate::walk;
use analyzer::compute;
use report::{FileHalsteadMetrics, print_json, print_report};
use tokenizer::{count_tokens, rules_for};

/// For languages with triple-quoted strings (Python), mark interior lines
/// of multi-line strings so the tokenizer can skip them.
/// Only marks lines that both start AND end inside a triple-quoted string
/// (true interior lines). Opening/closing lines are not masked — their
/// string content is handled by `mask_strings` in the tokenizer.
fn multi_line_string_mask(lines: &[String], spec: &LanguageSpec) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    if !spec.triple_quote_strings {
        return mask;
    }
    let mut in_triple: Option<&str> = None;
    for (idx, line) in lines.iter().enumerate() {
        let started_in_string = in_triple.is_some();

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        while i < len {
            if let Some(delim) = in_triple {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2; // skip backslash and escaped char
                } else if bytes[i..].starts_with(delim.as_bytes()) {
                    in_triple = None;
                    i += delim.len();
                } else {
                    i += 1;
                }
            } else if bytes[i] == b'"' || bytes[i] == b'\'' {
                let q = bytes[i];
                let triple: &str = if q == b'"' { "\"\"\"" } else { "'''" };
                if bytes[i..].starts_with(triple.as_bytes()) {
                    in_triple = Some(triple);
                    i += 3;
                } else {
                    // Skip regular single-line string
                    i += 1;
                    while i < len && bytes[i] != q {
                        if bytes[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    if i < len {
                        i += 1;
                    }
                }
            } else {
                i += 1;
            }
        }

        // Only mask lines that are entirely inside a string
        // (started inside and still inside at end of line).
        mask[idx] = started_in_string && in_triple.is_some();
    }
    mask
}

pub(crate) fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
) -> Result<Option<FileHalsteadMetrics>, Box<dyn Error>> {
    let rules = match rules_for(spec.name) {
        Some(r) => r,
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
    let string_mask = multi_line_string_mask(&lines, spec);

    // Collect only code lines that are not inside multi-line strings
    let code_lines: Vec<&str> = lines
        .iter()
        .zip(&kinds)
        .zip(&string_mask)
        .filter(|((_, k), in_string)| **k == LineKind::Code && !*in_string)
        .map(|((line, _), _)| line.as_str())
        .collect();

    if code_lines.is_empty() {
        return Ok(None);
    }

    let counts = count_tokens(&code_lines, rules);
    let metrics = match compute(&counts) {
        Some(m) => m,
        None => return Ok(None),
    };

    Ok(Some(FileHalsteadMetrics {
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
    let mut results: Vec<FileHalsteadMetrics> = Vec::new();

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

    // Sort by chosen metric descending
    match sort_by {
        "volume" => results.sort_by(|a, b| {
            b.metrics
                .volume
                .partial_cmp(&a.metrics.volume)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "bugs" => results.sort_by(|a, b| {
            b.metrics
                .bugs
                .partial_cmp(&a.metrics.bugs)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => results.sort_by(|a, b| {
            b.metrics
                .effort
                .partial_cmp(&a.metrics.effort)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
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
        run(dir.path(), false, false, 20, "effort").unwrap();
    }

    #[test]
    fn run_on_rust_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, "effort").unwrap();
    }

    #[test]
    fn run_on_python_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("app.py"),
            "def main():\n    x = 1\n    if x > 0:\n        print(x)\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, "effort").unwrap();
    }

    #[test]
    fn run_json_output() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        run(dir.path(), true, false, 20, "effort").unwrap();
    }

    #[test]
    fn run_skips_binary() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
        run(dir.path(), false, false, 20, "effort").unwrap();
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
        run(dir.path(), false, false, 20, "effort").unwrap();
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
        run(dir.path(), false, true, 20, "effort").unwrap();
    }

    #[test]
    fn run_skips_unsupported_languages() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.json"), "{\"key\": \"value\"}\n").unwrap();
        run(dir.path(), false, false, 20, "effort").unwrap();
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
    fn multi_line_string_mask_python() {
        let spec = detect(std::path::Path::new("test.py")).unwrap();
        let lines: Vec<String> = vec![
            "x = 1",         // 0: not in string
            "y = \"\"\"",    // 1: opens triple (has code before delimiter)
            "def foo():",    // 2: interior — entirely inside triple string
            "    return 42", // 3: interior — entirely inside triple string
            "\"\"\"",        // 4: closes triple (not interior)
            "z = 2",         // 5: not in string
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let mask = multi_line_string_mask(&lines, spec);
        assert!(!mask[0]); // normal code
        assert!(!mask[1]); // opening line — not masked
        assert!(mask[2]); // interior line
        assert!(mask[3]); // interior line
        assert!(!mask[4]); // closing line — not masked
        assert!(!mask[5]); // normal code after close
    }

    #[test]
    fn multi_line_string_mask_closing_with_code() {
        let spec = detect(std::path::Path::new("test.py")).unwrap();
        let lines: Vec<String> = vec![
            "x = \"\"\"", // 0: opens triple
            "docstring",  // 1: interior
            "\"\"\" + y", // 2: closes triple, has code after
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let mask = multi_line_string_mask(&lines, spec);
        assert!(!mask[0]); // opening line
        assert!(mask[1]); // interior line
        assert!(!mask[2]); // closing line — code `+ y` preserved
    }

    #[test]
    fn multi_line_string_mask_escaped_triple_quotes() {
        let spec = detect(std::path::Path::new("test.py")).unwrap();
        // Escaped triple-quotes inside a triple-quoted string should not close it
        let lines: Vec<String> = vec![
            r#"x = """has fake \"\"\" inside""""#, // 0: opens and closes on same line
            "y = 1 + 2",                           // 1: normal code
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let mask = multi_line_string_mask(&lines, spec);
        assert!(!mask[0]); // single-line triple string, not masked
        assert!(!mask[1]); // normal code
    }

    #[test]
    fn multi_line_string_mask_non_triple_language() {
        let spec = detect(std::path::Path::new("test.rs")).unwrap();
        let lines: Vec<String> = vec!["let x = 1;", "let y = 2;"]
            .into_iter()
            .map(String::from)
            .collect();
        let mask = multi_line_string_mask(&lines, spec);
        assert!(!mask[0]);
        assert!(!mask[1]);
    }

    #[test]
    fn python_docstring_not_tokenized() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("app.py"),
            "def foo(x):\n    \"\"\"\n    if this and that:\n        return 42\n    \"\"\"\n    return x + 1\n",
        )
        .unwrap();
        // Should not panic, and the "if", "return 42" inside the docstring
        // should not be counted as operators/operands
        run(dir.path(), false, false, 20, "effort").unwrap();
    }

    #[test]
    fn run_sort_by_bugs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, "bugs").unwrap();
    }
}
