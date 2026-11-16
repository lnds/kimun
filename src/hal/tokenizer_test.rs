use super::*;

#[test]
fn rules_for_known_languages() {
    assert!(rules_for("Rust").is_some());
    assert!(rules_for("Python").is_some());
    assert!(rules_for("JavaScript").is_some());
    assert!(rules_for("JSX").is_some());
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
        "Kaikai",
        "Elixir",
        "Fortran 90",
        "Pascal",
        "Lua",
        "Perl",
        "Scala",
        "Zig",
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

#[test]
fn kaikai_rules_present() {
    assert!(rules_for("Kaikai").is_some());
}

#[test]
fn kaikai_pipeline_and_keywords() {
    let rules = rules_for("Kaikai").unwrap();
    let lines = vec![
        "fn f(x: Int) : Int = x |> g |> h",
        "let y = if x and z { 1 } else { 2 }",
    ];
    let counts = count_tokens(&lines, rules, &[]);
    assert!(counts.total_operators > 0);
    assert!(counts.total_operands > 0);
    // `fn`/`let` are declarations → ignored
    assert!(!counts.distinct_operators.contains("fn"));
    assert!(!counts.distinct_operators.contains("let"));
    // control-flow and boolean keywords are operators
    assert!(counts.distinct_operators.contains("if"));
    assert!(counts.distinct_operators.contains("and"));
    // pipeline symbol is an operator
    assert!(counts.distinct_operators.contains("|>"));
}

#[test]
fn elixir_rules_cover_both_extensions() {
    assert!(rules_for("Elixir").is_some());
    assert!(rules_for("Elixir Script").is_some());
}

#[test]
fn elixir_operators_and_declarations() {
    let rules = rules_for("Elixir").unwrap();
    let lines = vec![
        "defp total(items, acc \\\\ 0) when is_list(items) do",
        "  case items |> Enum.map(&price/1) do",
        "    [] -> acc",
        "    prices -> acc + Enum.sum(prices) <> \"\"",
        "  end",
        "end",
    ];
    let counts = count_tokens(&lines, rules, &["#"]);
    for op in ["case", "do", "end", "when", "|>", "->", "<>", "\\\\", "&"] {
        assert!(
            counts.distinct_operators.contains(op),
            "{op} is an operator"
        );
    }
    assert!(!counts.distinct_operators.contains("defp"));
    assert!(!counts.distinct_operands.contains("defp"));
    assert!(counts.distinct_operands.contains("total"));
    assert!(counts.distinct_operands.contains("acc"));
}

#[test]
fn elixir_attribute_names_are_ignored() {
    let rules = rules_for("Elixir").unwrap();
    let counts = count_tokens(&["@spec run(t) :: :ok", "@impl true"], rules, &["#"]);
    assert!(!counts.distinct_operands.contains("spec"));
    assert!(!counts.distinct_operands.contains("impl"));
    assert!(counts.distinct_operators.contains("::"));
    assert!(counts.distinct_operators.contains("@"));
}

#[test]
fn kaikai_coalesce_each_pipe_and_reference_are_operators() {
    let rules = rules_for("Kaikai").unwrap();
    let lines = vec!["let r = &cell", "xs |! log ?? 0"];
    let counts = count_tokens(&lines, rules, &[]);
    for op in ["&", "|!", "??"] {
        assert!(counts.distinct_operators.contains(op), "{op}");
    }
    assert!(!counts.distinct_operators.contains("|"));
    assert!(!counts.distinct_operators.contains("?"));
}

/// Each line of `code` read with the rules of `lang`: the operators and
/// operands it must find, and the declarations it must skip.
fn assert_tokens(lang: &str, code: &[&str], comments: &[&str], ops: &[&str], skipped: &[&str]) {
    let rules = rules_for(lang).unwrap();
    let counts = count_tokens(code, rules, comments);
    for op in ops {
        assert!(counts.distinct_operators.contains(*op), "{lang}: {op}");
    }
    for word in skipped {
        assert!(!counts.distinct_operators.contains(*word), "{lang}: {word}");
        assert!(!counts.distinct_operands.contains(*word), "{lang}: {word}");
    }
    assert!(counts.total_operands > 0, "{lang} has operands");
}

#[test]
fn lua_blocks_concatenation_and_inequality() {
    let code = [
        "local function add(a, b)",
        "  if a ~= b then return a .. b // 2 end",
        "  return #a",
        "end",
    ];
    let ops = ["if", "then", "end", "return", "~=", "..", "//", "#"];
    assert_tokens("Lua", &code, &["--"], &ops, &["local", "function"]);
}

#[test]
fn perl_sigils_and_word_operators() {
    let code = [
        "sub total {",
        "  my ($x, @rest) = @_;",
        "  return $x <=> $y if $s eq 'a' and $x =~ m;",
        "}",
    ];
    let ops = ["$", "@", "<=>", "eq", "and", "=~", "if", "return"];
    assert_tokens("Perl", &code, &["#"], &ops, &["sub", "my"]);
    let counts = count_tokens(&["my $x = 1;"], rules_for("Perl").unwrap(), &["#"]);
    assert!(counts.distinct_operands.contains("x"), "x is a variable");
}

#[test]
fn scala_match_for_and_mixins() {
    let code = [
        "class A extends B with C {",
        "  def f(xs: List[Int]): Int = xs match {",
        "    case h :: t => for (x <- t) yield x + h",
        "  }",
        "}",
    ];
    let ops = ["match", "case", "::", "=>", "<-", "for", "yield"];
    assert_tokens(
        "Scala",
        &code,
        &["//"],
        &ops,
        &["def", "class", "with", "Int"],
    );
}

#[test]
fn zig_error_handling_and_wrapping_arithmetic() {
    let code = [
        "pub fn next(self: *Iter) !u32 {",
        "    defer self.i +%= 1;",
        "    const v = try self.read() orelse unreachable;",
        "    return v.* ++ @as(u32, 1);",
        "}",
    ];
    let ops = [
        "defer",
        "+%=",
        "try",
        "orelse",
        "unreachable",
        ".*",
        "++",
        "@",
    ];
    assert_tokens("Zig", &code, &["//"], &ops, &["pub", "fn", "const", "u32"]);
}
