//! Functions of indent-scoped languages (Python, Ruby, Elixir): declared
//! by a marker at the start of a line, their body by indentation.

use super::*;

/// Indent-scoped languages (Python, Ruby, Elixir): declarations by marker
/// at the start of a line, the body by indentation.
pub(super) fn detect_indent_scoped<'a>(
    all_lines: &[String],
    code_lines: &[(usize, &'a str)],
    markers: &dyn FunctionDetectionMarkers,
) -> Vec<FunctionBody<'a>> {
    let mut functions = Vec::new();
    let mut i = 0;
    while i < code_lines.len() {
        let (line_idx, line) = code_lines[i];
        let trimmed = line.trim();
        let mut function_markers = markers.function_markers().iter();
        if !function_markers.any(|m| trimmed.starts_with(m)) {
            i += 1;
            continue;
        }
        let (body, next) = indented_body(all_lines, code_lines, i);
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
/// than it, with the index of the line to go on from: the first after
/// them, or the first inside a `quote` block among them. The code of a
/// `quote` is generated, not run by the function: it is left out of the
/// body, and the functions it declares are measured on their own.
pub(super) fn indented_body<'a>(
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
    let quoted = quoted_lines(all_lines, &code_lines[start..end]);
    let body = code_lines[start..end]
        .iter()
        .zip(&quoted)
        .filter(|(_, q)| !**q)
        .map(|(line, _)| *line)
        .collect();
    let first_quoted = quoted.iter().position(|q| *q).map(|k| start + k);
    (body, first_quoted.unwrap_or(end))
}

/// Which of `lines` sit inside a `quote` block: indented deeper than the
/// line that opens it.
fn quoted_lines(all_lines: &[String], lines: &[(usize, &str)]) -> Vec<bool> {
    let mut quote: Option<usize> = None;
    lines
        .iter()
        .map(|&(idx, line)| {
            let indent = indent_level(&all_lines[idx]);
            let inside = quote.is_some_and(|q| indent > q || line.trim().is_empty());
            if !inside {
                quote = opens_quote(line.trim()).then_some(indent);
            }
            inside
        })
        .collect()
}

/// Whether a line opens an Elixir `quote` block: `quote do`, or `quote`
/// with options before its `do`.
pub(super) fn opens_quote(trimmed: &str) -> bool {
    trimmed == "quote do" || (trimmed.starts_with("quote ") && trimmed.ends_with(" do"))
}
