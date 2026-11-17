//! Regular expression literals of JavaScript: a `/` where a value may
//! start, up to the `/` that closes it.

/// Whether a value, rather than an operand of a division, may start after
/// `before`.
pub(crate) fn value_may_start(before: &[u8]) -> bool {
    let text = String::from_utf8_lossy(before);
    let text = text.trim_end();
    match text.bytes().last() {
        None => true,
        Some(b) if b"(,=:[!&|?{};+-*%<>~^".contains(&b) => true,
        Some(_) => ["return", "typeof", "case", "in", "of"].iter().any(|kw| {
            text.ends_with(kw)
                && !text[..text.len() - kw.len()]
                    .ends_with(|c: char| c.is_alphanumeric() || c == '_')
        }),
    }
}

/// Where the regular expression whose body starts at `i` closes.
pub(crate) fn regex_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut class = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'[' => class = true,
            b']' => class = false,
            b'/' if !class => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}
