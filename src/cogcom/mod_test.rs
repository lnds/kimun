use super::*;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 1, 20, false, "total").unwrap();
}

#[test]
fn run_on_rust_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    if x > 0 {\n        println!(\"hi\");\n    }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 1, 20, false, "total").unwrap();
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
    run(&cfg, true, 1, 20, false, "total").unwrap();
}

#[test]
fn run_per_function() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\nfn bar() {\n    baz();\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 1, 20, true, "total").unwrap();
}

#[test]
fn run_with_min_complexity_filter() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("simple.rs"),
        "fn foo() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 5, 20, false, "total").unwrap();
}

#[test]
fn run_sort_by_max() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 1, 20, false, "max").unwrap();
}

#[test]
fn run_sort_by_avg() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 1, 20, false, "avg").unwrap();
}

#[test]
fn analyze_content_returns_some_for_rust() {
    let lines: Vec<String> = "fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\n"
        .lines()
        .map(String::from)
        .collect();
    let kinds = vec![crate::loc::counter::LineKind::Code; lines.len()];
    let spec = crate::loc::language::detect(std::path::Path::new("test.rs")).unwrap();
    let result = analyze_content(&lines, &kinds, spec);
    assert!(result.is_some());
    assert_eq!(result.unwrap().max_complexity, 1);
}

#[test]
fn analyze_content_returns_none_for_unknown() {
    let lines = vec!["hello".to_string()];
    let kinds = vec![crate::loc::counter::LineKind::Code];
    // JSON has no cognitive markers
    let spec = crate::loc::language::detect(std::path::Path::new("test.json")).unwrap();
    let result = analyze_content(&lines, &kinds, spec);
    assert!(result.is_none());
}
