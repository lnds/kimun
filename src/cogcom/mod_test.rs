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
    run(&cfg, OutputMode::Table, 1, 20, false, "total").unwrap();
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
    run(&cfg, OutputMode::Table, 1, 20, false, "total").unwrap();
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
    run(&cfg, OutputMode::Json, 1, 20, false, "total").unwrap();
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
    run(&cfg, OutputMode::Table, 1, 20, true, "total").unwrap();
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
    run(&cfg, OutputMode::Table, 5, 20, false, "total").unwrap();
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
    run(&cfg, OutputMode::Table, 1, 20, false, "max").unwrap();
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
    run(&cfg, OutputMode::Table, 1, 20, false, "avg").unwrap();
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

#[test]
fn run_github_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Github, 1, 20, false, "total").unwrap();
}

#[test]
fn run_format_json_via_format_param() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Json, 1, 20, false, "total").unwrap();
}

#[test]
fn analyze_file_returns_none_for_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.json");
    fs::write(&path, "{\"key\": \"value\"}\n").unwrap();
    let spec = detect(&path).unwrap();
    let result = analyze_file(&path, spec).unwrap();
    assert!(
        result.is_none(),
        "JSON has no cognitive markers, should return None"
    );
}

#[test]
fn analyze_file_returns_none_for_comment_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("comments.rs");
    fs::write(&path, "// just a comment\n// another\n").unwrap();
    let spec = detect(&path).unwrap();
    let result = analyze_file(&path, spec).unwrap();
    // A file with only comments has no functions, so analyze returns None
    assert!(
        result.is_none(),
        "comment-only Rust file should return None"
    );
}

#[test]
fn run_short_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn foo() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Short, 0, 20, false, "total").unwrap();
}

#[test]
fn run_terse_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn foo() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Terse, 0, 20, false, "total").unwrap();
}
