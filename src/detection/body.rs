//! The body of a brace-scoped function: where it opens and closes, read
//! over lines whose literals and comments are blanked.

use super::*;

/// Whether the `=` at `pos` stands alone, rather than being part of an
/// operator such as `==`, `=>`, `<=` or `+=`.
pub(super) fn is_plain_assign(bytes: &[u8], pos: usize) -> bool {
    let before = pos.checked_sub(1).map(|p| bytes[p]);
    let after = bytes.get(pos + 1).copied();
    !before.is_some_and(|b| b"=!<>+-*/%&|^:~".contains(&b))
        && !after.is_some_and(|b| b"=>~".contains(&b))
}

/// Whether a line after the declaration can hold the `=` that introduces
/// the body: the one that closes the parameter list, or one the `=` ends.
/// Elsewhere in a signature a `=` binds something else, such as the
/// associated type of `impl Iterator<Item = T>`.
pub(super) fn may_introduce_body(masked: &str) -> bool {
    masked.trim_start().starts_with(')') || masked.trim_end().ends_with('=')
}

/// Whether `next` still belongs to an expression body declared at
/// `base_indent`: it is indented deeper, or it opens the block of a
/// signature whose `=` was not the body (`where T: Iterator<Item = u32>`).
pub(super) fn continues_expression(next: Option<&(usize, &str)>, base_indent: usize) -> bool {
    next.is_some_and(|(_, line)| {
        indent_level(line) > base_indent || line.trim_start().starts_with('{')
    })
}

/// What has been read of a function so far: how deep its braces and
/// parentheses are, and what introduces its body.
#[derive(Default)]
pub(super) struct BodyScan {
    pub(super) brace_depth: isize,
    pub(super) paren_depth: isize,
    pub(super) found_open: bool,
    pub(super) expression_body: bool,
    /// The declaration ended in `;` before any body: there is none.
    pub(super) declaration_only: bool,
    /// Braces open in the parameter list: a type or a pattern.
    pub(super) param_braces: usize,
}

impl BodyScan {
    /// Read one line, already masked. `may_introduce` tells whether a `=`
    /// on it can be the one that introduces the body.
    pub(super) fn read(&mut self, masked: &str, may_introduce: bool) {
        let bytes = masked.as_bytes();
        let last = masked.trim_end().len().saturating_sub(1);
        for (pos, &ch) in bytes.iter().enumerate() {
            match ch {
                b'{' => self.open_brace(pos == last),
                b'}' => self.close_brace(),
                b'(' => self.paren_depth += 1,
                b')' => self.paren_depth -= 1,
                b'=' if may_introduce && self.outside_signature() => {
                    self.expression_body |= is_plain_assign(bytes, pos);
                }
                _ => {}
            }
        }
        self.declaration_only =
            !self.expression_body && self.outside_signature() && masked.trim_end().ends_with(';');
    }

    /// A `{`. Inside the parameter list, one that does not end the line is
    /// a type or a pattern; one that ends it opens a body, such as that of
    /// a function passed as an argument.
    fn open_brace(&mut self, ends_line: bool) {
        if self.outside_body_in_params() && !ends_line {
            self.param_braces += 1;
        } else {
            self.brace_depth += 1;
            self.found_open = true;
        }
    }

    fn close_brace(&mut self) {
        if self.param_braces > 0 {
            self.param_braces -= 1;
        } else {
            self.brace_depth -= 1;
        }
    }

    /// Before the body and inside the parameter list.
    fn outside_body_in_params(&self) -> bool {
        !self.found_open && self.paren_depth > 0
    }

    /// Before the body and outside the parameter list, where a `=` is a
    /// default value.
    fn outside_signature(&self) -> bool {
        !self.found_open && self.paren_depth <= 0
    }

    /// Whether the body ends at the line just read, given the one after it.
    fn ends(&self, next: Option<&(usize, &str)>, base_indent: usize) -> bool {
        if self.declaration_only {
            true
        } else if self.expression_body {
            self.brace_depth <= 0 && !continues_expression(next, base_indent)
        } else {
            self.found_open && self.brace_depth == 0
        }
    }
}

/// Starting from `code_lines[start]`, collect all lines belonging to the
/// function body by tracking brace depth over `masked`, the same lines with
/// literals blanked. Returns `(func_code_lines, end_index)` where
/// `end_index` is the index of the last line of the body in `code_lines`.
///
/// A body introduced by `=` instead of a brace (`fn f(x) = expr`) has no
/// closing delimiter of its own: it runs while its braces are open and
/// while the lines that follow are indented deeper than the declaration.
pub(super) fn find_function_body<'a>(
    code_lines: &[(usize, &'a str)],
    masked: &[String],
    start: usize,
    expression_bodies: bool,
) -> (Vec<(usize, &'a str)>, usize) {
    let mut scan = BodyScan::default();
    let base_indent = indent_level(code_lines[start].1);
    let mut func_code_lines: Vec<(usize, &str)> = Vec::new();
    let mut j = start;

    while j < code_lines.len() {
        func_code_lines.push(code_lines[j]);
        let may_introduce = j == start || may_introduce_body(&masked[j]);
        scan.read(&masked[j], expression_bodies && may_introduce);
        if scan.ends(code_lines.get(j + 1), base_indent) {
            break;
        }
        j += 1;
    }

    if scan.declaration_only {
        func_code_lines.clear();
    }
    (func_code_lines, j)
}

/// A code line with its literals and comments blanked, as the detector
/// reads it.
pub(super) fn masked(line: &str, markers: &dyn FunctionDetectionMarkers) -> String {
    let comments = markers.line_comments();
    match markers.function_syntax() {
        FunctionSyntax::KeywordWithLifetimes => mask_strings(&without_lifetimes(line), comments),
        FunctionSyntax::Script => without_regex_literals(&mask_strings(line, comments)),
        _ => mask_strings(line, comments),
    }
}

/// Blank the quote of each lifetime (`'a`, `'static`) and loop label, so
/// that masking does not take it for the start of a character literal and
/// hide the rest of the line. Character literals are left as they are.
pub(super) fn without_lifetimes(line: &str) -> String {
    let mut bytes = line.as_bytes().to_vec();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\'' {
            i += 1;
            continue;
        }
        let rest = &bytes[i + 1..];
        let name = rest
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric() || **b == b'_')
            .count();
        if name > 0 && rest.get(name) != Some(&b'\'') {
            bytes[i] = b' ';
            i += 1 + name;
        } else {
            i += 1 + literal_len(rest);
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| line.to_string())
}

/// Length of the character literal whose opening quote precedes `rest`,
/// closing quote included; 0 when it does not close nearby.
pub(super) fn literal_len(rest: &[u8]) -> usize {
    // An escape may hold a quote (`'\''`) and runs as long as `'\u{10FFFF}'`.
    let (skip, reach) = if rest.first() == Some(&b'\\') {
        (2, 12)
    } else {
        (1, 5)
    };
    let close = rest.iter().take(reach).skip(skip).position(|b| *b == b'\'');
    close.map_or(0, |at| skip + at + 1)
}
