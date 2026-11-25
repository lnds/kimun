use super::*;

const RUST: FunctionSyntax = FunctionSyntax::KeywordWithLifetimes;

#[test]
fn names_literals_and_closing_brackets_end_an_operand() {
    for text in ["a", "x.y()", "v[0]", "f()?", "\"s\"", "'c'", "1"] {
        assert!(ends_with_operand(text.as_bytes()), "{text}");
    }
    for text in ["", "(", "f(a,", "x =", "move", "return", "=>"] {
        assert!(!ends_with_operand(text.as_bytes()), "{text}");
    }
}

#[test]
fn a_bar_pair_heads_a_closure_where_an_operand_would_start() {
    assert_eq!(count(RUST, "f(|| 0)", false), 1);
    assert_eq!(count(RUST, "let g = move || a || b;", false), 1);
    assert_eq!(count(RUST, "|| b", true), 0);
    assert_eq!(count(RUST, "|| b,", false), 1);
    assert_eq!(count(RUST, "a || b", false), 0);
}

#[test]
fn bars_head_no_closure_elsewhere() {
    assert_eq!(count(FunctionSyntax::Loose, "f(|| 0)", false), 0);
}
