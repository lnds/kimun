//! Shared function detection for complexity analyzers.
//!
//! Both cyclomatic and cognitive complexity analyzers need to detect function
//! boundaries (brace-scoped or indent-scoped). This module provides the shared
//! detection logic via the `FunctionDetectionMarkers` trait, avoiding ~200 lines
//! of duplication between `cycom/detection.rs` and `cogcom/detection.rs`.

use crate::util::{indent_level, mask_strings};

/// Trait for language marker structs that support function detection.
/// Both `ComplexityMarkers` and `CognitiveMarkers` implement this.
pub trait FunctionDetectionMarkers {
    fn function_markers(&self) -> &[&str];
    fn brace_scoped(&self) -> bool;
    fn line_comments(&self) -> &[&str];
}

/// A detected function body: name, start line, and the code lines within it.
pub struct FunctionBody<'a> {
    pub name: String,
    pub start_line: usize,
    pub code_lines: Vec<(usize, &'a str)>,
}

/// Control-flow keywords that should NOT be treated as function definitions
/// in the C-family heuristic. If the first word of a line matches one of
/// these, it is treated as a control statement rather than a function.
const CONTROL_KEYWORDS: &[&str] = &[
    "if", "for", "while", "switch", "else", "do", "catch", "return", "case",
];

/// Check whether a line is a function declaration, using explicit markers
/// AND the C-family heuristic as fallback.
///
/// If any explicit marker matches, the line is a function declaration. If
/// not, the C-family heuristic is tried as a fallback (has `(`, ends with
/// `{` or `)`, first word not a control keyword, no `=` before the `(`).
/// This covers languages that have BOTH a keyword-prefixed form and a
/// paren-suffixed form — for example, Bash allows both
/// `function foo { ... }` (matched by the `"function "` marker) and
/// `foo() { ... }` (matched by the C-family heuristic).
///
/// The fallback is safe for marker-only languages: it only fires on lines
/// ending with `{` or `)`. Python `def foo(x):` ends with `:` and doesn't
/// reach it; Rust `fn foo() {` and JavaScript `function foo() {` already
/// match via their markers.
fn is_function_declaration(trimmed: &str, markers: &dyn FunctionDetectionMarkers) -> bool {
    if markers
        .function_markers()
        .iter()
        .any(|m| trimmed.contains(m))
    {
        return true;
    }
    is_c_family_function(trimmed)
}

/// Heuristic for C/C++/Java/C#/Bash function detection: line contains '('
/// and ends with '{' or ')', and the first word is NOT a control keyword,
/// and there is no '=' before the first '(' (which would make it an
/// assignment such as `x = foo(...)` or a shell command substitution
/// `VAR=$(cmd ...)` rather than a function definition).
///
/// Known limitations:
/// - Multiline declarations where '{' is on a separate line are missed.
/// - Function pointers (e.g., `void (*fp)(int)`) may be misdetected.
/// - C++ constructor initializer lists are not handled.
fn is_c_family_function(trimmed: &str) -> bool {
    // Preprocessor directives (`#define`, `#if`, `#include`, ...) are not
    // functions. A function-like `#define NAME(args) body` otherwise looks
    // like a C function (`identifier (params) body`) and would be misdetected,
    // absorbing the cognitive complexity of the code that follows it.
    if trimmed.starts_with('#') {
        return false;
    }
    let paren_pos = match trimmed.find('(') {
        Some(p) => p,
        None => return false,
    };
    if !(trimmed.ends_with('{') || trimmed.ends_with(')')) {
        return false;
    }

    // Assignment before the first '(' means this is a variable assignment
    // or a shell command substitution, not a function definition. Examples:
    //   x = foo(bar)             (C-family)
    //   VAR=$(cmd | filter)      (Bash command substitution)
    //   local FOO=$(...)         (Bash local assignment)
    // The `=` is checked in the prefix only; a `=` inside the parens
    // (default-argument, comparison) is unrelated.
    if trimmed[..paren_pos].contains('=') {
        return false;
    }

    let first_word = trimmed.split_whitespace().next().unwrap_or("");
    let first_word = first_word.trim_start_matches('*');
    !CONTROL_KEYWORDS.contains(&first_word)
}

