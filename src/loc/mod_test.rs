use super::*;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;
use std::path::Path;

#[test]
fn run_on_temp_dir_with_rust_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    // hello\n    println!(\"hi\");\n}\n",
    )
    .unwrap();

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed without error
    run(&cfg, false, false).unwrap();
}

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed and print "No recognized source files found."
    run(&cfg, false, false).unwrap();
}

#[test]
fn run_skips_binary_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed — binary file silently skipped
    run(&cfg, false, false).unwrap();
}

#[test]
fn run_deduplicates_identical_files() {
    let dir = tempfile::tempdir().unwrap();
    let content = "int x = 1;\n";
    fs::write(dir.path().join("a.c"), content).unwrap();
    fs::write(dir.path().join("b.c"), content).unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed — one of the duplicates skipped
    run(&cfg, false, false).unwrap();
}

#[test]
fn run_with_shebang_detection() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("script"),
        "#!/usr/bin/env python3\nprint('hello')\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, false).unwrap();
}

#[test]
fn run_verbose_on_temp_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn x() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed with verbose stats printed
    run(&cfg, true, false).unwrap();
}

#[test]
fn run_verbose_with_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let content = "int x = 1;\n";
    fs::write(dir.path().join("a.c"), content).unwrap();
    fs::write(dir.path().join("b.c"), content).unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should show skipped_files=1 (duplicate)
    run(&cfg, true, false).unwrap();
}

#[test]
fn run_verbose_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, true, false).unwrap();
}

#[test]
fn run_json_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    println!(\"hi\");\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, true).unwrap();
}

#[test]
fn run_json_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, true).unwrap();
}

#[test]
fn hash_file_works() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    fs::write(&path, "hello world").unwrap();

    let h1 = hash_file(&path).unwrap();
    let h2 = hash_file(&path).unwrap();
    assert_eq!(h1, h2, "same content should produce same hash");
}

#[test]
fn hash_file_nonexistent() {
    assert!(hash_file(Path::new("/nonexistent/file")).is_none());
}
