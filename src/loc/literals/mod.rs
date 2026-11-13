//! How each language writes its literals: strings, characters, heredocs
//! and regular expressions. `km loc` and the analyzers of code
//! (`code_text`) read them alike.

pub(crate) mod heredoc;
pub(crate) mod openers;
pub(crate) mod regex;
pub(crate) mod syntax;

/// Length of the character literal whose opening quote precedes `rest`,
/// closing quote included; 0 when it does not close nearby.
pub(crate) fn literal_len(rest: &[u8]) -> usize {
    // An escape may hold a quote (`'\''`) and runs as long as `'\u{10FFFF}'`.
    let (skip, reach) = if rest.first() == Some(&b'\\') {
        (2, 12)
    } else {
        (1, 5)
    };
    let close = rest.iter().take(reach).skip(skip).position(|b| *b == b'\'');
    close.map_or(0, |at| skip + at + 1)
}

/// The length of the character literal whose opening quote precedes
/// `rest`, closing quote included: 0 when the quote is that of a lifetime
/// or a label (`'a`, `'outer:`), or closes nothing nearby.
pub(crate) fn character_len(rest: &[u8]) -> usize {
    let name = rest.iter().take_while(|b| is_word_byte(**b)).count();
    if name > 0 && rest.get(name) != Some(&b'\'') {
        0
    } else {
        literal_len(rest)
    }
}

/// Whether a byte belongs to a name.
pub(crate) fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