/// Extract the function name from a declaration line using language-specific
/// markers or the C-family heuristic (token before the first `(`).
pub fn extract_function_name(trimmed: &str, markers: &dyn FunctionDetectionMarkers) -> String {
    for marker in markers.function_markers() {
        if let Some(pos) = trimmed.find(marker) {
            let after = &trimmed[pos + marker.len()..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return name;
            }
        }
    }

    // C-family heuristic: name is the token before '('
    if let Some(paren_pos) = trimmed.find('(') {
        let before = trimmed[..paren_pos].trim();
        if let Some(name) = before.split_whitespace().next_back() {
            let name = name.trim_start_matches('*');
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }

    "<anonymous>".to_string()
}

/// Whether the `=` at `pos` stands alone, rather than being part of an
/// operator such as `==`, `=>`, `<=` or `+=`.
fn is_plain_assign(bytes: &[u8], pos: usize) -> bool {
    let before = pos.checked_sub(1).map(|p| bytes[p]);
    let after = bytes.get(pos + 1).copied();
    !before.is_some_and(|b| b"=!<>+-*/%&|^:~".contains(&b))
        && !after.is_some_and(|b| b"=>~".contains(&b))
}

/// Whether a line after the declaration can hold the `=` that introduces
/// the body: the one that closes the parameter list, or one the `=` ends.
/// Elsewhere in a signature a `=` binds something else, such as the
/// associated type of `impl Iterator<Item = T>`.
fn may_introduce_body(masked: &str) -> bool {
    masked.trim_start().starts_with(')') || masked.trim_end().ends_with('=')
}

/// Whether `next` still belongs to an expression body declared at
/// `base_indent`: it is indented deeper, or it opens the block of a
/// signature whose `=` was not the body (`where T: Iterator<Item = u32>`).
fn continues_expression(next: Option<&(usize, &str)>, base_indent: usize) -> bool {
    next.is_some_and(|(_, line)| {
        indent_level(line) > base_indent || line.trim_start().starts_with('{')
    })
}

/// Starting from `code_lines[start]`, collect all lines belonging to the
/// function body by tracking brace depth (string/char literals are masked).
/// Returns `(func_code_lines, end_index)` where `end_index` is the index
/// of the last line of the body in `code_lines`.
///
/// A body introduced by `=` instead of a brace (`fn f(x) = expr`) has no
/// closing delimiter of its own: it runs while its braces are open and
/// while the lines that follow are indented deeper than the declaration.
fn find_function_body<'a>(
    code_lines: &[(usize, &'a str)],
    start: usize,
    line_comments: &[&str],
) -> (Vec<(usize, &'a str)>, usize) {
    let mut brace_depth: isize = 0;
    let mut paren_depth: isize = 0;
    let mut found_open = false;
    let mut expression_body = false;
    let base_indent = indent_level(code_lines[start].1);
    let mut func_code_lines: Vec<(usize, &str)> = Vec::new();
    let mut j = start;

    while j < code_lines.len() {
        let (jidx, jline) = code_lines[j];
        func_code_lines.push((jidx, jline));

        let masked = mask_strings(jline, line_comments);
        let bytes = masked.as_bytes();
        let may_introduce = j == start || may_introduce_body(&masked);
        for (pos, &ch) in bytes.iter().enumerate() {
            match ch {
                b'{' => {
                    brace_depth += 1;
                    found_open = true;
                }
                b'}' => brace_depth -= 1,
                b'(' => paren_depth += 1,
                b')' => paren_depth -= 1,
                // A `=` inside the parameter list is a default value.
                b'=' if may_introduce
                    && !found_open
                    && paren_depth <= 0
                    && is_plain_assign(bytes, pos) =>
                {
                    expression_body = true;
                }
                _ => {}
            }
        }

        let done = if expression_body {
            brace_depth <= 0 && !continues_expression(code_lines.get(j + 1), base_indent)
        } else {
            found_open && brace_depth == 0
        };
        if done {
            break;
        }
        j += 1;
    }

    (func_code_lines, j)
}

/// Detect function boundaries and return their bodies.
/// Works for both brace-scoped and indent-scoped languages.
pub fn detect_function_bodies<'a>(
    all_lines: &[String],
    code_lines: &[(usize, &'a str)],
    markers: &dyn FunctionDetectionMarkers,
) -> Vec<FunctionBody<'a>> {
    let mut functions = Vec::new();

    if markers.brace_scoped() {
        detect_brace_scoped(code_lines, markers, &mut functions);
    } else {
        detect_indent_scoped(all_lines, code_lines, markers, &mut functions);
    }

    functions
}

/// Walk code lines, find function declarations by markers or C-family
/// heuristic, track brace depth to determine body extent.
fn detect_brace_scoped<'a>(
    code_lines: &[(usize, &'a str)],
    markers: &dyn FunctionDetectionMarkers,
    functions: &mut Vec<FunctionBody<'a>>,
) {
    let mut i = 0;
    while i < code_lines.len() {
        let (line_idx, line) = code_lines[i];
        let trimmed = line.trim();

        if is_function_declaration(trimmed, markers) {
            let name = extract_function_name(trimmed, markers);
            let (func_code_lines, end) = find_function_body(code_lines, i, markers.line_comments());
            functions.push(FunctionBody {
                name,
                start_line: line_idx + 1,
                code_lines: func_code_lines,
            });
            i = end + 1;
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Markers(&'static str);

    impl FunctionDetectionMarkers for Markers {
        fn function_markers(&self) -> &[&str] {
            std::slice::from_ref(&self.0)
        }
        fn brace_scoped(&self) -> bool {
            true
        }
        fn line_comments(&self) -> &[&str] {
            &["#"]
        }
    }

    /// Name and number of lines of each function detected in `src`.
    fn bodies(marker: &'static str, src: &str) -> Vec<(String, usize)> {
        let lines: Vec<String> = src.lines().map(String::from).collect();
        let code_lines: Vec<(usize, &str)> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .map(|(i, l)| (i, l.as_str()))
            .collect();
        detect_function_bodies(&lines, &code_lines, &Markers(marker))
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
}

/// Walk code lines for indent-scoped languages (Python, Ruby), using
/// indentation level to determine where function bodies end.
fn detect_indent_scoped<'a>(
    all_lines: &[String],
    code_lines: &[(usize, &'a str)],
    markers: &dyn FunctionDetectionMarkers,
    functions: &mut Vec<FunctionBody<'a>>,
) {
    let mut i = 0;
    while i < code_lines.len() {
        let (line_idx, line) = code_lines[i];
        let trimmed = line.trim();

        let is_function = markers
            .function_markers()
            .iter()
            .any(|m| trimmed.starts_with(m));

        if is_function {
            let name = extract_function_name(trimmed, markers);
            let start_line = line_idx + 1;
            let base_indent = indent_level(line);

            let mut func_code_lines: Vec<(usize, &str)> = vec![(line_idx, line)];
            let mut j = i + 1;

            while j < code_lines.len() {
                let (jidx, jline) = code_lines[j];
                let jline_full = &all_lines[jidx];
                if indent_level(jline_full) <= base_indent && !jline.trim().is_empty() {
                    break;
                }
                func_code_lines.push((jidx, jline));
                j += 1;
            }

            functions.push(FunctionBody {
                name,
                start_line,
                code_lines: func_code_lines,
            });

            i = j;
        } else {
            i += 1;
        }
    }
}
