use super::*;
use std::fs;

#[test]
fn run_on_temp_dir_with_rust_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    // hello\n    println!(\"hi\");\n}\n",
    )
    .unwrap();

    // Should succeed without error
    run(dir.path(), false, false).unwrap();
}

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    // Should succeed and print "No recognized source files found."
    run(dir.path(), false, false).unwrap();
}

#[test]
fn run_skips_binary_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
    // Should succeed — binary file silently skipped
    run(dir.path(), false, false).unwrap();
}

#[test]
fn run_deduplicates_identical_files() {
    let dir = tempfile::tempdir().unwrap();
    let content = "int x = 1;\n";
    fs::write(dir.path().join("a.c"), content).unwrap();
    fs::write(dir.path().join("b.c"), content).unwrap();
    // Should succeed — one of the duplicates skipped
    run(dir.path(), false, false).unwrap();
}

#[test]
fn run_with_shebang_detection() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("script"),
        "#!/usr/bin/env python3\nprint('hello')\n",
    )
    .unwrap();
    run(dir.path(), false, false).unwrap();
}

#[test]
fn run_verbose_on_temp_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn x() {}\n").unwrap();
    // Should succeed with verbose stats printed
    run(dir.path(), true, false).unwrap();
}

#[test]
fn run_verbose_with_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let content = "int x = 1;\n";
    fs::write(dir.path().join("a.c"), content).unwrap();
    fs::write(dir.path().join("b.c"), content).unwrap();
    // Should show skipped_files=1 (duplicate)
    run(dir.path(), true, false).unwrap();
}

#[test]
fn run_verbose_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), true, false).unwrap();
}

#[test]
fn run_json_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    println!(\"hi\");\n}\n",
    )
    .unwrap();
    run(dir.path(), false, true).unwrap();
}

#[test]
fn run_json_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), false, true).unwrap();
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
