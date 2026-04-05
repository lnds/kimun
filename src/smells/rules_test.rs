use crate::detection::FunctionDetectionMarkers;
use crate::loc::counter::LineKind;

use super::*;

/// Minimal markers for testing (Rust-like).
struct TestMarkers;

impl FunctionDetectionMarkers for TestMarkers {
    fn function_markers(&self) -> &[&str] {
        &["fn "]
    }
    fn brace_scoped(&self) -> bool {
        true
    }
    fn line_comments(&self) -> &[&str] {
        &["//"]
    }
}

fn lines(s: &str) -> Vec<String> {
    s.lines().map(String::from).collect()
}

fn all_code(n: usize) -> Vec<LineKind> {
    vec![LineKind::Code; n]
}

// ── Long function ──

#[test]
fn long_function_detected() {
    // fn foo() { + 53 body lines + } = 55 total, body = 53
    let mut src = String::from("fn foo() {\n");
    for i in 0..53 {
        src.push_str(&format!("    let x{i} = {i};\n"));
    }
    src.push_str("}\n");

    let ls = lines(&src);
    let kinds = all_code(ls.len());
    let smells = detect_long_functions(&ls, &kinds, &TestMarkers, 50);
    assert_eq!(smells.len(), 1);
    assert_eq!(smells[0].kind, SmellKind::LongFunction);
    assert!(smells[0].detail.contains("foo"));
    assert!(smells[0].detail.contains("53 lines"));
}

#[test]
fn long_function_body_excludes_signature_and_closing() {
    // fn bar() { + 50 body lines + } = body is exactly 50, should NOT trigger
    let mut src = String::from("fn bar() {\n");
    for i in 0..50 {
        src.push_str(&format!("    let x{i} = {i};\n"));
    }
    src.push_str("}\n");

    let ls = lines(&src);
    let kinds = all_code(ls.len());
    let smells = detect_long_functions(&ls, &kinds, &TestMarkers, 50);
    assert!(
        smells.is_empty(),
        "50 body lines should not trigger at max_lines=50"
    );
}

#[test]
fn short_function_not_detected() {
    let src = "fn bar() {\n    let x = 1;\n}\n";
    let ls = lines(src);
    let kinds = all_code(ls.len());
    let smells = detect_long_functions(&ls, &kinds, &TestMarkers, 50);
    assert!(smells.is_empty());
}

// ── Long parameter list ──

#[test]
fn long_params_detected() {
    let src = "fn baz(a: i32, b: i32, c: i32, d: i32, e: i32) {\n}\n";
    let ls = lines(src);
    let kinds = all_code(ls.len());
    let smells = detect_long_params(&ls, &kinds, &TestMarkers, 4);
    assert_eq!(smells.len(), 1);
    assert_eq!(smells[0].kind, SmellKind::LongParameterList);
    assert!(smells[0].detail.contains("5 params"));
}

#[test]
fn long_params_multiline_detected() {
    let src =
        "fn process(\n    a: i32,\n    b: i32,\n    c: i32,\n    d: i32,\n    e: i32,\n) {\n}\n";
    let ls = lines(src);
    let kinds = all_code(ls.len());
    let smells = detect_long_params(&ls, &kinds, &TestMarkers, 4);
    assert_eq!(smells.len(), 1, "multi-line signature should be detected");
    assert!(smells[0].detail.contains("5 params"));
}

#[test]
fn few_params_not_detected() {
    let src = "fn qux(a: i32, b: i32) {\n}\n";
    let ls = lines(src);
    let kinds = all_code(ls.len());
    let smells = detect_long_params(&ls, &kinds, &TestMarkers, 4);
    assert!(smells.is_empty());
}

#[test]
fn no_params_not_detected() {
    let src = "fn empty() {\n}\n";
    let ls = lines(src);
    let kinds = all_code(ls.len());
    let smells = detect_long_params(&ls, &kinds, &TestMarkers, 4);
    assert!(smells.is_empty());
}

// ── TODO/FIXME debt ──

