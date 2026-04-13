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
        "// doc comment\nfn foo() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
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
        "// comment\nfn foo() {\n    let x = 1;\n}\n",
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

#[test]
fn run_short_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n    println!(\"{}\", x);\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Short, 20, "mi").unwrap();
}

#[test]
fn run_terse_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n    println!(\"{}\", x);\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Terse, 20, "mi").unwrap();
}
