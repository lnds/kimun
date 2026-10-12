use super::*;

struct Markers(&'static str, FunctionSyntax);

impl FunctionDetectionMarkers for Markers {
    fn function_markers(&self) -> &[&str] {
        std::slice::from_ref(&self.0)
    }
    fn brace_scoped(&self) -> bool {
        true
    }
    fn function_syntax(&self) -> FunctionSyntax {
        self.1
    }
    fn line_comments(&self) -> &[&str] {
        &["#"]
    }
}

/// Name and number of lines of each function detected in `src`.
fn bodies(marker: &'static str, src: &str) -> Vec<(String, usize)> {
    bodies_as(marker, FunctionSyntax::Loose, src)
}

/// The same, for a language that writes its functions as `syntax` says.
fn bodies_as(marker: &'static str, syntax: FunctionSyntax, src: &str) -> Vec<(String, usize)> {
    let lines: Vec<String> = src.lines().map(String::from).collect();
    let code_lines: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i, l.as_str()))
        .collect();
    detect_function_bodies(&lines, &code_lines, &Markers(marker, syntax))
        .into_iter()
        .map(|b| (b.name, b.code_lines.len()))
        .collect()
}

fn named(pairs: &[(&str, usize)]) -> Vec<(String, usize)> {
    pairs.iter().map(|(n, l)| (n.to_string(), *l)).collect()
}

#[test]
fn expression_body_without_braces_ends_at_its_indentation() {
    let src = "\
fn first(x: Int) : Int =
  second(x)

fn second(x: Int) : Int {
  if x > 0 { 1 } else { 0 }
}
";
    assert_eq!(bodies("fn ", src), named(&[("first", 2), ("second", 3)]));
}

#[test]
fn expression_body_runs_past_the_first_balanced_line() {
    let src = "\
fn outer(e: Int, d: Int) : Int =
  if e > 0 { e }
  else {
if d > 0 { e } else {
  render(d)
  mark(e)
}
  }

fn next(x: Int) : Int = if x > 0 { 1 } else { 0 }
";
    assert_eq!(bodies("fn ", src), named(&[("outer", 8), ("next", 1)]));
}

#[test]
fn expression_body_closing_brace_at_declaration_indent() {
    let src = "\
fun kind(x: Int) = when (x) {
0 -> 1
else -> 2
}
fun other(x: Int) = x + 1
";
    assert_eq!(bodies("fun ", src), named(&[("kind", 4), ("other", 1)]));
}

#[test]
fn expression_body_after_a_signature_on_several_lines() {
    let src = "\
fn long(
  a: Int = 1,
  b: Int,
) : Int =
  a + b

fn after() : Int = 1
";
    assert_eq!(bodies("fn ", src), named(&[("long", 5), ("after", 1)]));
}

#[test]
fn default_argument_is_not_an_expression_body() {
    let src = "\
fn f(a: Int = 1)
{
  a
}
fn g() { 1 }
";
    assert_eq!(bodies("fn ", src), named(&[("f", 4), ("g", 1)]));
}

#[test]
fn assign_in_where_clause_keeps_the_block_on_the_next_line() {
    let src = "\
fn sum<I>(items: I) -> u32
where
I: Iterator<Item = u32>,
{
items.sum()
}
fn g() { 1 }
";
    assert_eq!(bodies("fn ", src), named(&[("sum", 6), ("g", 1)]));
}

#[test]
fn assign_inside_a_signature_is_not_an_expression_body() {
    // The lifetime hides the opening parenthesis from the masking.
    let src = "\
fn compute<'a>(
graph: &'a Graph,
files: impl Iterator<Item = &'a Path>,
) -> Radius {
let seeds = files
    .count();
seeds
}
fn g() { 1 }
";
    assert_eq!(bodies("fn ", src), named(&[("compute", 8), ("g", 1)]));
}

#[test]
fn comparison_and_arrow_are_not_assignments() {
    assert!(is_plain_assign(b"a = b", 2));
    for (text, pos) in [("a == b", 2), ("a == b", 3), ("a => b", 2), ("a <= b", 3)] {
        assert!(!is_plain_assign(text.as_bytes(), pos), "{text} at {pos}");
    }
    assert!(!is_plain_assign(b"a += b", 3));
    assert!(!is_plain_assign(b"a != b", 3));
}

