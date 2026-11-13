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
fn code_outside_functions_that_branches_is_file() {
    let (lines, kinds) = make_lines(
        "import sys\n\ndef f(x):\n    return x\n\nif len(sys.argv) > 1 and sys.argv[1]:\n    for a in sys.argv:\n        print(f(a))\n",
    );
    let result = analyze(&lines, &kinds, python_markers()).unwrap();
    let found: Vec<_> = result
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.complexity))
        .collect();
    assert_eq!(found, vec![("<file>", 4), ("f", 1)]);
    assert_eq!(result.total_complexity, 5);
    assert_eq!(result.max_complexity, 4);
}

#[test]
fn code_outside_functions_without_branches_is_not_measured() {
    let (lines, kinds) = make_lines(
        "use std::io;\nconst N: u32 = 1;\nimpl Display for Level {\n    fn fmt(&self) {\n        if x {}\n    }\n}\nunsafe impl<T> Send for Box<T> {}\n",
    );
    let result = analyze(&lines, &kinds, rust_markers()).unwrap();
    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].name, "fmt");
}

#[test]
fn methods_of_a_kotlin_class_with_a_constructor_are_measured_apart() {
    let (lines, kinds) = make_lines(
        "class Repo(private val db: Db) {\n    fun find(id: Int): Row? {\n        if (id < 0) return null\n        return db.get(id)\n    }\n    fun all(limit: Int): List<Row> {\n        if (limit <= 0) return emptyList()\n        for (r in db.rows()) { if (r.ok) out.add(r) }\n        return out\n    }\n}\n",
    );
    let markers = super::super::markers::markers_for("Kotlin").unwrap();
    let result = analyze(&lines, &kinds, markers).unwrap();
    let found: Vec<_> = result
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.complexity))
        .collect();
    assert_eq!(found, vec![("find", 2), ("all", 4)]);
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

#[test]
fn elixir_branches_by_clause_keyword_and_operator() {
    let code = "\
defmodule Branches do
  def with_it(a, b) do
    with {:ok, x} <- a,
         {:ok, y} <- b do
      x + y
    end
  end
  def unless_it(x) do
    unless x do
      1
    end
  end
  def receive_it do
    receive do
      :a -> 1
      :b -> 2
    end
  end
  def case_it(x) do
    case x do
      1 -> :one
      2 -> :two
      _ -> :many
    end
  end
  def bool_it(a, b) do
    a and b or a && b
  end
  def map_it(xs) do
    Enum.map(xs, fn x -> x + 1 end)
  end
end";
    let markers = super::super::markers::markers_for("Elixir").unwrap();
    let (lines, kinds) = make_lines(code);
    let found: Vec<(String, usize)> = analyze(&lines, &kinds, markers)
        .unwrap()
        .functions
        .into_iter()
        .map(|f| (f.name, f.complexity))
        .collect();
    let expected: Vec<(String, usize)> = [
        ("with_it", 2),
        ("unless_it", 2),
        ("receive_it", 3),
        // `1` and `2` branch; the catch-all `_` does not, as `default`.
        ("case_it", 3),
        ("bool_it", 4),
        ("map_it", 1),
    ]
    .iter()
    .map(|(n, c)| (n.to_string(), *c))
    .collect();
    assert_eq!(found, expected);
}

#[test]
fn kaikai_default_of_an_option_is_a_branch() {
    let markers = super::super::markers::markers_for("Kaikai").unwrap();
    let code = "fn port(env: Map[String, Int]) : Int {\n  \
                let p = env |> map.get(\"PORT\") ?? 8080; p\n}\n";
    let (lines, kinds) = make_lines(code);
    let result = analyze(&lines, &kinds, markers).unwrap();
    assert_eq!(result.functions[0].complexity, 2);
}

#[test]
fn python_signature_closed_at_the_indentation_of_def_keeps_its_body() {
    let src = "\
def black_style(
    a: int,
    b: int,
) -> int:
    if a:
        return 1
    elif b:
        return 2
    while a and b:
        a -= 1
    return 0
def after():
    return 1
";
    let (lines, kinds) = make_lines(src);
    let result = analyze(&lines, &kinds, python_markers()).unwrap();
    let found: Vec<(&str, usize)> = result
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.complexity))
        .collect();
    assert_eq!(found, [("black_style", 5), ("after", 1)]);
}

