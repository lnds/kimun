use super::*;

fn rust_markers() -> &'static ComplexityMarkers {
    super::super::markers::markers_for("Rust").unwrap()
}

fn python_markers() -> &'static ComplexityMarkers {
    super::super::markers::markers_for("Python").unwrap()
}

fn c_markers() -> &'static ComplexityMarkers {
    super::super::markers::markers_for("C").unwrap()
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
    assert_eq!(result.functions[0].complexity, 1);
    assert_eq!(result.functions[0].level, CyclomaticLevel::Simple);
}

#[test]
fn function_with_if() {
    let (lines, kinds) = make_lines("fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 2);
}

#[test]
fn function_with_if_and_and() {
    let (lines, kinds) =
        make_lines("fn foo() {\n    if x > 0 && y > 0 {\n        bar();\n    }\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 3);
}

#[test]
fn nested_if_and_for() {
    let (lines, kinds) = make_lines(
        "fn foo() {\n    if x > 0 {\n        for i in items {\n            bar();\n        }\n    }\n}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 3);
}

#[test]
fn word_boundary_notify_not_if() {
    let (lines, kinds) = make_lines("fn foo() {\n    notify();\n    ifdef();\n    life();\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 1);
}

#[test]
fn else_if_counts_as_one() {
    let (lines, kinds) = make_lines(
        "fn foo() {\n    if x > 0 {\n        a();\n    } else if y > 0 {\n        b();\n    }\n}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 3);
}

#[test]
fn two_rust_functions() {
    let (lines, kinds) =
        make_lines("fn foo() {\n    if x > 0 {\n        a();\n    }\n}\nfn bar() {\n    b();\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions.len(), 2);
    assert_eq!(result.functions[0].complexity, 2);
    assert_eq!(result.functions[0].name, "foo");
    assert_eq!(result.functions[1].complexity, 1);
    assert_eq!(result.functions[1].name, "bar");
}

#[test]
fn python_function_by_indent() {
    let (lines, kinds) =
        make_lines("def foo():\n    if x > 0:\n        bar()\n\ndef baz():\n    pass\n");
    let mut kinds = kinds;
    kinds[3] = LineKind::Blank;
    let result = analyze(&lines, &kinds, python_markers()).unwrap();
    assert_eq!(result.functions.len(), 2);
    assert_eq!(result.functions[0].name, "foo");
    assert_eq!(result.functions[0].complexity, 2);
    assert_eq!(result.functions[1].name, "baz");
    assert_eq!(result.functions[1].complexity, 1);
}

#[test]
fn c_family_function_detection() {
    let (lines, kinds) = make_lines(
        "int main(int argc, char *argv[]) {\n    if (argc > 1) {\n        printf(\"hi\");\n    }\n    return 0;\n}\n",
    );
    let result = analyze(&lines, &kinds, c_markers()).unwrap();
    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].name, "main");
    assert_eq!(result.functions[0].complexity, 2);
}

#[test]
fn file_with_no_functions_uses_implicit() {
    let (lines, kinds) = make_lines("let x = 1;\nif true { foo(); }\n");
    let markers = super::super::markers::markers_for("Haskell").unwrap();
    let result = analyze(&lines, &kinds, markers).unwrap();
    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].name, "<file>");
}

#[test]
fn threshold_boundaries() {
    assert_eq!(CyclomaticLevel::from_complexity(1), CyclomaticLevel::Simple);
    assert_eq!(CyclomaticLevel::from_complexity(5), CyclomaticLevel::Simple);
    assert_eq!(
        CyclomaticLevel::from_complexity(6),
        CyclomaticLevel::Moderate
    );
    assert_eq!(
        CyclomaticLevel::from_complexity(10),
        CyclomaticLevel::Moderate
    );
    assert_eq!(
        CyclomaticLevel::from_complexity(11),
        CyclomaticLevel::Complex
    );
    assert_eq!(
        CyclomaticLevel::from_complexity(20),
        CyclomaticLevel::Complex
    );
    assert_eq!(
        CyclomaticLevel::from_complexity(21),
        CyclomaticLevel::HighlyComplex
    );
    assert_eq!(
        CyclomaticLevel::from_complexity(50),
        CyclomaticLevel::HighlyComplex
    );
    assert_eq!(
        CyclomaticLevel::from_complexity(51),
        CyclomaticLevel::Extreme
    );
}

#[test]
fn empty_input_returns_none() {
    let markers = rust_markers();
    assert!(analyze(&[], &[], markers).is_none());
}

#[test]
fn all_comments_returns_none() {
    let lines = vec!["// comment".to_string()];
    let kinds = vec![LineKind::Comment];
    assert!(analyze(&lines, &kinds, rust_markers()).is_none());
}

#[test]
fn operator_counting() {
    assert_eq!(count_operator("x && y && z", "&&"), 2);
    assert_eq!(count_operator("x || y", "||"), 1);
    assert_eq!(count_operator("no operators", "&&"), 0);
}

#[test]
fn keyword_word_boundary() {
    assert_eq!(count_keyword("if x > 0", "if"), 1);
    assert_eq!(count_keyword("notify()", "if"), 0);
    assert_eq!(count_keyword("elif x", "if"), 0);
    assert_eq!(count_keyword("if_something", "if"), 0);
}

#[test]
fn level_display() {
    assert_eq!(CyclomaticLevel::Simple.as_str(), "simple");
    assert_eq!(CyclomaticLevel::Moderate.as_str(), "moderate");
    assert_eq!(CyclomaticLevel::Complex.as_str(), "complex");
    assert_eq!(CyclomaticLevel::HighlyComplex.as_str(), "highly complex");
    assert_eq!(CyclomaticLevel::Extreme.as_str(), "extreme");
}

#[test]
fn level_serde() {
    assert_eq!(
        serde_json::to_string(&CyclomaticLevel::HighlyComplex).unwrap(),
        "\"highly_complex\""
    );
}

#[test]
fn keywords_in_strings_not_counted() {
    let (lines, kinds) = make_lines(
        "fn foo() {\n    let kw = [\"if\", \"for\", \"while\", \"match\"];\n    let s = \"if x && y || z\";\n}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions[0].complexity, 1);
}

#[test]
fn braces_in_char_literals_not_counted() {
    let (lines, kinds) = make_lines(
        "fn foo() {\n    if c == '{' {\n        bar();\n    }\n}\nfn bar() {\n    baz();\n}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions.len(), 2);
    assert_eq!(result.functions[0].name, "foo");
    assert_eq!(result.functions[0].complexity, 2);
    assert_eq!(result.functions[1].name, "bar");
    assert_eq!(result.functions[1].complexity, 1);
}

#[test]
fn aggregation_stats() {
    let (lines, kinds) =
        make_lines("fn foo() {\n    if x > 0 {\n        a();\n    }\n}\nfn bar() {\n    b();\n}\n");
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.total_complexity, 3);
    assert_eq!(result.max_complexity, 2);
    assert!((result.avg_complexity - 1.5).abs() < 0.01);
}