#[test]
fn is_c_family_function_no_paren_returns_false() {
    assert!(!is_c_family_function("let x = 1;"));
    assert!(!is_c_family_function("int x = 1;"));
    assert!(!is_c_family_function("struct Foo"));
}

#[test]
fn is_c_family_function_no_open_brace_or_closing_paren() {
    // Has '(' but doesn't end with '{' or ')'
    assert!(!is_c_family_function("foo(x, y,"));
    assert!(!is_c_family_function("bar(x, y;"));
}

#[test]
fn is_c_family_function_control_keyword_returns_false() {
    assert!(!is_c_family_function("if (condition) {"));
    assert!(!is_c_family_function("while (x > 0) {"));
    assert!(!is_c_family_function("for (int i = 0; i < n; i++) {"));
}

#[test]
fn is_c_family_function_valid_function() {
    assert!(is_c_family_function("int foo(int x) {"));
    assert!(is_c_family_function("void bar()"));
    assert!(is_c_family_function("static void baz(int a, int b) {"));
}

#[test]
fn is_c_family_function_preprocessor_directive_returns_false() {
    // Function-like #define looks like a C function but is not one.
    assert!(!is_c_family_function(
        "#define TRACE_ALLOC(name) trace_record(name)"
    ));
    assert!(!is_c_family_function(
        "#define MAX(a, b) ((a) > (b) ? (a) : (b))"
    ));
    assert!(!is_c_family_function("#if defined(FOO)"));
    assert!(!is_c_family_function("#include <stdio.h>"));
}

#[test]
fn is_c_family_function_matches_bash_posix_form() {
    // `foo() {` on its own line is a bash function definition (POSIX form).
    // The C-family heuristic must accept it so `Bourne Shell` files get
    // per-function decomposition — SHELL markers include `"function "`
    // for the `function foo { ... }` shape, but the more common POSIX
    // `foo() {` shape has no keyword prefix and relies on this fallback.
    assert!(is_c_family_function("foo() {"));
    assert!(is_c_family_function("_private_helper() {"));
}

#[test]
fn is_c_family_function_rejects_shell_command_substitution() {
    // `VAR=$(cmd | filter)` has `(`, ends with `)`, first word not a
    // control keyword — but the `=` before the `(` means it's an
    // assignment, not a function definition. Same for local/readonly
    // assignments with command substitution.
    assert!(!is_c_family_function(
        "SRC_FINGERPRINT=$(cd \"$SCRIPT_DIR\" && cat file | sha256sum | cut -c1-16)"
    ));
    assert!(!is_c_family_function("OUT_SIZE=$(wc -c < \"$OUT\")"));
    assert!(!is_c_family_function("local pid=$(pgrep vault)"));
    assert!(!is_c_family_function("readonly FOO=$(date +%s)"));
}

#[test]
fn is_c_family_function_rejects_c_family_assignment() {
    // `x = foo(bar)` — same category as bash assignments: an assignment,
    // not a function definition.
    assert!(!is_c_family_function("x = foo(bar)"));
    assert!(!is_c_family_function("let x = compute(a, b)"));
}

#[test]
fn item_with_a_parenthesis_is_no_function_where_a_keyword_declares_them() {
    let src = "\
pub(super) enum Kind {
    A,
    B,
}

pub fn first() -> u32 {
    1
}
";
    assert_eq!(
        bodies_as("fn ", FunctionSyntax::Keyword, src),
        named(&[("first", 3)])
    );
    assert_eq!(bodies("fn ", src), named(&[("pub", 4), ("first", 3)]));
}

#[test]
fn declaration_without_a_body_is_no_function() {
    let src = "\
pub trait Markers {
    fn function_markers(&self) -> &[&str];
    fn spread(
        &self,
        depth: usize,
    ) -> bool;
}

pub struct Body {
    pub name: String,
}

fn is_declaration(trimmed: &str) -> bool {
    trimmed.contains('(')
}
";
    assert_eq!(
        bodies_as("fn ", FunctionSyntax::Keyword, src),
        named(&[("is_declaration", 3)])
    );
}

