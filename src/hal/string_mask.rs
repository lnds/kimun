use crate::loc::language::LanguageSpec;

/// For languages with triple-quoted strings (Python), mark interior lines
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

/// Scan a single line for triple-quote delimiters, updating the in-string state.
fn scan_triple_quotes(line: &str, in_triple: &mut Option<&str>) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if let Some(delim) = *in_triple {
            if bytes[i] == b'\\' && i + 1 < len {
                i += 2;
            } else if bytes[i..].starts_with(delim.as_bytes()) {
                *in_triple = None;
                i += delim.len();
            } else {
                i += 1;
            }
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            let q = bytes[i];
            let triple: &str = if q == b'"' { "\"\"\"" } else { "'''" };
            if bytes[i..].starts_with(triple.as_bytes()) {
                *in_triple = Some(triple);
                i += 3;
            } else {
                i += 1;
                while i < len && bytes[i] != q {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
#[path = "string_mask_test.rs"]
mod tests;
