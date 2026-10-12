use std::borrow::Cow;

use crate::loc::counter::{LineKind, RawSpan, raw_string_spans};
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

/// The lines and kinds an analysis of code reads: without the text of
/// literals that span lines or hold quotes, which is no code whatever it
/// looks like. The interior lines of multi-line strings become blanks, and
/// each raw string is left as `""`.
pub fn code_view<'a>(
    lines: &'a [String],
    kinds: &[LineKind],
    spec: &LanguageSpec,
) -> (Cow<'a, [String]>, Vec<LineKind>) {
    let mut kinds = demote_multi_line_strings(lines, kinds, spec);
    let may_hold_one = |line: &String| line.contains("r\"") || line.contains("r#");
    if !spec.raw_strings || !lines.iter().any(may_hold_one) {
        return (Cow::Borrowed(lines), kinds);
    }
    let mut view = lines.to_vec();
    for (at, spans) in raw_string_spans(lines, spec).iter().enumerate() {
        if spans.is_empty() {
            continue;
        }
        view[at] = without_raw_text(&lines[at], spans);
        if view[at].is_empty() {
            kinds[at] = LineKind::Blank;
        }
    }
    (Cow::Owned(view), kinds)
}

/// `line` with each raw string blanked, and `""` where one opens.
fn without_raw_text(line: &str, spans: &[RawSpan]) -> String {
    let mut bytes = line.as_bytes().to_vec();
    for span in spans {
        bytes[span.start..span.end].fill(b' ');
        if span.opens {
            bytes[span.start..span.start + 2].fill(b'"');
        }
    }
    let text = String::from_utf8(bytes).unwrap_or_default();
    text.trim_end().to_string()
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
