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
