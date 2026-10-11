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

/// What has been read of a function so far: how deep its braces and
/// parentheses are, and what introduces its body.
#[derive(Default)]
struct BodyScan {
    brace_depth: isize,
    paren_depth: isize,
    found_open: bool,
    expression_body: bool,
}

impl BodyScan {
    /// Read one line, already masked. `may_introduce` tells whether a `=`
    /// on it can be the one that introduces the body.
    fn read(&mut self, masked: &str, may_introduce: bool) {
        let bytes = masked.as_bytes();
        for (pos, &ch) in bytes.iter().enumerate() {
            match ch {
                b'{' => {
                    self.brace_depth += 1;
                    self.found_open = true;
                }
                b'}' => self.brace_depth -= 1,
                b'(' => self.paren_depth += 1,
                b')' => self.paren_depth -= 1,
                b'=' if may_introduce && self.outside_signature() => {
                    self.expression_body |= is_plain_assign(bytes, pos);
                }
                _ => {}
            }
        }
    }

    /// Before the body and outside the parameter list, where a `=` is a
    /// default value.
    fn outside_signature(&self) -> bool {
        !self.found_open && self.paren_depth <= 0
    }

    /// Whether the body ends at the line just read, given the one after it.
    fn ends(&self, next: Option<&(usize, &str)>, base_indent: usize) -> bool {
        if self.expression_body {
            self.brace_depth <= 0 && !continues_expression(next, base_indent)
        } else {
            self.found_open && self.brace_depth == 0
        }
    }
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
    let mut scan = BodyScan::default();
    let base_indent = indent_level(code_lines[start].1);
    let mut func_code_lines: Vec<(usize, &str)> = Vec::new();
    let mut j = start;

    while j < code_lines.len() {
        let (jidx, jline) = code_lines[j];
        func_code_lines.push((jidx, jline));

        let masked = mask_strings(jline, line_comments);
        scan.read(&masked, j == start || may_introduce_body(&masked));
        if scan.ends(code_lines.get(j + 1), base_indent) {
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
    if markers.brace_scoped() {
        // Declarations by markers or the C-family heuristic; the body by
        // brace depth.
        detect_with(
            code_lines,
            markers,
            |trimmed| is_function_declaration(trimmed, markers),
            |start| {
                let (body, end) = find_function_body(code_lines, start, markers.line_comments());
                (body, end + 1)
            },
        )
    } else {
        // Indent-scoped languages (Python, Ruby): the body by indentation.
        detect_with(
            code_lines,
            markers,
            |trimmed| {
                let mut function_markers = markers.function_markers().iter();
                function_markers.any(|m| trimmed.starts_with(m))
            },
            |start| indented_body(all_lines, code_lines, start),
        )
    }
}

/// Walk the code lines. Where `is_declaration` accepts a line, `body_at`
/// gives the lines of that function and the index of the first line after it.
fn detect_with<'a>(
    code_lines: &[(usize, &'a str)],
    markers: &dyn FunctionDetectionMarkers,
    is_declaration: impl Fn(&str) -> bool,
    body_at: impl Fn(usize) -> (Vec<(usize, &'a str)>, usize),
) -> Vec<FunctionBody<'a>> {
    let mut functions = Vec::new();
    let mut i = 0;
    while i < code_lines.len() {
        let (line_idx, line) = code_lines[i];
        let trimmed = line.trim();
        if !is_declaration(trimmed) {
            i += 1;
            continue;
        }
        let (body, next) = body_at(i);
        functions.push(FunctionBody {
            name: extract_function_name(trimmed, markers),
            start_line: line_idx + 1,
            code_lines: body,
        });
        i = next;
    }
    functions
}

/// The declaration at `code_lines[start]` and the lines indented deeper
/// than it, with the index of the first line after them.
fn indented_body<'a>(
    all_lines: &[String],
    code_lines: &[(usize, &'a str)],
    start: usize,
) -> (Vec<(usize, &'a str)>, usize) {
    let base_indent = indent_level(code_lines[start].1);
    let len = code_lines[start + 1..]
        .iter()
        .take_while(|(idx, line)| {
            indent_level(&all_lines[*idx]) > base_indent || line.trim().is_empty()
        })
        .count();
    let end = start + 1 + len;
    (code_lines[start..end].to_vec(), end)
}

#[cfg(test)]
#[path = "detection_test.rs"]
mod tests;
