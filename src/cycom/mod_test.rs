use super::*;
use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 1, Some(20), false, "total").unwrap();
}

#[test]
fn run_on_rust_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 1, Some(20), false, "total").unwrap();
}

#[test]
fn run_on_python_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("app.py"),
        "def main():\n    if True:\n        print(\"hi\")\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 1, Some(20), false, "total").unwrap();
}

#[test]
fn run_skips_binary() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 1, Some(20), false, "total").unwrap();
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
    run(&cfg, OutputMode::Table, 1, Some(20), false, "total").unwrap();
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
    run(&cfg, OutputMode::Json, 1, Some(20), false, "total").unwrap();
}

#[test]
fn run_per_function_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\nfn baz() {\n    quux();\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 1, Some(20), true, "total").unwrap();
}

#[test]
fn run_min_complexity_filter() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("simple.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    // min_complexity=5 should filter out simple functions
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 5, Some(20), false, "total").unwrap();
}

#[test]
fn run_top_limit() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("a.rs"),
        "fn a() {\n    if x { bar(); }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("b.rs"),
        "fn b() {\n    if y { baz(); }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 1, Some(1), false, "total").unwrap();
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
    run(&cfg, OutputMode::Table, 1, Some(20), false, "total").unwrap();
}

#[test]
fn run_skips_non_code_languages() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.json"), "{\"key\": \"value\"}\n").unwrap();
    fs::write(dir.path().join("style.css"), "body { color: red; }\n").unwrap();
    // Should produce no results (JSON/CSS have no markers)
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 1, Some(20), false, "total").unwrap();
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
    run(&cfg, OutputMode::Short, 0, Some(20), false, "total").unwrap();
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
    run(&cfg, OutputMode::Terse, 0, Some(20), false, "total").unwrap();
}

#[test]
fn run_codeclimate_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Codeclimate, 1, Some(20), false, "total").unwrap();
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
    run(&cfg, OutputMode::Github, 1, Some(20), false, "total").unwrap();
}

#[test]
fn analyze_file_leaves_inline_tests_out_when_they_are_excluded() {
    let (_dir, path) = crate::test_blocks::tests::source_with_test_module();
    let spec = crate::loc::language::detect(&path).unwrap();
    let production = analyze_file(&path, spec, true).unwrap().unwrap();
    let everything = analyze_file(&path, spec, false).unwrap().unwrap();
    assert_eq!(production.function_count, 1);
    assert_eq!(everything.function_count, 2);
    assert!(production.total_complexity < everything.total_complexity);
}

#[test]
fn text_inside_raw_strings_is_no_code() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib.rs");
    let src = "\
fn text() -> &'static str {
    r#\"
fn fake(a: u32) {
    if a > 1 && a < 3 { }
}
\"#
}

fn json() -> &'static str {
    r#\"{ \"if\": \"while {\" }\"#
}

fn real(n: u32) -> u32 {
    if n > 1 { 2 } else { 3 }
}
";
    fs::write(&path, src).unwrap();
    let spec = crate::loc::language::detect(&path).unwrap();
    let file = analyze_file(&path, spec, true).unwrap().unwrap();
    let found: Vec<(&str, usize)> = file
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.complexity))
        .collect();
    assert_eq!(found, [("text", 1), ("json", 1), ("real", 2)]);
}

#[test]
fn case_insensitive_languages_count_keywords_in_any_case_and_keep_names() {
    use crate::loc::language::detect;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("u.pas");
    fs::write(
        &path,
        "unit U;\ninterface\nimplementation\nprocedure TCounter.Bump(X: Integer);\nBEGIN\n  IF X > 0 THEN Inc(X)\n  ELSE If X < -1 Then Dec(X);\n  While X > 9 do X := X Div 2;\nEND;\nend.\n",
    )
    .unwrap();
    let metrics = analyze_file(&path, detect(&path).unwrap(), false)
        .unwrap()
        .unwrap();
    assert_eq!(metrics.functions.len(), 1);
    assert_eq!(metrics.functions[0].name, "TCounter.Bump");
    // if, else if, while
    assert_eq!(metrics.functions[0].complexity, 4);
}

