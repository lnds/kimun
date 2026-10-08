//! Comments and literals of Rust source.
//!
//! Paths are read from code alone. Comments and character literals are
//! blanked; a string keeps its quotes and loses what is inside, which is
//! handed back apart: an attribute such as `#[path = "x.rs"]` needs it.

/// The index past the block comment opened at `from`, which nests.
fn block_comment_end(chars: &[char], from: usize) -> usize {
    let mut depth = 0usize;
    let mut i = from;
    while i < chars.len() {
        if chars[i..].starts_with(&['/', '*']) {
            depth += 1;
            i += 2;
        } else if chars[i..].starts_with(&['*', '/']) {
            depth -= 1;
            i += 2;
            if depth == 0 {
                break;
            }
        } else {
            i += 1;
        }
    }
    i.min(chars.len())
}

/// The index of the quote that closes the string whose content starts at
/// `from`, where a backslash escapes what follows.
fn string_close(chars: &[char], from: usize) -> usize {
    let mut i = from;
    while i < chars.len() && chars[i] != '"' {
        i += if chars[i] == '\\' { 2 } else { 1 };
    }
    i.min(chars.len())
}

/// The raw string that starts at `i` with its `r`, as the index of its
/// opening quote and the number of `#` around it.
fn raw_string(chars: &[char], i: usize) -> Option<(usize, usize)> {
    let named = |at: usize| chars[at].is_ascii_alphanumeric() || chars[at] == '_';
    // `r` has to start a token, alone or after the `b` of a byte string.
    let starts = match i {
        0 => true,
        1 => !named(0) || chars[0] == 'b',
        _ => !named(i - 1) || (chars[i - 1] == 'b' && !named(i - 2)),
    };
    let hashes = chars[i + 1..].iter().take_while(|&&c| c == '#').count();
    let quote = i + 1 + hashes;
    (starts && chars.get(quote) == Some(&'"')).then_some((quote, hashes))
}

/// The index of the quote that closes a raw string opened with `hashes`
/// marks, whose content starts at `from`.
fn raw_close(chars: &[char], from: usize, hashes: usize) -> usize {
    (from..chars.len())
        .find(|&i| {
            chars[i] == '"'
                && chars[i + 1..]
                    .iter()
                    .take(hashes)
                    .filter(|&&c| c == '#')
                    .count()
                    == hashes
        })
        .unwrap_or(chars.len())
}

/// The index past the character literal at `i`, or `i` when the quote
/// there opens a lifetime or a label instead.
fn char_end(chars: &[char], i: usize) -> usize {
    match chars[i + 1..] {
        ['\\', ..] => (i + 3..chars.len())
            .find(|&at| chars[at] == '\'')
            .map_or(chars.len(), |at| at + 1),
        [_, '\'', ..] => i + 3,
        _ => i,
    }
}

/// What a piece of source turns into: nothing but blanks, or a string
/// that keeps its quotes at `open` and `close`.
enum Piece {
    Blank(usize),
    Text {
        open: usize,
        close: usize,
        end: usize,
    },
}

/// The comment or literal that starts at `i`, if one does.
fn piece_at(chars: &[char], i: usize) -> Option<Piece> {
    match chars[i..] {
        ['/', '/', ..] => {
            let len = chars[i..].iter().take_while(|&&c| c != '\n').count();
            Some(Piece::Blank(i + len))
        }
        ['/', '*', ..] => Some(Piece::Blank(block_comment_end(chars, i))),
        ['"', ..] => {
            let close = string_close(chars, i + 1);
            Some(Piece::Text {
                open: i,
                close,
                end: close + 1,
            })
        }
        ['r', ..] => raw_string(chars, i).map(|(open, hashes)| {
            let close = raw_close(chars, open + 1, hashes);
            Piece::Text {
                open,
                close,
                end: close + 1 + hashes,
            }
        }),
        ['\'', ..] => Some(char_end(chars, i))
            .filter(|&end| end > i)
            .map(Piece::Blank),
        _ => None,
    }
}

/// The source with comments and literals blanked, lines kept, and the
/// content of each string, in the order they appear. A string is left as
/// its two quotes around blanks.
pub(super) fn code_and_strings(source: &str) -> (String, Vec<String>) {
    let chars: Vec<char> = source.chars().collect();
    let mut code = String::with_capacity(source.len());
    let mut strings = Vec::new();
    let keep_lines = |c: &char| if *c == '\n' { '\n' } else { ' ' };
    let mut i = 0;
    while i < chars.len() {
        let end = match piece_at(&chars, i) {
            None => {
                code.push(chars[i]);
                i + 1
            }
            Some(Piece::Blank(end)) => {
                code.extend(chars[i..end.min(chars.len())].iter().map(keep_lines));
                end
            }
            Some(Piece::Text { open, close, end }) => {
                let close = close.min(chars.len());
                let end = end.min(chars.len());
                strings.push(chars[(open + 1).min(close)..close].iter().collect());
                code.extend(chars[i..end].iter().enumerate().map(|(at, c)| {
                    let here = i + at;
                    if here == open || (here == close && close < chars.len()) {
                        '"'
                    } else {
                        keep_lines(c)
                    }
                }));
                end
            }
        };
        i = end;
    }
    (code, strings)
}

#[cfg(test)]
#[path = "rust_literals_test.rs"]
mod tests;