#[test]
fn lifetime_does_not_hide_the_brace_that_opens_the_body() {
    let src = "\
fn pick<'a>(
    lines: &[(usize, &'a str)],
) -> (Vec<(usize, &'a str)>, usize) {
    let mut out = Vec::new();
    for (i, line) in lines {
        if line.is_empty() {
            continue;
        }
        out.push((*i, *line));
    }
    let n = out
        .iter()
        .count();
    (out, n)
}

fn after() -> u32 {
    1
}
";
    assert_eq!(
        bodies_as("fn ", FunctionSyntax::KeywordWithLifetimes, src),
        named(&[("pick", 15), ("after", 3)])
    );
}

#[test]
fn lifetimes_lose_their_quote_and_character_literals_keep_theirs() {
    assert_eq!(
        without_lifetimes("fn f<'a>(x: &'a str) {"),
        "fn f< a>(x: & a str) {"
    );
    assert_eq!(without_lifetimes("x: &'static str,"), "x: & static str,");
    assert_eq!(without_lifetimes("'outer: loop {"), " outer: loop {");
    for literal in [
        "'{'",
        "'a'",
        "'\\''",
        "'\\n'",
        "'\\u{10FFFF}'",
        "'ñ'",
        "b'x'",
    ] {
        let line = format!("if c == {literal} {{");
        assert_eq!(without_lifetimes(&line), line);
    }
    assert_eq!(
        without_lifetimes("m('(', &'a x, ')')"),
        "m('(', & a x, ')')"
    );
}

#[test]
fn a_line_may_introduce_the_body_by_closing_the_parameters_or_ending_in_assign() {
    assert!(may_introduce_body(") : Int = a + 1"));
    assert!(may_introduce_body("  : Int ="));
    assert!(!may_introduce_body("files: impl Iterator<Item = u32>,"));
}

#[test]
fn an_assign_inside_the_parameter_list_introduces_no_body() {
    let mut closed = BodyScan::default();
    closed.read("fn f(a: Int = 1)", true);
    assert_eq!(closed.paren_depth, 0);
    assert!(!closed.expression_body);

    let mut open = BodyScan::default();
    open.read("fn f(a: Int = 1,", true);
    assert_eq!(open.paren_depth, 1);
    assert!(!open.expression_body);

    let mut after = BodyScan::default();
    after.read("fn f(a: Int) : Int = a", true);
    assert!(after.expression_body);
}

#[test]
fn a_lifetime_right_after_a_character_literal_loses_its_quote() {
    assert_eq!(without_lifetimes("('x''a y)"), "('x' a y)");
}

#[test]
fn a_character_literal_is_as_long_as_its_closing_quote_says() {
    assert_eq!(literal_len(b"a'"), 2);
    assert_eq!(literal_len(b"\\''"), 3);
    assert_eq!(literal_len(b"\\n' rest"), 3);
    // The longest ones: four bytes of a character, eleven of an escape.
    assert_eq!(literal_len("\u{1F600}'".as_bytes()), 5);
    assert_eq!(literal_len(b"\\u{10FFFFF}'"), 12);
}

#[test]
fn a_quote_that_closes_too_far_is_no_character_literal() {
    assert_eq!(literal_len(b"abcdef"), 0);
    assert_eq!(literal_len(b"abcde'"), 0);
    assert_eq!(literal_len(b"\\u{10FFFFFF}'"), 0);
    assert_eq!(literal_len(b""), 0);
}

#[test]
fn a_function_right_after_a_line_that_declares_none_is_found() {
    let src = "\
const LIMIT: u32 = 1;
fn first() -> u32 { 1 }
";
    assert_eq!(
        bodies_as("fn ", FunctionSyntax::Keyword, src),
        named(&[("first", 1)])
    );
}

struct Indented;

impl FunctionDetectionMarkers for Indented {
    fn function_markers(&self) -> &[&str] {
        &["def "]
    }
    fn brace_scoped(&self) -> bool {
        false
    }
    fn function_syntax(&self) -> FunctionSyntax {
        FunctionSyntax::Loose
    }
    fn line_comments(&self) -> &[&str] {
        &["#"]
    }
}

#[test]
fn an_indented_body_holds_every_line_deeper_than_its_declaration() {
    let lines: Vec<String> = "\
def first():
    x = 1
    return x
def second():
    return 2
"
    .lines()
    .map(String::from)
    .collect();
    let code_lines: Vec<(usize, &str)> = lines.iter().map(String::as_str).enumerate().collect();
    let found: Vec<(String, usize)> = detect_function_bodies(&lines, &code_lines, &Indented)
        .into_iter()
        .map(|b| (b.name, b.code_lines.len()))
        .collect();
    assert_eq!(found, named(&[("first", 3), ("second", 2)]));
}
