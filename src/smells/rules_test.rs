use crate::detection::{FunctionDetectionMarkers, FunctionSyntax};
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
    fn function_syntax(&self) -> FunctionSyntax {
        FunctionSyntax::KeywordWithLifetimes
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

#[test]
fn debt_keyword_not_matched_as_substring_in_prose() {
    // Descriptive comment prose containing "debugging" and "bug report" —
    // must NOT fire the todo_debt smell (lowercase `bug` in English prose
    // is a common word, not a debt marker).
    let ls = lines("# harmless prose: this is about debugging a bug report from a customer");
    let kinds = vec![LineKind::Comment];
    let smells = detect_todo_debt(&ls, &kinds);
    assert!(
        smells.is_empty(),
        "prose containing 'bug' as a common English word should not trigger todo_debt: got {:?}",
        smells
    );
}

#[test]
fn debt_keyword_matched_as_whole_word_with_colon() {
    // Canonical `BUG: description` form (uppercase) must still fire.
    let ls = lines("// BUG: race condition when N > 100");
    let kinds = vec![LineKind::Comment];
    let smells = detect_todo_debt(&ls, &kinds);
    assert_eq!(smells.len(), 1);
    assert!(smells[0].detail.contains("BUG"));
}

#[test]
fn debt_keyword_matched_as_whole_word_with_parens() {
    // Canonical `FIXME(user) description` form must still fire.
    let ls = lines("# FIXME(alice) revisit after cache lands");
    let kinds = vec![LineKind::Comment];
    let smells = detect_todo_debt(&ls, &kinds);
    assert_eq!(smells.len(), 1);
    assert!(smells[0].detail.contains("FIXME"));
}

#[test]
fn debt_keyword_not_matched_inside_longer_word() {
    // "hackathon", "todolist", "buggy" — all contain a debt keyword as a
    // substring but shouldn't fire the smell.
    let ls = lines("# hackathon last week\n# my todolist for the sprint\n# fixed a buggy behavior");
    let kinds = vec![LineKind::Comment, LineKind::Comment, LineKind::Comment];
    let smells = detect_todo_debt(&ls, &kinds);
    assert!(
        smells.is_empty(),
        "words containing debt keywords as substrings should not trigger: got {:?}",
        smells
    );
}

#[test]
fn lowercase_bug_in_prose_not_matched() {
    // The uppercase BUG debt marker must not match the lowercase common
    // English noun "bug" — this is the specific case-sensitivity rule for
    // BUG (unlike TODO/FIXME/HACK/XXX which stay case-insensitive since
    // they don't collide with prose).
    let ls = lines("# the bug fix landed yesterday");
    let kinds = vec![LineKind::Comment];
    let smells = detect_todo_debt(&ls, &kinds);
    assert!(smells.is_empty());
}

// ── Magic numbers ──
