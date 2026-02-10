mod detector;
mod report;

use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use crate::loc::counter::{LineKind, classify_reader};
use crate::loc::language::{LanguageSpec, detect};
use crate::util::is_binary_reader;
use crate::walk;
use detector::{NormalizedFile, NormalizedLine, detect_duplicates};
use report::{DuplicationMetrics, display_limit, print_detailed, print_json, print_summary};

fn normalize_file(
    path: &Path,
    spec: &LanguageSpec,
) -> Result<Option<NormalizedFile>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if is_binary_reader(&mut reader)? {
        return Ok(None);
    }

    let content = std::io::read_to_string(reader)?;
    let lines: Vec<&str> = content.lines().collect();
    let kinds = classify_reader(BufReader::new(Cursor::new(&content)), spec);

    let normalized: Vec<NormalizedLine> = lines
        .iter()
        .zip(kinds.iter())
        .enumerate()
        .filter(|(_, (_, kind))| **kind == LineKind::Code)
        .map(|(i, (line, _))| NormalizedLine {
            original_line_number: i + 1,
            content: line.trim().to_string(),
        })
        .collect();

    Ok(Some(NormalizedFile {
        path: path.to_path_buf(),
        lines: normalized,
    }))
}

pub fn run(
    path: &Path,
    min_lines: usize,
    show_report: bool,
    show_all: bool,
    json: bool,
    exclude_tests: bool,
) -> Result<(), Box<dyn Error>> {
    let mut files: Vec<NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

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

        match normalize_file(file_path, spec) {
            Ok(Some(nf)) => {
                total_code_lines += nf.lines.len();
                files.push(nf);
            }
            Ok(None) => {} // binary, skip
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }

    if files.is_empty() {
        if json {
            let metrics = DuplicationMetrics {
                total_code_lines: 0,
                duplicated_lines: 0,
                duplicate_groups: 0,
                files_with_duplicates: 0,
                largest_block: 0,
            };
            print_json(&metrics, &[])?;
        } else {
            println!("No recognized source files found.");
        }
        return Ok(());
    }

    let groups = detect_duplicates(&files, min_lines, json);

    let duplicated_lines: usize = groups.iter().map(|g| g.duplicated_lines()).sum();
    let largest_block = groups.iter().map(|g| g.line_count).max().unwrap_or(0);

    let files_with_dups: HashSet<&Path> = groups
        .iter()
        .flat_map(|g| g.locations.iter().map(|l| l.file_path.as_path()))
        .collect();

    let metrics = DuplicationMetrics {
        total_code_lines,
        duplicated_lines,
        duplicate_groups: groups.len(),
        files_with_duplicates: files_with_dups.len(),
        largest_block,
    };

    if json {
        let limit = display_limit(groups.len(), show_all);
        print_json(&metrics, &groups[..limit])?;
    } else if show_report {
        let limit = display_limit(groups.len(), show_all);
        print_detailed(&metrics, &groups[..limit], groups.len());
    } else {
        print_summary(&metrics, &groups);
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
        run(dir.path(), 6, false, false, false, false).unwrap();
    }

    #[test]
    fn run_with_no_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("a.rs"),
            "fn foo() {\n    let x = 1;\n    let y = 2;\n    let z = x + y;\n    println!(\"{}\", z);\n    return z;\n}\n",
        ).unwrap();
        fs::write(
            dir.path().join("b.rs"),
            "fn bar() {\n    let a = 10;\n    let b = 20;\n    let c = a * b;\n    println!(\"{}\", c);\n    return c;\n}\n",
        ).unwrap();
        run(dir.path(), 6, false, false, false, false).unwrap();
    }

    #[test]
    fn run_detects_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        // Should not panic, should detect duplicates
        run(dir.path(), 6, false, false, false, false).unwrap();
    }

    #[test]
    fn run_with_report_flag() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        run(dir.path(), 6, true, false, false, false).unwrap();
    }

    #[test]
    fn run_with_show_all_flag() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        run(dir.path(), 6, true, true, false, false).unwrap();
    }

    #[test]
    fn run_skips_binary_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
        run(dir.path(), 6, false, false, false, false).unwrap();
    }

    #[test]
    fn run_with_high_min_lines() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn f() {\n    let x = 1;\n    let y = 2;\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        // min_lines=20 means no 4-line file can produce duplicates
        run(dir.path(), 20, false, false, false, false).unwrap();
    }

    #[test]
    fn normalize_file_skips_comments_and_blanks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(
            &path,
            "// comment\n\nfn main() {\n    // another comment\n    let x = 1;\n}\n",
        )
        .unwrap();

        let spec = detect(Path::new("test.rs")).unwrap();
        let nf = normalize_file(&path, spec).unwrap().unwrap();

        // Should only have code lines: "fn main() {", "let x = 1;", "}"
        assert_eq!(nf.lines.len(), 3);
        assert_eq!(nf.lines[0].content, "fn main() {");
        assert_eq!(nf.lines[1].content, "let x = 1;");
        assert_eq!(nf.lines[2].content, "}");
    }

    #[test]
    fn normalize_file_binary_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.c");
        fs::write(&path, b"hello\x00world").unwrap();

        let spec = detect(Path::new("test.c")).unwrap();
        assert!(normalize_file(&path, spec).unwrap().is_none());
    }

    #[test]
    fn run_json_with_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        run(dir.path(), 6, false, false, true, false).unwrap();
    }

    #[test]
    fn run_json_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), 6, false, false, true, false).unwrap();
    }

    #[test]
    fn run_json_no_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "fn foo() {\n    let x = 1;\n}\n").unwrap();
        run(dir.path(), 6, false, false, true, false).unwrap();
    }

    // --- is_test_file tests (use shared walk module) ---

    #[test]
    fn test_file_rust() {
        assert!(walk::is_test_file(Path::new("parser_test.rs")));
        assert!(!walk::is_test_file(Path::new("parser.rs")));
        assert!(!walk::is_test_file(Path::new("test.rs"))); // no _test suffix
    }

    #[test]
    fn test_file_python() {
        assert!(walk::is_test_file(Path::new("test_parser.py")));
        assert!(walk::is_test_file(Path::new("parser_test.py")));
        assert!(!walk::is_test_file(Path::new("parser.py")));
    }

    #[test]
    fn test_file_javascript() {
        assert!(walk::is_test_file(Path::new("parser.test.js")));
        assert!(walk::is_test_file(Path::new("parser.spec.js")));
        assert!(walk::is_test_file(Path::new("parser.test.tsx")));
        assert!(walk::is_test_file(Path::new("parser.spec.ts")));
        assert!(!walk::is_test_file(Path::new("parser.js")));
    }

    #[test]
    fn test_file_java_kotlin() {
        assert!(walk::is_test_file(Path::new("ParserTest.java")));
        assert!(walk::is_test_file(Path::new("ParserTests.java")));
        assert!(!walk::is_test_file(Path::new("Parser.java")));
        assert!(walk::is_test_file(Path::new("ParserTest.kt")));
    }

    #[test]
    fn test_file_go() {
        assert!(walk::is_test_file(Path::new("parser_test.go")));
        assert!(!walk::is_test_file(Path::new("parser.go")));
    }

    #[test]
    fn test_file_csharp() {
        assert!(walk::is_test_file(Path::new("ParserTest.cs")));
        assert!(walk::is_test_file(Path::new("ParserTests.cs")));
        assert!(!walk::is_test_file(Path::new("Parser.cs")));
    }

    #[test]
    fn test_file_ruby() {
        assert!(walk::is_test_file(Path::new("parser_spec.rb")));
        assert!(walk::is_test_file(Path::new("parser_test.rb")));
        assert!(!walk::is_test_file(Path::new("parser.rb")));
    }

    #[test]
    fn test_file_cpp() {
        assert!(walk::is_test_file(Path::new("parser_test.cpp")));
        assert!(walk::is_test_file(Path::new("test_parser.cpp")));
        assert!(walk::is_test_file(Path::new("parser_unittest.cpp")));
        assert!(walk::is_test_file(Path::new("ParserTest.cpp")));
        assert!(!walk::is_test_file(Path::new("parser.cpp")));
    }

    #[test]
    fn test_file_c() {
        assert!(walk::is_test_file(Path::new("parser_test.c")));
        assert!(walk::is_test_file(Path::new("test_parser.c")));
        assert!(walk::is_test_file(Path::new("parser_unittest.c")));
        assert!(!walk::is_test_file(Path::new("parser.c")));
    }

    #[test]
    fn test_file_other_languages() {
        assert!(walk::is_test_file(Path::new("parser_test.exs")));
        assert!(walk::is_test_file(Path::new("parser_test.dart")));
        assert!(walk::is_test_file(Path::new("ParserTest.swift")));
        assert!(walk::is_test_file(Path::new("ParserSpec.scala")));
        assert!(walk::is_test_file(Path::new("ParserSpec.hs")));
        assert!(walk::is_test_file(Path::new("ParserTest.php")));
    }

    #[test]
    fn test_file_no_extension() {
        assert!(!walk::is_test_file(Path::new("Makefile")));
        assert!(!walk::is_test_file(Path::new("README")));
    }

    #[test]
    fn test_file_unknown_extension() {
        assert!(!walk::is_test_file(Path::new("test_foo.xyz")));
    }

    // --- exclude_tests integration tests ---

    #[test]
    fn run_exclude_tests_skips_test_dir() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";

        // Only duplicates are inside tests/
        fs::create_dir(dir.path().join("tests")).unwrap();
        fs::write(dir.path().join("tests/a.rs"), code).unwrap();
        fs::write(dir.path().join("tests/b.rs"), code).unwrap();
        fs::write(dir.path().join("lib.rs"), "fn foo() {\n    let x = 1;\n}\n").unwrap();

        // Without exclude: detects duplicates (does not panic)
        run(dir.path(), 6, false, false, false, false).unwrap();
        // With exclude: tests/ is skipped entirely
        run(dir.path(), 6, false, false, false, true).unwrap();
    }

    #[test]
    fn run_exclude_tests_skips_test_files() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";

        // Duplicate in test-named files
        fs::write(dir.path().join("parser_test.rs"), code).unwrap();
        fs::write(dir.path().join("handler_test.rs"), code).unwrap();
        fs::write(dir.path().join("lib.rs"), "fn foo() {\n    let x = 1;\n}\n").unwrap();

        // With exclude_tests, the *_test.rs files are skipped
        run(dir.path(), 6, false, false, false, true).unwrap();
    }

    #[test]
    fn run_exclude_tests_skips_test_file_in_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";

        // Test file nested in a non-test directory
        fs::create_dir_all(dir.path().join("src/utils")).unwrap();
        fs::write(dir.path().join("src/utils/parser_test.rs"), code).unwrap();
        fs::write(dir.path().join("src/utils/handler_test.rs"), code).unwrap();
        fs::write(
            dir.path().join("src/lib.rs"),
            "fn foo() {\n    let x = 1;\n}\n",
        )
        .unwrap();

        // With exclude_tests, *_test.rs files in any directory are skipped
        run(dir.path(), 6, false, false, false, true).unwrap();
    }

    #[test]
    fn run_exclude_tests_skips_entire_test_dir_tree() {
        let dir = tempfile::tempdir().unwrap();
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";

        // Files inside tests/ with no test suffix — excluded by directory filter
        fs::create_dir_all(dir.path().join("tests/helpers")).unwrap();
        fs::write(dir.path().join("tests/integration.rs"), code).unwrap();
        fs::write(dir.path().join("tests/helpers/utils.rs"), code).unwrap();
        fs::write(dir.path().join("lib.rs"), "fn foo() {\n    let x = 1;\n}\n").unwrap();

        // With exclude_tests, the entire tests/ tree is skipped
        run(dir.path(), 6, false, false, false, true).unwrap();
    }
}
