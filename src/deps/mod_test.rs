use super::*;
use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;

// ── detect_go_module ─────────────────────────────────────────────────────────

#[test]
fn detect_go_module_returns_none_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    assert!(detect_go_module(dir.path()).is_none());
}

#[test]
fn detect_go_module_returns_module_name() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("go.mod"),
        "module github.com/user/myproject\n\ngo 1.21\n",
    )
    .unwrap();
    assert_eq!(
        detect_go_module(dir.path()),
        Some("github.com/user/myproject".to_string())
    );
}

#[test]
fn detect_go_module_no_module_line() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("go.mod"), "go 1.21\n").unwrap();
    assert!(detect_go_module(dir.path()).is_none());
}

// ── run ──────────────────────────────────────────────────────────────────────

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_on_empty_dir_json() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Json, false, "default", 20).unwrap();
}

#[test]
fn run_on_rust_files_no_deps() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn helper() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_on_rust_with_mod_declaration() {
    let dir = tempfile::tempdir().unwrap();
    // lib.rs declares mod foo
    fs::write(dir.path().join("lib.rs"), "mod foo;\n\npub fn bar() {}\n").unwrap();
    // foo.rs exists
    fs::write(dir.path().join("foo.rs"), "pub fn foo_fn() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_sort_by_fan_in() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "fan-in", 20).unwrap();
}

#[test]
fn run_sort_by_fan_out() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "fan-out", 20).unwrap();
}

#[test]
fn run_cycles_only_filter() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, true, "default", 20).unwrap();
}

#[test]
fn run_cycles_only_json() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Json, true, "default", 20).unwrap();
}

#[test]
fn run_with_go_module() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("go.mod"),
        "module github.com/example/project\n\ngo 1.21\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.go"),
        "package main\n\nfunc main() {}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_unreadable_file_gracefully_handled() {
    // Write a file with null bytes so read_to_string fails
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("binary.rs"), b"fn main() \x00{}").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should not panic — errors are handled gracefully
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_json_with_mod_deps() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("lib.rs"), "mod foo;\n").unwrap();
    fs::write(dir.path().join("foo.rs"), "pub fn f() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Json, false, "default", 20).unwrap();
}
