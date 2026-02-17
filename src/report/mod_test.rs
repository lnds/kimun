use super::*;
use std::fs;

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), false, false, 20, 6).unwrap();
}

#[test]
fn run_on_empty_dir_json() {
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), true, false, 20, 6).unwrap();
}

#[test]
fn run_on_rust_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    run(dir.path(), false, false, 20, 6).unwrap();
}

#[test]
fn run_json_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    run(dir.path(), true, false, 20, 6).unwrap();
}

#[test]
fn run_skips_binary() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
    run(dir.path(), false, false, 20, 6).unwrap();
}

// --- Tests that verify actual report structure ---

#[test]
fn build_report_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let report = build_report(dir.path(), false, 20, 6).unwrap();
    assert!(report.loc.is_empty());
    assert_eq!(report.duplication.total_code_lines, 0);
    assert_eq!(report.indent.total_count, 0);
    assert!(report.indent.entries.is_empty());
}

#[test]
fn build_report_counts_loc() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "// comment\nfn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let report = build_report(dir.path(), false, 20, 6).unwrap();

    assert_eq!(report.loc.len(), 1);
    assert_eq!(report.loc[0].name, "Rust");
    assert_eq!(report.loc[0].files, 1);
    assert_eq!(report.loc[0].comment, 1);
    assert_eq!(report.loc[0].code, 3);
}

#[test]
fn build_report_no_dedup_for_loc() {
    let dir = tempfile::tempdir().unwrap();
    let content = "fn foo() {\n    let x = 1;\n}\n";
    fs::write(dir.path().join("a.rs"), content).unwrap();
    fs::write(dir.path().join("b.rs"), content).unwrap();
    let report = build_report(dir.path(), false, 20, 6).unwrap();

    // Both files counted — no dedup in report
    assert_eq!(report.loc[0].files, 2);
}

#[test]
fn build_report_detects_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    // 7 code lines per file, all identical
    let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";
    fs::write(dir.path().join("a.rs"), code).unwrap();
    fs::write(dir.path().join("b.rs"), code).unwrap();
    let report = build_report(dir.path(), false, 20, 6).unwrap();

    // 1 duplicate group of 7 lines across 2 files
    assert_eq!(report.duplication.duplicate_groups, 1);
    // duplicated_lines = line_count * (locations - 1) = 7 * 1 = 7
    assert_eq!(report.duplication.duplicated_lines, 7);
    // total_code_lines = 7 * 2 files = 14
    assert_eq!(report.duplication.total_code_lines, 14);
    assert!((report.duplication.duplication_percentage - 50.0).abs() < 0.1);
    assert_eq!(report.duplication.files_with_duplicates, 2);
    assert_eq!(report.duplication.largest_block, 7);
}

#[test]
fn build_report_top_truncates() {
    let dir = tempfile::tempdir().unwrap();
    // Create 3 files with different content so all produce indent results
    fs::write(dir.path().join("a.rs"), "fn a() {\n    let x = 1;\n}\n").unwrap();
    fs::write(
        dir.path().join("b.rs"),
        "fn b() {\n    let x = 1;\n    let y = 2;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("c.rs"),
        "fn c() {\n    let x = 1;\n    let y = 2;\n    let z = 3;\n}\n",
    )
    .unwrap();
    let report = build_report(dir.path(), false, 2, 6).unwrap();

    // 3 files analyzed, but only top 2 shown
    assert_eq!(report.indent.total_count, 3);
    assert_eq!(report.indent.entries.len(), 2);
}

#[test]
fn build_report_full_shows_all() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.rs"), "fn a() {\n    let x = 1;\n}\n").unwrap();
    fs::write(
        dir.path().join("b.rs"),
        "fn b() {\n    let x = 1;\n    let y = 2;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("c.rs"),
        "fn c() {\n    let x = 1;\n    let y = 2;\n    let z = 3;\n}\n",
    )
    .unwrap();
    let report = build_report(dir.path(), false, usize::MAX, 6).unwrap();

    // all 3 files shown when top is usize::MAX (--full mode)
    assert_eq!(report.indent.total_count, 3);
    assert_eq!(report.indent.entries.len(), 3);
}

#[test]
fn build_report_excludes_tests() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("tests/integration.rs"),
        "fn test() {\n    assert!(true);\n}\n",
    )
    .unwrap();
    let report = build_report(dir.path(), false, 20, 6).unwrap();

    // Test file in tests/ dir should be excluded
    assert!(report.loc.is_empty());
}

#[test]
fn build_report_includes_tests_with_flag() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("tests/integration.rs"),
        "fn test() {\n    assert!(true);\n}\n",
    )
    .unwrap();
    let report = build_report(dir.path(), true, 20, 6).unwrap();

    assert!(!report.loc.is_empty());
}

#[test]
fn build_report_mi_computed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    let report = build_report(dir.path(), false, 20, 6).unwrap();

    assert_eq!(report.mi_visual_studio.entries.len(), 1);
    assert_eq!(report.mi_verifysoft.entries.len(), 1);

    let vs = &report.mi_visual_studio.entries[0];
    // VS MI ~71.07 for this simple function — green level
    assert!((vs.mi_score - 71.07).abs() < 1.0);
    assert_eq!(vs.level, "green");

    let vf = &report.mi_verifysoft.entries[0];
    // Verifysoft MI ~121.54 — good level (no comments, so MIcw is zero)
    assert!((vf.mi_score - 121.54).abs() < 1.0);
    assert_eq!(vf.level, "good");
}

#[test]
fn build_report_json_structure() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "// comment\nfn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let report = build_report(dir.path(), false, 20, 6).unwrap();

    // Serialize to JSON and parse back to verify structure
    let json_str = serde_json::to_string(&report).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert!(value["path"].is_string());
    assert_eq!(value["top"], 20);
    assert_eq!(value["include_tests"], false);
    assert_eq!(value["min_lines"], 6);
    assert!(value["loc"].is_array());
    assert!(value["duplication"]["total_code_lines"].is_number());
    assert!(value["indent"]["total_count"].is_number());
    assert!(value["indent"]["entries"].is_array());
    assert!(value["halstead"]["total_count"].is_number());
    assert!(value["cyclomatic"]["total_count"].is_number());
    assert!(value["mi_visual_studio"]["total_count"].is_number());
    assert!(value["mi_verifysoft"]["total_count"].is_number());
}

#[test]
fn build_report_min_lines_affects_dups() {
    let dir = tempfile::tempdir().unwrap();
    // 5 code lines per file
    let code = "fn f() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}\n";
    fs::write(dir.path().join("a.rs"), code).unwrap();
    fs::write(dir.path().join("b.rs"), code).unwrap();

    // min_lines=3: block of 5 lines >= 3, so duplicates detected
    let report_low = build_report(dir.path(), false, 20, 3).unwrap();
    assert!(report_low.duplication.duplicate_groups > 0);

    // min_lines=100: block of 5 lines < 100, so no duplicates detected
    let report_high = build_report(dir.path(), false, 20, 100).unwrap();
    assert_eq!(report_high.duplication.duplicate_groups, 0);
}
