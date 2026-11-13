use std::borrow::Cow;

use crate::case_fold::folded;
use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;

/// For languages with triple-quoted strings (Python, Kaikai), mark interior lines
/// of multi-line strings so the tokenizer can skip them.
/// Only marks lines that both start AND end inside a triple-quoted string
/// (true interior lines). Opening/closing lines are not masked — their
/// string content is handled by `mask_strings` in the tokenizer.
pub fn multi_line_string_mask(lines: &[String], spec: &LanguageSpec) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    if !spec.triple_quote_strings {
        return mask;
    }
    let mut in_triple: Option<&str> = None;
    for (idx, line) in lines.iter().enumerate() {
        let started_in_string = in_triple.is_some();
        scan_triple_quotes(line, &mut in_triple);
        mask[idx] = started_in_string && in_triple.is_some();
    }
    mask
}

/// Mark the lines that start inside a triple-quoted string: the interior
/// lines and the one that closes it. What such a line holds before the
/// closing delimiter is text, whatever it looks like.
pub fn starts_in_string_mask(lines: &[String], spec: &LanguageSpec) -> Vec<bool> {
    if !spec.triple_quote_strings {
        return vec![false; lines.len()];
    }
    let mut in_triple: Option<&str> = None;
    lines
        .iter()
        .map(|line| {
            let started_in_string = in_triple.is_some();
            scan_triple_quotes(line, &mut in_triple);
            started_in_string
        })
        .collect()
}

/// Demote the interior lines of multi-line strings from `Code` to `Blank`.
///
/// Prose and data embedded in a triple-quoted literal are not control flow:
/// counting them inflates complexity and smell metrics. Analyzers that select
/// `LineKind::Code` call this first so those lines drop out.
pub fn demote_multi_line_strings(
    lines: &[String],
    kinds: &[LineKind],
    spec: &LanguageSpec,
) -> Vec<LineKind> {
    if !spec.triple_quote_strings {
        return kinds.to_vec();
    }
    let mask = multi_line_string_mask(lines, spec);
    kinds
        .iter()
        .enumerate()
        .map(|(i, k)| {
            if mask.get(i).copied().unwrap_or(false) {
                LineKind::Blank
            } else {
                *k
            }
        })
        .collect()
}

/// The lines and kinds an analysis of code reads: code lines without
/// their comments and the text of their literals, read by the rules of the
/// language (`code_text`), which is no code whatever it looks like; comment
/// lines as written. The lines inside a literal that spans lines become
/// blanks. A language that ignores case is read in lower case
/// (`case_fold`), and one without backslash escapes without its
/// backslashes.
pub fn code_view<'a>(
    lines: &'a [String],
    kinds: &[LineKind],
    spec: &LanguageSpec,
) -> (Cow<'a, [String]>, Vec<LineKind>) {
    if spec.case_insensitive || !spec.backslash_escapes {
        let kinds = demote_multi_line_strings(lines, kinds, spec);
        let view = folded(lines, spec).into_iter();
        let view = view.map(|line| as_escapes_read(line, spec)).collect();
        return (Cow::Owned(view), kinds);
    }
    let read = crate::code_text::read(lines, spec);
    let kinds: Vec<LineKind> = kinds
        .iter()
        .zip(&read)
        .map(|(kind, line)| as_code(*kind, line))
        .collect();
    let view = read
        .into_iter()
        .zip(lines)
        .zip(&kinds)
        .map(|((line, text), kind)| match kind {
            LineKind::Comment => text.clone(),
            _ => line.text,
        })
        .collect();
    (Cow::Owned(view), kinds)
}

/// The kind of a line for an analysis of code: a line inside a literal is
/// no code nor comment, and one whose code is all within literals or
/// comments is no code.
fn as_code(kind: LineKind, line: &crate::code_text::CodeLine) -> LineKind {
    match kind {
        _ if line.inside => LineKind::Blank,
        LineKind::Code if !line.code && !line.literal => LineKind::Blank,
        _ => kind,
    }
}

/// A line of a language where a backslash escapes nothing, without its
/// backslashes, so that readers that take one for an escape do not run a
/// string such as `'\'` past its end. Neither Fortran nor Pascal has a
/// backslash outside strings.
fn as_escapes_read(line: String, spec: &LanguageSpec) -> String {
    if spec.backslash_escapes {
        line
    } else {
        line.replace('\\', " ")
    }
}

/// Advance past a character while inside a triple-quoted string.
/// Updates `in_triple` when the closing delimiter is found.
/// Returns the new position after consuming one step.
fn advance_inside_triple(
    bytes: &[u8],
    i: usize,
    delim: &str,
    in_triple: &mut Option<&str>,
) -> usize {
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
        i + 2
    } else if bytes[i..].starts_with(delim.as_bytes()) {
        *in_triple = None;
        i + delim.len()
    } else {
        i + 1
    }
}

/// Advance past a single-quoted string literal starting at `i` (after the opening quote).
/// Returns the position after the closing quote.
fn advance_single_quote(bytes: &[u8], mut i: usize, q: u8) -> usize {
    let len = bytes.len();
    while i < len && bytes[i] != q {
        if bytes[i] == b'\\' {
            i += 1;
        }
        i += 1;
    }
    if i < len { i + 1 } else { i }
}

/// Scan a single line for triple-quote delimiters, updating the in-string state.
fn scan_triple_quotes(line: &str, in_triple: &mut Option<&str>) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if let Some(delim) = *in_triple {
            i = advance_inside_triple(bytes, i, delim, in_triple);
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            let q = bytes[i];
            let triple: &str = if q == b'"' { "\"\"\"" } else { "'''" };
            if bytes[i..].starts_with(triple.as_bytes()) {
                *in_triple = Some(triple);
                i += 3;
            } else {
                i = advance_single_quote(bytes, i + 1, q);
            }
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
#[path = "string_mask_test.rs"]
mod tests;
