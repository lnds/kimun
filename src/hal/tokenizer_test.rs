use super::*;

#[test]
fn rules_for_known_languages() {
    assert!(rules_for("Rust").is_some());
    assert!(rules_for("Python").is_some());
    assert!(rules_for("JavaScript").is_some());
    assert!(rules_for("Go").is_some());
    assert!(rules_for("C").is_some());
    assert!(rules_for("Ruby").is_some());
}

#[test]
fn rules_for_unknown_language() {
    assert!(rules_for("COBOL").is_none());
    assert!(rules_for("JSON").is_none());
}

#[test]
fn rust_simple_function() {
    let rules = rules_for("Rust").unwrap();
    let lines = vec!["fn foo() {", "    let x = 1;", "    let y = x + 2;", "}"];
    let counts = count_tokens(&lines, rules, &[]);

    // Operators: (, ), {, =, ;, =, +, ;, }
    assert!(counts.total_operators > 0);
    // Operands: foo, x, 1, y, x, 2
    assert!(counts.total_operands > 0);
    // fn and let are declarations → ignored, not operators
    assert!(!counts.distinct_operators.contains("fn"));
    assert!(!counts.distinct_operators.contains("let"));
    // foo is an operand (function name), not an operator
    assert!(counts.distinct_operands.contains("foo"));
    assert!(counts.distinct_operands.contains("x"));
    assert!(counts.distinct_operands.contains("1"));
}

#[test]
fn keywords_in_strings_not_counted() {
    let rules = rules_for("Rust").unwrap();
    let lines = vec!["let s = \"if for while match\";"];
    let counts = count_tokens(&lines, rules, &[]);

    // "if", "for", "while", "match" are inside a string — not operators
    assert!(!counts.distinct_operators.contains("if"));
    assert!(!counts.distinct_operators.contains("for"));
    assert!(!counts.distinct_operators.contains("while"));
    assert!(!counts.distinct_operators.contains("match"));
}

#[test]
fn function_names_are_operands() {
    let rules = rules_for("Rust").unwrap();
    let lines = vec!["foo(x, y);"];
    let counts = count_tokens(&lines, rules, &[]);

    // foo is an operand; () , ; are operators
    assert!(counts.distinct_operands.contains("foo"));
    assert!(counts.distinct_operands.contains("x"));
    assert!(counts.distinct_operands.contains("y"));
    assert!(counts.distinct_operators.contains("("));
    assert!(counts.distinct_operators.contains(","));
    assert!(counts.distinct_operators.contains(";"));
}

#[test]
fn numeric_literals_are_operands() {
    let rules = rules_for("Rust").unwrap();
    let lines = vec!["let x = 42;", "let y = 0xff;"];
    let counts = count_tokens(&lines, rules, &[]);

    assert!(counts.distinct_operands.contains("42"));
    assert!(counts.distinct_operands.contains("0xff"));
}

#[test]
fn multi_char_symbols() {
    let rules = rules_for("Rust").unwrap();
    let lines = vec!["if x && y || z == w {"];
    let counts = count_tokens(&lines, rules, &[]);

    assert!(counts.distinct_operators.contains("&&"));
    assert!(counts.distinct_operators.contains("||"));
    assert!(counts.distinct_operators.contains("=="));
}

#[test]
fn python_tokens() {
    let rules = rules_for("Python").unwrap();
    let lines = vec!["def foo(x):", "    if x > 0:", "        return x + 1"];
    let counts = count_tokens(&lines, rules, &[]);

    // def is a declaration → ignored
    assert!(!counts.distinct_operators.contains("def"));
    // if, return are control flow → operators
    assert!(counts.distinct_operators.contains("if"));
    assert!(counts.distinct_operators.contains("return"));
    // foo is a function name → operand
    assert!(counts.distinct_operands.contains("foo"));
    assert!(counts.distinct_operands.contains("x"));
    assert!(counts.distinct_operands.contains("0"));
    assert!(counts.distinct_operands.contains("1"));
}

#[test]
fn empty_input() {
    let rules = rules_for("Rust").unwrap();
    let lines: Vec<&str> = vec![];
    let counts = count_tokens(&lines, rules, &[]);

    assert_eq!(counts.total_operators, 0);
    assert_eq!(counts.total_operands, 0);
    assert!(counts.distinct_operators.is_empty());
    assert!(counts.distinct_operands.is_empty());
}

#[test]
fn operator_symbols_sorted_longest_first() {
    // Validates that all operator_symbols arrays are sorted by length descending.
    // If a shorter symbol appears before a longer one with the same prefix,
    // try_match_symbol will match the short one first and misparse.
    let languages = [
        "Rust",
        "Python",
        "JavaScript",
        "Go",
        "C",
        "Ruby",
        "Kotlin",
        "Swift",
        "Bourne Shell",
    ];
    for lang in &languages {
        let rules = rules_for(lang).unwrap();
        for (i, sym) in rules.operator_symbols.iter().enumerate() {
            for later in &rules.operator_symbols[i + 1..] {
                if later.starts_with(sym) && later.len() > sym.len() {
                    panic!(
                        "{lang}: \"{sym}\" appears before \"{later}\" — \
                         longer symbols must come first for correct longest-match"
                    );
                }
            }
        }
    }
}

#[test]
fn longest_match_for_symbols() {
    let rules = rules_for("Rust").unwrap();
    let lines = vec!["x >>= y;"];
    let counts = count_tokens(&lines, rules, &[]);

    // Should match ">>=" as a single operator, not ">>" + "="
    assert!(counts.distinct_operators.contains(">>="));
    // Two operators: >>= and ;
    assert_eq!(counts.total_operators, 2);
}

#[test]
fn ignored_keywords_not_counted() {
    let rules = rules_for("Rust").unwrap();
    let lines = vec!["pub fn foo(x: i32) -> bool {"];
    let counts = count_tokens(&lines, rules, &[]);

    // pub, fn, i32, bool are all ignored (declarations, modifiers, types)
    assert!(!counts.distinct_operators.contains("pub"));
    assert!(!counts.distinct_operators.contains("fn"));
    assert!(!counts.distinct_operands.contains("i32"));
    assert!(!counts.distinct_operands.contains("bool"));
    // foo, x are operands
    assert!(counts.distinct_operands.contains("foo"));
    assert!(counts.distinct_operands.contains("x"));
}
