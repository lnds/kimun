//! Heredocs: a marker on a line, the text on the lines after it, up to a
//! line that holds the terminator.

use super::is_word_byte;
use super::syntax::Heredocs;

/// A heredoc whose text starts on the next line.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Heredoc {
    pub terminator: String,
    /// The terminator may be indented.
    pub indented: bool,
}

/// The heredoc whose marker starts at `at`, with the length of the marker.
pub(crate) fn heredoc_at(bytes: &[u8], at: usize, flavor: Heredocs) -> Option<(Heredoc, usize)> {
    let rest = &bytes[at..];
    let (arrows, bare_needs_upper) = match flavor {
        Heredocs::None => return None,
        Heredocs::Php => (3, false),
        Heredocs::Ruby => (2, true),
        Heredocs::Shell | Heredocs::Perl => (2, false),
    };
    if rest.len() <= arrows || rest[..arrows].iter().any(|b| *b != b'<') {
        return None;
    }
    let mut i = arrows;
    let dash = matches!(rest[i], b'-' | b'~') && flavor != Heredocs::Php;
    i += usize::from(dash);
    if flavor == Heredocs::Php {
        i += rest[i..].iter().take_while(|b| **b == b' ').count();
    }
    let quote = matches!(rest.get(i), Some(b'\'' | b'"')).then(|| rest[i]);
    i += usize::from(quote.is_some());
    let word = word_len(&rest[i..]);
    let bare = !dash && quote.is_none();
    if word == 0 || (bare && bare_needs_upper && !rest[i].is_ascii_uppercase()) {
        return None;
    }
    let terminator = String::from_utf8_lossy(&rest[i..i + word]).into_owned();
    i += word;
    if let Some(q) = quote {
        if rest.get(i) != Some(&q) {
            return None;
        }
        i += 1;
    }
    let indented = dash || flavor == Heredocs::Php;
    Some((
        Heredoc {
            terminator,
            indented,
        },
        i,
    ))
}

/// Where the terminator of `heredoc` ends on `line`, when the line holds it.
pub(crate) fn terminator_end(line: &str, heredoc: &Heredoc) -> Option<usize> {
    let start = if heredoc.indented {
        line.len() - line.trim_start().len()
    } else {
        0
    };
    let end = start + heredoc.terminator.len();
    let holds = line[start..].starts_with(&heredoc.terminator)
        && !line.as_bytes().get(end).is_some_and(|b| is_word_byte(*b));
    holds.then_some(end)
}

fn word_len(bytes: &[u8]) -> usize {
    if !bytes
        .first()
        .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
    {
        return 0;
    }
    bytes.iter().take_while(|b| is_word_byte(**b)).count()
}
