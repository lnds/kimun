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
    assert!(!is_c_family_function("let x = 1;", true));
    assert!(!is_c_family_function("int x = 1;", true));
    assert!(!is_c_family_function("struct Foo", true));
}

#[test]
fn is_c_family_function_no_open_brace_or_closing_paren() {
    // Has '(' but doesn't end with '{' or ')'
    assert!(!is_c_family_function("foo(x, y,", true));
    assert!(!is_c_family_function("bar(x, y;", true));
}

#[test]
fn is_c_family_function_control_keyword_returns_false() {
    assert!(!is_c_family_function("if (condition) {", true));
    assert!(!is_c_family_function("while (x > 0) {", true));
    assert!(!is_c_family_function("for (int i = 0; i < n; i++) {", true));
}

#[test]
fn is_c_family_function_valid_function() {
    assert!(is_c_family_function("int foo(int x) {", true));
    assert!(is_c_family_function("void bar()", true));
    assert!(is_c_family_function(
        "static void baz(int a, int b) {",
        true
    ));
}

#[test]
fn is_c_family_function_preprocessor_directive_returns_false() {
    // Function-like #define looks like a C function but is not one.
    assert!(!is_c_family_function(
        "#define TRACE_ALLOC(name) trace_record(name)",
        true
    ));
    assert!(!is_c_family_function(
        "#define MAX(a, b) ((a) > (b) ? (a) : (b))",
        true
    ));
    assert!(!is_c_family_function("#if defined(FOO)", true));
    assert!(!is_c_family_function("#include <stdio.h>", true));
}

#[test]
fn is_c_family_function_matches_bash_posix_form() {
    // `foo() {` on its own line is a bash function definition (POSIX form).
    // The C-family heuristic must accept it so `Bourne Shell` files get
    // per-function decomposition — SHELL markers include `"function "`
    // for the `function foo { ... }` shape, but the more common POSIX
    // `foo() {` shape has no keyword prefix and relies on this fallback.
    assert!(is_c_family_function("foo() {", true));
    assert!(is_c_family_function("_private_helper() {", true));
}

#[test]
fn is_c_family_function_rejects_shell_command_substitution() {
    // `VAR=$(cmd | filter)` has `(`, ends with `)`, first word not a
    // control keyword — but the `=` before the `(` means it's an
    // assignment, not a function definition. Same for local/readonly
    // assignments with command substitution.
    assert!(!is_c_family_function(
        "SRC_FINGERPRINT=$(cd \"$SCRIPT_DIR\" && cat file | sha256sum | cut -c1-16)",
        true
    ));
    assert!(!is_c_family_function("OUT_SIZE=$(wc -c < \"$OUT\")", true));
    assert!(!is_c_family_function("local pid=$(pgrep vault)", true));
    assert!(!is_c_family_function("readonly FOO=$(date +%s)", true));
}

#[test]
fn is_c_family_function_rejects_c_family_assignment() {
    // `x = foo(bar)` — same category as bash assignments: an assignment,
    // not a function definition.
    assert!(!is_c_family_function("x = foo(bar)", true));
    assert!(!is_c_family_function("let x = compute(a, b)", true));
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

/// Name and first line of each function detected in `src`, written in
/// `language`, through the markers and code lines the analyzers use.
fn functions_in(language: &str, src: &str) -> Vec<(String, usize)> {
    let markers = crate::cycom::markers::markers_for(language).unwrap();
    let lines: Vec<String> = src.lines().map(String::from).collect();
    let kinds: Vec<LineKind> = lines
        .iter()
        .map(|l| match l.trim() {
            "" => LineKind::Blank,
            t if t.starts_with("//") => LineKind::Comment,
            _ => LineKind::Code,
        })
        .collect();
    let code = code_lines(&lines, &kinds, markers);
    detect_function_bodies(&lines, &code, markers)
        .into_iter()
        .map(|b| (b.name, b.start_line))
        .collect()
}

fn at(pairs: &[(&str, usize)]) -> Vec<(String, usize)> {
    named(pairs)
}

#[test]
fn go_method_is_named_after_the_receiver() {
    let src = "\
func (n *node) getValue(x int) int {
	return n.n
}
func (n node) size() int {
	return n.n
}";
    assert_eq!(functions_in("Go", src), at(&[("getValue", 1), ("size", 4)]));
}

#[test]
fn c_parameters_over_several_lines() {
    let src = "\
static int g(int a,
             int b) {
  return a;
}
int add(int x,
        int y)
{
    return x + y;
}";
    assert_eq!(functions_in("C", src), at(&[("g", 1), ("add", 5)]));
}

#[test]
fn c_name_on_its_own_line() {
    let src = "\
double
jvp_strtod
  (struct dtoa_context *C, const char *s00)
{
  return 1;
}";
    assert_eq!(functions_in("C", src), at(&[("jvp_strtod", 2)]));
}