#[test]
fn fortran_ends_of_constructs_are_no_branches() {
    use crate::loc::language::detect;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.f90");
    fs::write(
        &path,
        "SUBROUTINE Walk(N)\n  IF (N > 0 .AND. N < 9) THEN\n    CALL Walk(N - 1)\n  END IF\n  DO I = 1, N\n  END DO\n  SELECT CASE (N)\n  CASE (1)\n  END SELECT\nEND SUBROUTINE Walk\n",
    )
    .unwrap();
    let metrics = analyze_file(&path, detect(&path).unwrap(), false)
        .unwrap()
        .unwrap();
    assert_eq!(metrics.functions[0].name, "Walk");
    // if, .and., do, select case, case
    assert_eq!(metrics.functions[0].complexity, 6);
}

/// Name and complexity of each function of `src`, written in a file named
/// `name`.
fn complexities(name: &str, src: &str) -> Vec<(String, usize)> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    fs::write(&path, src).unwrap();
    let spec = crate::loc::language::detect(&path).unwrap();
    let file = analyze_file(&path, spec, true).unwrap().unwrap();
    file.functions
        .iter()
        .map(|f| (f.name.clone(), f.complexity))
        .collect()
}

fn named(pairs: &[(&str, usize)]) -> Vec<(String, usize)> {
    pairs.iter().map(|(n, c)| (n.to_string(), *c)).collect()
}

#[test]
fn loop_labels_and_strings_on_several_lines_hide_no_code() {
    let src = "pub fn labeled(xs: &[Vec<i32>]) -> i32 {
    let mut s = 0;
    'outer: for row in xs {
        'inner: while s < 10 {
            if row.is_empty() {
                break 'outer;
            }
            s += 1;
        }
    }
    s
}

pub fn unlabeled(xs: &[Vec<i32>]) -> i32 {
    let mut s = 0;
    for row in xs {
        while s < 10 {
            if row.is_empty() {
                break;
            }
            s += 1;
        }
    }
    s
}

pub fn msg(x: i32) -> String {
    let m = \"this is shown if the value is out of range, \\
             or while waiting for the match to loop\";
    format!(\"{m} {x}\")
}
";
    assert_eq!(
        complexities("lbl.rs", src),
        named(&[("labeled", 4), ("unlabeled", 4), ("msg", 1)])
    );
}

#[test]
fn a_brace_in_a_template_literal_opens_no_block() {
    let src = "function open(x) {
  const s = `{`;
  if (x) return s;
  return '';
}

function second(x) {
  if (x && x.y) return 1;
  return 0;
}

function third(x) {
  while (x) x--;
}
";
    assert_eq!(
        complexities("brace.js", src),
        named(&[("open", 2), ("second", 3), ("third", 2)])
    );
}

#[test]
fn a_url_in_a_template_literal_opens_no_comment() {
    let src = "function load(id, cb) {
  fetch(`https://api.example.com/items/${id}`).then((r) => {
    if (r.ok) {
      cb(r);
    }
  });
  if (id > 10) {
    return 1;
  }
  for (const x of id) {
    if (x) return 2;
  }
  return 0;
}

function next(x) {
  if (x) return 1;
  return 0;
}
";
    assert_eq!(
        complexities("url.js", src),
        named(&[("load", 5), ("next", 2)])
    );
}

#[test]
fn a_brace_in_a_go_raw_string_opens_no_block() {
    let src = "package a

func Open(x bool) string {
	s := `{\"a\": [`
	if x {
		return s
	}
	return \"\"
}

func Second(x, y bool) int {
	if x && y {
		return 1
	}
	return 0
}
";
    assert_eq!(
        complexities("brace.go", src),
        named(&[("Open", 2), ("Second", 3)])
    );
}

#[test]
fn a_block_comment_after_code_counts_nothing() {
    let src = "int inline_comment(int x)
{
    int y = x; /* if (y) case */
    return y;  /* && || */
}
";
    assert_eq!(complexities("a.c", src), named(&[("inline_comment", 1)]));
}
