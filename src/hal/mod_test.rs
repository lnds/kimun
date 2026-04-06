use super::*;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 20, "effort").unwrap();
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
    run(&cfg, false, 20, "effort").unwrap();
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
    run(&cfg, false, 20, "effort").unwrap();
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
    run(&cfg, true, 20, "effort").unwrap();
}

#[test]
fn run_skips_binary() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 20, "effort").unwrap();
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
    run(&cfg, false, 20, "effort").unwrap();
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
    run(&cfg, false, 20, "effort").unwrap();
}

#[test]
fn run_skips_unsupported_languages() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.json"), "{\"key\": \"value\"}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 20, "effort").unwrap();
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
    run(&cfg, false, 20, "volume").unwrap();
}

#[test]
fn python_docstring_not_tokenized() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("app.py"),
        "def foo(x):\n    \"\"\"\n    if this and that:\n        return 42\n    \"\"\"\n    return x + 1\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 20, "effort").unwrap();
}

#[test]
fn run_sort_by_bugs() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 20, "bugs").unwrap();
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
        "fn bar(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 20, "volume").unwrap();
}

#[test]
fn run_sort_by_bugs_two_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("a.rs"),
        "fn foo() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("b.rs"),
        "fn bar(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 20, "bugs").unwrap();
}

#[test]
fn analyze_file_returns_none_for_unsupported_language() {
    use crate::loc::language::detect;
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
fn analyze_file_returns_none_for_comment_only_file() {
    use crate::loc::language::detect;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("comments_only.rs");
    fs::write(&path, "// just a comment\n// another comment\n").unwrap();
    let spec = detect(&path).unwrap();
    let result = analyze_file(&path, spec).unwrap();
    assert!(
        result.is_none(),
        "comment-only file should return None (no code lines)"
    );
}
