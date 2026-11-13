//! A signature joined over the lines it spans.

use super::*;

/// How many lines a signature may span before it is given up on.
pub(super) const MAX_SIGNATURE_LINES: usize = 30;

/// The signature at line `i`, with a name alone on its line joined to the
/// parameters on the next (`jvp_strtod` above `(struct ...)`) and a Java
/// `throws` clause on the line after them, and the index of its last line.
pub(super) fn shaped_signature(masked: &[String], i: usize) -> (String, usize) {
    let line = masked[i].trim();
    let params_below = masked
        .get(i + 1)
        .is_some_and(|next| next.trim_start().starts_with('('));
    let (sig, end) = if is_identifier(line) && params_below {
        let (params, end) = signature(masked, i + 1);
        (format!("{line} {params}"), end)
    } else {
        signature(masked, i)
    };
    match masked.get(end + 1).map(|next| next.trim()) {
        Some(throws) if throws.starts_with("throws ") && sig.ends_with(')') => {
            (format!("{sig} {throws}"), end + 1)
        }
        _ => (sig, end),
    }
}

/// The line at `i` joined with those after it until its parentheses
/// close, and the index of its last line. Type parameters that a line
/// leaves open at its end (`<`) are followed to the line that closes them
/// (`>`).
pub(super) fn signature(masked: &[String], i: usize) -> (String, usize) {
    let last = (i + MAX_SIGNATURE_LINES).min(masked.len()) - 1;
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut generics = false;
    for (j, line) in masked.iter().enumerate().take(last + 1).skip(i) {
        let line = line.trim();
        parts.push(line);
        depth += paren_balance(line);
        generics = (generics && !line.starts_with('>')) || line.ends_with('<');
        if depth <= 0 && !generics {
            return (parts.join(" "), j);
        }
    }
    (parts.join(" "), last)
}
