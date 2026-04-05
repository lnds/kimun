use std::io::Write;
use std::path::PathBuf;

use tempfile::tempdir;

use super::analyzer::SmellKind;

#[test]
fn finds_long_function_with_correct_count() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("example.rs");

    let mut f = std::fs::File::create(&file).unwrap();
    writeln!(f, "fn long_func() {{").unwrap();
    // 55 body lines > default 50
    for i in 0..55 {
        writeln!(f, "    let x{i} = {i};").unwrap();
    }
    writeln!(f, "}}").unwrap();

    let spec = crate::loc::language::detect(&file).unwrap();
    let result = super::analyze_file(&file, spec, 50, 4).unwrap().unwrap();

    assert!(
        result.total >= 1,
        "expected at least 1 smell, got {}",
        result.total
    );
    let long_fns: Vec<_> = result
        .smells
        .smells
        .iter()
        .filter(|s| s.kind == SmellKind::LongFunction)
        .collect();
    assert_eq!(long_fns.len(), 1);
    assert!(long_fns[0].detail.contains("long_func"));
}

#[test]
fn finds_todo_debt_in_comments() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("debt.rs");

    let mut f = std::fs::File::create(&file).unwrap();
    writeln!(f, "fn main() {{").unwrap();
    writeln!(f, "    // TODO: fix this").unwrap();
    writeln!(f, "    // FIXME: also broken").unwrap();
    writeln!(f, "    let x = 1;").unwrap();
    writeln!(f, "}}").unwrap();

    let spec = crate::loc::language::detect(&file).unwrap();
    let result = super::analyze_file(&file, spec, 50, 4).unwrap().unwrap();

    let todos: Vec<_> = result
        .smells
        .smells
        .iter()
        .filter(|s| s.kind == SmellKind::TodoDebt)
        .collect();
    assert_eq!(todos.len(), 2);
    assert!(todos[0].detail.contains("TODO"));
    assert!(todos[1].detail.contains("FIXME"));
}

#[test]
fn finds_long_params_single_line() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("params.rs");

    let mut f = std::fs::File::create(&file).unwrap();
    writeln!(f, "fn many(a: i32, b: i32, c: i32, d: i32, e: i32) {{").unwrap();
    writeln!(f, "    let x = a;").unwrap();
    writeln!(f, "}}").unwrap();

    let spec = crate::loc::language::detect(&file).unwrap();
    let result = super::analyze_file(&file, spec, 50, 4).unwrap().unwrap();

    let params: Vec<_> = result
        .smells
        .smells
        .iter()
        .filter(|s| s.kind == SmellKind::LongParameterList)
        .collect();
    assert_eq!(params.len(), 1);
    assert!(params[0].detail.contains("5 params"));
}

#[test]
fn finds_long_params_multiline() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("multiline.rs");

    let mut f = std::fs::File::create(&file).unwrap();
    writeln!(f, "fn process(").unwrap();
    writeln!(f, "    a: i32,").unwrap();
    writeln!(f, "    b: i32,").unwrap();
    writeln!(f, "    c: i32,").unwrap();
    writeln!(f, "    d: i32,").unwrap();
    writeln!(f, "    e: i32,").unwrap();
    writeln!(f, ") {{").unwrap();
    writeln!(f, "    let x = a;").unwrap();
    writeln!(f, "}}").unwrap();

    let spec = crate::loc::language::detect(&file).unwrap();
    let result = super::analyze_file(&file, spec, 50, 4).unwrap().unwrap();

    let params: Vec<_> = result
        .smells
        .smells
        .iter()
        .filter(|s| s.kind == SmellKind::LongParameterList)
        .collect();
    assert_eq!(params.len(), 1, "multi-line signature should be detected");
    assert!(params[0].detail.contains("5 params"));
}

#[test]
fn no_smells_for_clean_file() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("clean.rs");

    let mut f = std::fs::File::create(&file).unwrap();
    writeln!(f, "fn add(a: i32, b: i32) -> i32 {{").unwrap();
    writeln!(f, "    a + b").unwrap();
    writeln!(f, "}}").unwrap();

    let spec = crate::loc::language::detect(&file).unwrap();
    let result = super::analyze_file(&file, spec, 50, 4).unwrap();
    assert!(result.is_none(), "clean file should have no smells");
}

#[test]
fn json_output_runs_without_error() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.rs");

    let mut f = std::fs::File::create(&file).unwrap();
    writeln!(f, "fn small() {{}}").unwrap();

    let filter = crate::walk::ExcludeFilter::default();
    let cfg = crate::walk::WalkConfig::new(dir.path(), true, &filter);
    let result = super::run(&cfg, true, 20, 50, 4, None);
    assert!(result.is_ok());
}

#[test]
fn doc_comments_with_use_not_false_positive() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("doc.rs");

    let mut f = std::fs::File::create(&file).unwrap();
    writeln!(f, "/// use std::io::Read;").unwrap();
    writeln!(f, "/// use tokio::runtime::Runtime;").unwrap();
    writeln!(f, "fn main() {{}}").unwrap();

    let spec = crate::loc::language::detect(&file).unwrap();
    let result = super::analyze_file(&file, spec, 50, 4).unwrap();
    // Should not detect commented-out code for doc comments with `use`
    if let Some(ref metrics) = result {
        let commented: Vec<_> = metrics
            .smells
            .smells
            .iter()
            .filter(|s| s.kind == SmellKind::CommentedOutCode)
            .collect();
        assert!(
            commented.is_empty(),
            "doc comments with `use` should not trigger commented-out code detection"
        );
    }
}

#[test]
fn run_on_files_with_smell() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("big.rs");
    let mut f = std::fs::File::create(&file).unwrap();
    writeln!(f, "fn big() {{").unwrap();
    for i in 0..55 {
        writeln!(f, "    let x{i} = {i};").unwrap();
    }
    writeln!(f, "}}").unwrap();

    let result = super::run_on_files(&[file], false, 20, 50, 4, None);
    assert!(result.is_ok());
}

#[test]
fn run_on_files_empty_list() {
    let result = super::run_on_files(&[], false, 20, 50, 4, None);
    assert!(result.is_ok());
}

#[test]
fn run_on_files_skips_unknown_extension() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("data.xyz");
    std::fs::write(&file, "hello world").unwrap();

    let result = super::run_on_files(&[file], false, 20, 50, 4, None);
    assert!(result.is_ok());
}

#[test]
fn run_on_files_json_empty() {
    let result = super::run_on_files(&[], true, 20, 50, 4, None);
    assert!(result.is_ok());
}

#[test]
fn run_on_files_nonexistent_path_skips() {
    let paths = vec![PathBuf::from("/nonexistent/path/fake.rs")];
    let result = super::run_on_files(&paths, false, 20, 50, 4, None);
    assert!(result.is_ok(), "nonexistent files should be skipped, not panic");
}
