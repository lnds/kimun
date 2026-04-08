use super::*;
use crate::cli::OutputMode;
use crate::loc::language::detect;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "mi").unwrap();
}

#[test]
fn run_on_rust_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "mi").unwrap();
}

#[test]
fn run_on_python_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("app.py"),
        "def main():\n    x = 1\n    if x > 0:\n        print(x)\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "mi").unwrap();
}

#[test]
fn run_json_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Json, 20, "mi").unwrap();
}

#[test]
fn run_skips_binary() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "mi").unwrap();
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
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "mi").unwrap();
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
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    run(&cfg, OutputMode::Table, 20, "mi").unwrap();
}

#[test]
fn run_sort_by_volume() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "volume").unwrap();
}

#[test]
fn run_sort_by_complexity() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "complexity").unwrap();
}

#[test]
fn run_sort_by_loc() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "loc").unwrap();
}

#[test]
fn run_sort_by_volume_two_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("a.rs"),
        "fn foo() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("b.rs"),
        "fn bar(a: i32, b: i32, c: i32) -> i32 {\n    if a > 0 { a + b } else { b + c }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "volume").unwrap();
}

#[test]
fn run_sort_by_complexity_two_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("a.rs"),
        "fn foo() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("b.rs"),
        "fn bar(a: i32) -> i32 {\n    if a > 0 { a } else { -a }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 20, "complexity").unwrap();
}

#[test]
fn analyze_file_returns_none_for_hal_unsupported() {
    // JSON files are not supported by hal — analyze_file returns None
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.json");
    fs::write(&path, "{\"key\": \"value\"}\n").unwrap();
    let spec = detect(&path).unwrap();
    let result = analyze_file(&path, spec).unwrap();
    assert!(
        result.is_none(),
        "JSON file should return None (hal unsupported)"
    );
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