#[test]
fn todo_detected() {
    let ls = lines("// TODO: fix this\nlet x = 1;");
    let kinds = vec![LineKind::Comment, LineKind::Code];
    let smells = detect_todo_debt(&ls, &kinds);
    assert_eq!(smells.len(), 1);
    assert_eq!(smells[0].kind, SmellKind::TodoDebt);
}

#[test]
fn fixme_detected() {
    let ls = lines("// FIXME: broken");
    let kinds = vec![LineKind::Comment];
    let smells = detect_todo_debt(&ls, &kinds);
    assert_eq!(smells.len(), 1);
    assert!(smells[0].detail.contains("FIXME"));
}

#[test]
fn no_debt_in_code() {
    let ls = lines("let todo = 5;");
    let kinds = vec![LineKind::Code];
    let smells = detect_todo_debt(&ls, &kinds);
    assert!(smells.is_empty());
}

// ── Magic numbers ──

#[test]
fn magic_number_detected() {
    let ls = lines("    timeout(3600);");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(smells.len(), 1);
    assert_eq!(smells[0].kind, SmellKind::MagicNumber);
}

#[test]
fn trivial_numbers_not_detected() {
    let ls = lines("    x = 0;\n    y = 1;\n    z = 2;\n    w = -1;");
    let kinds = all_code(4);
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert!(smells.is_empty());
}

#[test]
fn const_declaration_not_detected() {
    let ls = lines("const MAX: i32 = 100;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert!(smells.is_empty());
}

#[test]
fn let_declaration_not_detected() {
    let ls = lines("let threshold = 42;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert!(smells.is_empty());
}

#[test]
fn decl_keyword_not_matched_as_substring() {
    // "deleteable" contains "let " as substring — should NOT be excluded
    let ls = lines("    is_deleteable(3600);");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(
        smells.len(),
        1,
        "substring match should not suppress detection"
    );
}

#[test]
fn hex_literal_detected() {
    let ls = lines("    mask = 0xFF;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(smells.len(), 1);
}

#[test]
fn scientific_notation_detected() {
    let ls = lines("    threshold = 1.5e10;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(smells.len(), 1);
}

#[test]
fn digit_separator_detected_as_magic_number() {
    // `1_000_000` is a non-trivial literal with Rust/Python/Swift digit separators
    let ls = lines("    timeout(1_000_000);");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(smells.len(), 1, "1_000_000 should be detected as a magic number");
}

#[test]
fn hex_with_digit_separator_detected() {
    let ls = lines("    mask = 0xFF_FF;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(smells.len(), 1, "0xFF_FF should be detected as a magic number");
}

// ── Commented-out code ──

#[test]
fn commented_out_code_detected() {
    let ls = lines("// let x = 1;\n// if (y > 0) {\n// return z;\n// }");
    let kinds = vec![
        LineKind::Comment,
        LineKind::Comment,
        LineKind::Comment,
        LineKind::Comment,
    ];
    let smells = detect_commented_out_code(&ls, &kinds);
    assert_eq!(smells.len(), 1);
    assert_eq!(smells[0].kind, SmellKind::CommentedOutCode);
}

#[test]
fn regular_comments_not_detected() {
    let ls = lines("// This is a description\n// of the module behavior");
    let kinds = vec![LineKind::Comment, LineKind::Comment];
    let smells = detect_commented_out_code(&ls, &kinds);
    assert!(smells.is_empty());
}

#[test]
fn single_commented_code_line_not_detected() {
    let ls = lines("// let x = 1;");
    let kinds = vec![LineKind::Comment];
    let smells = detect_commented_out_code(&ls, &kinds);
    assert!(smells.is_empty());
}

#[test]
fn doc_comment_with_use_not_false_positive() {
    let ls = lines("/// use std::io::Read;\n/// use tokio::runtime::Runtime;");
    let kinds = vec![LineKind::Comment, LineKind::Comment];
    let smells = detect_commented_out_code(&ls, &kinds);
    assert!(
        smells.is_empty(),
        "doc comments with `use` should not trigger"
    );
}
