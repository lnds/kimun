//! The code of Java and C# as tokens, without comments and literals: its
//! words and its punctuation.
//!
//! A literal is left out whole, whatever it holds: a string with its
//! escapes, a Java text block or a C# raw string (`"""…"""`, closed by as
//! many quotes as opened it), a C# verbatim string (`@"…"`, where `""` is a
//! quote), and a character.

/// A token of code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'s> {
    Word(&'s str),
    Punct(u8),
}

/// The tokens of `source`, past the byte order mark it may start with.
pub fn tokens(source: &str) -> Vec<Token<'_>> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let rest = &bytes[at..];
        let skip = match rest {
            [c, ..] if c.is_ascii_whitespace() => 1,
            [b'/', b'/', ..] => line_len(rest),
            [b'/', b'*', ..] => find(rest, b"*/").map_or(rest.len(), |end| end + 2),
            [b'"', b'"', b'"', ..] => raw_len(rest),
            [b'@' | b'$', b'@' | b'$', b'"', ..] => 2 + verbatim_len(&rest[2..]),
            [b'@', b'"', ..] => 1 + verbatim_len(&rest[1..]),
            [b'$', b'"', ..] => 1 + quoted_len(&rest[1..], b'"'),
            [b'"', ..] => quoted_len(rest, b'"'),
            [b'\'', ..] => quoted_len(rest, b'\''),
            [c, ..] if is_word_start(*c) => {
                let len = rest.iter().take_while(|&&c| is_word(c)).count();
                tokens.push(Token::Word(&source[at..at + len]));
                len
            }
            [c, ..] => {
                tokens.push(Token::Punct(*c));
                1
            }
            [] => break,
        };
        at += skip.max(1);
    }
    tokens
}

fn is_word_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'$' || !c.is_ascii()
}

fn is_word(c: u8) -> bool {
    is_word_start(c) || c.is_ascii_digit()
}

/// The length of a line, without its newline.
pub(super) fn line_len(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .position(|&c| c == b'\n')
        .unwrap_or(bytes.len())
}

/// Where `needle` first is in `bytes`.
pub(super) fn find(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    bytes.windows(needle.len()).position(|w| w == needle)
}

/// The length of a literal closed by `quote`, with backslash escapes; it
/// ends with its line when it is not closed.
pub(super) fn quoted_len(rest: &[u8], quote: u8) -> usize {
    let mut i = 1;
    while let Some(&c) = rest.get(i) {
        match c {
            b'\\' => i += 2,
            b'\n' => return i,
            c if c == quote => return i + 1,
            _ => i += 1,
        }
    }
    rest.len()
}

/// The length of a verbatim string, from its quote: `""` is a quote.
fn verbatim_len(rest: &[u8]) -> usize {
    let mut i = 1;
    while let Some(&c) = rest.get(i) {
        match (c, rest.get(i + 1)) {
            (b'"', Some(b'"')) => i += 2,
            (b'"', _) => return i + 1,
            _ => i += 1,
        }
    }
    rest.len()
}

/// The length of a text block or raw string: closed by as many quotes as
/// opened it.
fn raw_len(rest: &[u8]) -> usize {
    let quotes = rest.iter().take_while(|&&c| c == b'"').count();
    let close = vec![b'"'; quotes];
    find(&rest[quotes..], &close).map_or(rest.len(), |end| quotes + end + quotes)
}

#[cfg(test)]
#[path = "c_like_test.rs"]
mod tests;