#[test]
fn ruby_rescue_at_the_indentation_of_def_stays_in_the_method() {
    let src = "\
def with_rescue(x)
  risky(x) if x
rescue ArgumentError => e
  if e.message && x
    log(e)
  end
rescue StandardError
  nil
end
";
    let markers = super::super::markers::markers_for("Ruby").unwrap();
    let (lines, kinds) = make_lines(src);
    let result = analyze(&lines, &kinds, markers).unwrap();
    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].complexity, 6);
}

#[test]
fn elixir_rescue_at_the_indentation_of_def_stays_in_the_function() {
    let src = "\
defmodule R do
  def with_rescue(x) do
    if x > 0 and x < 5 do
      risky(x)
    end
  rescue
    e in ArgumentError ->
      if x, do: log(e)
    _ -> nil
  end

  def next_one(x), do: x
end
";
    let markers = super::super::markers::markers_for("Elixir").unwrap();
    let (lines, kinds) = make_lines(src);
    let result = analyze(&lines, &kinds, markers).unwrap();
    let found: Vec<(&str, usize)> = result
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.complexity))
        .collect();
    // if, and, <, the `e in ArgumentError` clause and the `if` in it; the
    // catch-all `_` clause is no branch.
    assert_eq!(found, [("with_rescue", 6), ("next_one", 1)]);
}

fn complexity_of(language: &str, code: &str) -> Vec<usize> {
    let markers = super::super::markers::markers_for(language).unwrap();
    let (lines, kinds) = make_lines(code);
    analyze(&lines, &kinds, markers)
        .unwrap()
        .functions
        .iter()
        .map(|f| f.complexity)
        .collect()
}

#[test]
fn a_switch_counts_its_cases_and_not_itself() {
    let go = "func f(x int) int {\n\tswitch x {\n\tcase 1:\n\t\treturn 1\n\tcase 2:\n\t\treturn 2\n\tdefault:\n\t\treturn 0\n\t}\n}\n";
    assert_eq!(complexity_of("Go", go), [3]);
}

#[test]
fn a_do_while_loop_counts_once() {
    let c = "int f(int x) {\n    do {\n        x--;\n    } while (x > 0);\n    return x;\n}\n";
    assert_eq!(complexity_of("C", c), [2]);
}

#[test]
fn foreach_and_elseif_are_branches() {
    let cs = "void F(int[] xs) {\n    foreach (var x in xs) {\n        if (x > 0) { }\n    }\n}\n";
    assert_eq!(complexity_of("C#", cs), [3]);
    let php = "function f($x) {\n    if ($x) {\n    } elseif ($x > 1) {\n    }\n}\n";
    assert_eq!(complexity_of("PHP", php), [3]);
}

#[test]
fn a_ternary_is_a_branch() {
    let js = "function f(a) {\n  return a > 0 ? 1 : 2;\n}\n";
    assert_eq!(complexity_of("JavaScript", js), [2]);
    let java = "void f(List<? extends T> a, Map<K, ? super V> b) {\n    g(a);\n}\n";
    assert_eq!(complexity_of("Java", java), [1]);
}

#[test]
fn null_coalescing_is_a_branch_in_csharp() {
    let cs = "string F(string a) {\n    return a ?? \"x\";\n}\n";
    assert_eq!(complexity_of("C#", cs), [2]);
}

#[test]
fn each_arm_of_a_rust_match_but_the_catch_all_is_a_branch() {
    let rust = "fn f(x: u8) -> u8 {\n    match x {\n        0 => 1,\n        1 | 2 => 2,\n        n if n > 9 => 3,\n        _ => 4,\n    }\n}\n";
    // three arms and a guard
    assert_eq!(complexity_of("Rust", rust), [5]);
}

#[test]
fn a_keyword_naming_a_key_a_symbol_or_a_method_is_no_branch() {
    let ruby = "def f(opts)\n  opts[:if] || opts[:unless]\nend\n";
    assert_eq!(complexity_of("Ruby", ruby), [2]);
    let elixir = "def f(q) do\n  validate(q, if: true, for: :x, with: MyMod)\nend\n";
    assert_eq!(complexity_of("Elixir", elixir), [1]);
    let js = "function f(p) {\n  return p.then(g).catch(h);\n}\n";
    assert_eq!(complexity_of("JavaScript", js), [1]);
}
