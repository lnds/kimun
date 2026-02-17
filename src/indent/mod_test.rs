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