#[test]
fn preprocessor_lines_and_their_continuations_are_not_code() {
    let src = "\
#define Bcopy(x,y) memcpy((char *)&x->sign, \\
y->wds*sizeof(Long) + 2*sizeof(int))
#if FMT_HAS_INCLUDE(<filesystem>) && \\
    (!defined(FMT_CPP_LIB_FILESYSTEM) || FMT_CPP_LIB_FILESYSTEM != 0)
#  include <filesystem>
#endif
static int first(int x) {
  return x;
}";
    assert_eq!(functions_in("C++", src), at(&[("first", 7)]));
    let lines: Vec<String> = src.lines().map(String::from).collect();
    let kinds = vec![LineKind::Code; lines.len()];
    let markers = crate::cycom::markers::markers_for("C").unwrap();
    assert_eq!(code_lines(&lines, &kinds, markers).len(), 3);
}

#[test]
fn csharp_continuation_lines_are_not_declarations() {
    let src = "\
class Reader
{
    private bool Calculate(
        object target,
        [NotNullWhen(true)] out Type? found)
    {
        if ((tokenType == 1)
            && IsReadable(target))
        {
            current = target;
        }
        return true;
    }
}";
    assert_eq!(functions_in("C#", src), at(&[("Calculate", 3)]));
}

#[test]
fn java_annotations_and_anonymous_classes_are_not_declarations() {
    let src = "\
@SuppressWarnings(\"ExposedPrivateType\")
public final class Adapters {
  @SuppressWarnings(\"fallthrough\")
  int doPeek(int c) {
    return c;
  }
  public static final TypeAdapter<Number> BYTE =
      new TypeAdapter<Number>() {
        @Override
        public Number read(JsonReader in) throws IOException {
          return in.nextInt();
        }
      };
}";
    assert_eq!(
        functions_in("Java", src),
        at(&[("doPeek", 4), ("read", 10)])
    );
}

#[test]
fn javascript_functions_assigned_to_a_property() {
    let src = "\
res.format = function(obj){
  var keys = Object.keys(obj)
    .filter(function (v) { return v !== 'default' })
  if (keys.length) {
    obj.send(keys);
  } else if (obj.default) {
    obj.default(keys);
  }
  return this;
};
res.cookie = function (name, value) {
  return value;
};
defineGetter(req, 'fresh', function(){
  return true;
});";
    assert_eq!(
        functions_in("JavaScript", src),
        at(&[("res.format", 1), ("res.cookie", 11), ("<anonymous>", 14)])
    );
}

#[test]
fn typescript_arrow_functions_and_multi_line_signatures() {
    let src = "\
export const equal = (a: ArrayBuffer, b: ArrayBuffer): boolean => {
  return a === b
}
const buildMatcher = (
  rules: string[]
): ((addr: { addr: string }) => boolean) => {
  return () => false
}
export const handle = <
  E extends Env = Env
>(
  app: Hono<E>
) => {
  return app
}
class Client {
  async fetch(
    body: string,
    opt?: { headers?: Record<string, string> }
  ): Promise<Response> {
    form.append('a', body)
    return send(body)
  }
  basePath<SubPath extends string>(path: SubPath): string {
    return path
  }
}
const handlers: Record<string, () => T> = {
  a: 1,
}";
    assert_eq!(
        functions_in("TypeScript", src),
        at(&[
            ("equal", 1),
            ("buildMatcher", 4),
            ("handle", 9),
            ("fetch", 17),
            ("basePath", 24),
        ])
    );
}

#[test]
fn regex_literal_braces_are_not_blocks() {
    let src = "\
function wildcard(path: string): RegExp {
  return new RegExp(path.replace(/(?=[/{]|$)/g, '.'))
}
function second(x: number): number {
  return x / 2 / 3
}";
    assert_eq!(
        functions_in("TypeScript", src),
        at(&[("wildcard", 1), ("second", 4)])
    );
}

#[test]
fn elixir_macros_and_functions_inside_quote() {
    let src = "\
defmodule Helpers do
  defmacro pipeline(name) do
    name
  end
  defmacrop build(x) do
    x
  end
  def define(routes) do
    quote do
      def path(conn) do
        conn
      end
    end
  end
end";
    let markers = crate::cycom::markers::markers_for("Elixir").unwrap();
    let lines: Vec<String> = src.lines().map(String::from).collect();
    let kinds = vec![LineKind::Code; lines.len()];
    let code = code_lines(&lines, &kinds, markers);
    let found: Vec<(String, usize)> = detect_function_bodies(&lines, &code, markers)
        .into_iter()
        .map(|b| (b.name, b.code_lines.len()))
        .collect();
    // `define` keeps its `quote do` and the `end` of the quote, not the
    // generated `path`, which is measured on its own.
    assert_eq!(
        found,
        named(&[("pipeline", 2), ("build", 2), ("define", 3), ("path", 2)])
    );
}

#[test]
fn marker_must_start_a_word() {
    let src = "\
callmyfunction(a, b)
function real() {
  return 1
}";
    assert_eq!(functions_in("JavaScript", src), at(&[("real", 2)]));
}
