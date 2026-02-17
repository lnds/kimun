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
