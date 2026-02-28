use super::*;

fn rust_markers() -> &'static CognitiveMarkers {
    super::super::markers::cognitive_markers_for("Rust").unwrap()
}

fn python_markers() -> &'static CognitiveMarkers {
    super::super::markers::cognitive_markers_for("Python").unwrap()
}

fn c_markers() -> &'static CognitiveMarkers {
    super::super::markers::cognitive_markers_for("C").unwrap()
}

fn make_lines(code: &str) -> (Vec<String>, Vec<LineKind>) {
    let lines: Vec<String> = code.lines().map(String::from).collect();
    let kinds = vec![LineKind::Code; lines.len()];
    (lines, kinds)
}

#[test]
fn simple_function_no_branches() {
    let (lines, kinds) = make_lines("fn main() {\n    let x = 1;\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].complexity, 0);
    assert_eq!(result.functions[0].level, CognitiveLevel::Simple);
}

#[test]
fn simple_if() {
    // if: +1 (nesting=0) → total 1
    let (lines, kinds) = make_lines("fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 1);
}

#[test]
fn nested_if_in_if() {
    // outer if: +1 (nesting=0)
    // inner if: +1 + 1 (nesting=1) = +2
    // total: 3
    let (lines, kinds) = make_lines(
        "fn foo() {\n    if x > 0 {\n        if y > 0 {\n            bar();\n        }\n    }\n}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 3);
}

#[test]
fn if_else_if_else() {
    // if: +1 (structural, nesting=0)
    // else if: +1 (hybrid, no nesting)
    // else: +1 (fundamental)
    // total: 3
    let (lines, kinds) = make_lines(
        "fn foo() {\n    if x > 0 {\n        a();\n    } else if y > 0 {\n        b();\n    } else {\n        c();\n    }\n}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 3);
}

#[test]
fn boolean_sequence_same_operator() {
    // if: +1 (nesting=0)
    // && && : +1 (one sequence of same operator)
    // total: 2
    let (lines, kinds) = make_lines("fn foo() {\n    if a && b && c {\n        bar();\n    }\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 2);
}

#[test]
fn boolean_sequence_mixed_operators() {
    // if: +1 (nesting=0)
    // && then ||: +2 (first operator +1, change +1)
    // total: 3
    let (lines, kinds) = make_lines("fn foo() {\n    if a && b || c {\n        bar();\n    }\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 3);
}

#[test]
fn triple_nesting_if_for_if() {
    // if: +1 (nesting=0)
    // for: +1 + 1 (nesting=1) = +2
    // if: +1 + 2 (nesting=2) = +3
    // total: 6
    let (lines, kinds) = make_lines(
        "fn foo() {\n    if x > 0 {\n        for i in items {\n            if y > 0 {\n                bar();\n            }\n        }\n    }\n}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 6);
}

#[test]
fn keywords_in_strings_not_counted() {
    let (lines, kinds) = make_lines(
        "fn foo() {\n    let kw = [\"if\", \"for\", \"while\", \"match\"];\n    let s = \"if x && y || z\";\n}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 0);
}

#[test]
fn python_indent_scoped() {
    // if: +1 (nesting=0)
    // for: +1 + 1 (nesting=1) = +2
    // total: 3
    let code = "def foo():\n    if x > 0:\n        for i in items:\n            bar()\n";
    let (lines, kinds) = make_lines(code);
    let result = analyze(&lines, &kinds, python_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 3);
}

#[test]
fn python_elif() {
    // if: +1 (structural)
    // elif: +1 (hybrid, no nesting)
    // else: +1 (fundamental)
    // total: 3
    let code = "def foo():\n    if x > 0:\n        a()\n    elif y > 0:\n        b()\n    else:\n        c()\n";
    let (lines, kinds) = make_lines(code);
    let result = analyze(&lines, &kinds, python_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 3);
}

#[test]
fn two_functions() {
    let (lines, kinds) =
        make_lines("fn foo() {\n    if x > 0 {\n        a();\n    }\n}\nfn bar() {\n    b();\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions.len(), 2);
    assert_eq!(result.functions[0].name, "foo");
    assert_eq!(result.functions[0].complexity, 1);
    assert_eq!(result.functions[1].name, "bar");
    assert_eq!(result.functions[1].complexity, 0);
}

#[test]
fn empty_input_returns_none() {
    assert!(analyze(&[], &[], rust_markers()).is_none());
}

#[test]
fn all_comments_returns_none() {
    let lines = vec!["// comment".to_string()];
    let kinds = vec![LineKind::Comment];
    assert!(analyze(&lines, &kinds, rust_markers()).is_none());
}

#[test]
fn threshold_boundaries() {
    assert_eq!(CognitiveLevel::from_complexity(0), CognitiveLevel::Simple);
    assert_eq!(CognitiveLevel::from_complexity(4), CognitiveLevel::Simple);
    assert_eq!(CognitiveLevel::from_complexity(5), CognitiveLevel::Moderate);
    assert_eq!(CognitiveLevel::from_complexity(9), CognitiveLevel::Moderate);
    assert_eq!(CognitiveLevel::from_complexity(10), CognitiveLevel::Complex);
    assert_eq!(CognitiveLevel::from_complexity(14), CognitiveLevel::Complex);
    assert_eq!(
        CognitiveLevel::from_complexity(15),
        CognitiveLevel::VeryComplex
    );
    assert_eq!(
        CognitiveLevel::from_complexity(24),
        CognitiveLevel::VeryComplex
    );
    assert_eq!(CognitiveLevel::from_complexity(25), CognitiveLevel::Extreme);
}

#[test]
fn level_display() {
    assert_eq!(CognitiveLevel::Simple.as_str(), "simple");
    assert_eq!(CognitiveLevel::Moderate.as_str(), "moderate");
    assert_eq!(CognitiveLevel::Complex.as_str(), "complex");
    assert_eq!(CognitiveLevel::VeryComplex.as_str(), "very complex");
    assert_eq!(CognitiveLevel::Extreme.as_str(), "extreme");
}

#[test]
fn level_serde() {
    assert_eq!(
        serde_json::to_string(&CognitiveLevel::VeryComplex).unwrap(),
        "\"very_complex\""
    );
}

#[test]
fn file_with_no_functions_uses_implicit() {
    let (lines, kinds) = make_lines("let x = 1;\nif true { foo(); }\n");
    let markers = super::super::markers::cognitive_markers_for("Haskell").unwrap();
    let result = analyze(&lines, &kinds, markers).unwrap();
    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].name, "<file>");
}

#[test]
fn aggregation_stats() {
    let (lines, kinds) =
        make_lines("fn foo() {\n    if x > 0 {\n        a();\n    }\n}\nfn bar() {\n    b();\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.total_complexity, 1); // foo=1, bar=0
    assert_eq!(result.max_complexity, 1);
    assert!((result.avg_complexity - 0.5).abs() < 0.01);
}

#[test]
fn c_family_function_detection() {
    let (lines, kinds) = make_lines(
        "int main(int argc, char *argv[]) {\n    if (argc > 1) {\n        printf(\"hi\");\n    }\n    return 0;\n}\n",
    );
    let result = analyze(&lines, &kinds, c_markers()).unwrap();
    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].name, "main");
    assert_eq!(result.functions[0].complexity, 1);
}

#[test]
fn match_with_nested_if() {
    // match: +1 (nesting=0)
    // if: +1 + 1 (nesting=1) = +2
    // total: 3
    let (lines, kinds) = make_lines(
        "fn foo() {\n    match x {\n        1 => {\n            if y > 0 {\n                bar();\n            }\n        }\n        _ => {}\n    }\n}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 3);
}

#[test]
fn while_loop() {
    // while: +1 (nesting=0)
    let (lines, kinds) = make_lines("fn foo() {\n    while x > 0 {\n        x -= 1;\n    }\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 1);
}

#[test]
fn loop_keyword() {
    // loop: +1 (nesting=0)
    let (lines, kinds) = make_lines("fn foo() {\n    loop {\n        break;\n    }\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 1);
}

#[test]
fn word_boundary_notify_not_if() {
    let (lines, kinds) = make_lines("fn foo() {\n    notify();\n    ifdef();\n    life();\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 0);
}

#[test]
fn boolean_sequence_reset_at_semicolon() {
    // Two separate boolean expressions separated by semicolons
    // if a && b: +1 (if) +1 (&&) = 2
    // if c || d: +1 (if) +1 (||) = 2
    // total: 4
    let (lines, kinds) =
        make_lines("fn foo() {\n    if a && b { x(); }\n    if c || d { y(); }\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 4);
}
