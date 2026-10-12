//! Raw strings: `r"…"`, `r#"…"#` and the forms with more `#`.
//!
//! Nothing is escaped inside one, and one opened with `#` ends only at a
//! quote followed by as many. It spans lines.
use super::fsm::StringKind;

/// Detect a raw string opening at `pos`: an `r`, any number of `#` and a
/// quote. The `r` starts the literal, alone or after a `b` or `c` prefix;
/// at the end of a name it is a letter like any other. Returns the width
/// of what opens it and the kind opened.
pub(super) fn open(bytes: &[u8], pos: usize) -> Option<(usize, StringKind)> {
    let rest = &bytes[pos..];
    if rest[0] != b'r' {
        return None;
    }
    let hashes = rest[1..].iter().take_while(|b| **b == b'#').count();
    if rest.get(1 + hashes) != Some(&b'"') {
        return None;
    }
    let before = match &bytes[..pos] {
        [before @ .., b'b' | b'c'] => before,
        before => before,
    };
    let in_a_name = |b: &u8| b.is_ascii_alphanumeric() || *b == b'_' || !b.is_ascii();
    (!before.last().is_some_and(in_a_name)).then_some((hashes + 2, StringKind::Raw(hashes)))
}

/// Scan one byte inside a raw string opened with `hashes`: how far to
/// advance, and whether the literal closes here.
pub(super) fn scan(rest: &[u8], hashes: usize) -> (usize, bool) {
    let closing = rest[1..].iter().take_while(|b| **b == b'#').count();
    let closed = rest[0] == b'"' && closing >= hashes;
    (if closed { 1 + hashes } else { 1 }, closed)
}
