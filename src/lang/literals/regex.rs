//! Regular expression literals of JavaScript and Ruby: a `/` where a
//! value may start, up to the `/` that closes it.

use super::is_word_byte;

/// Where a value, rather than an operand of a division, may start in a
/// language that writes regular expressions.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ValueStart {
    /// The operators and opening brackets a value may follow.
    operators: &'static [u8],
    /// The words a value may follow.
    words: &'static [&'static str],
    /// A value may start after a space when none follows, as the argument
    /// of a call without parentheses (Ruby's `split /,/`).
    arguments: bool,
    /// `#{…}` holds code (Ruby).
    interpolates: bool,
}

/// JavaScript and TypeScript.
pub(crate) const SCRIPT: ValueStart = ValueStart {
    operators: b"(,=:[!&|?{};+-*%<>~^",
    words: &["return", "typeof", "case", "in", "of"],
    arguments: false,
    interpolates: false,
};

/// Ruby, where `:` and `?` take the byte after them (`:%` is a Symbol, `?/`
/// a character), so a value starts after them only past a space.
const RUBY: ValueStart = ValueStart {
    operators: b"(,=[!&|{};+-*%<>~^",
    words: &[
        "when", "if", "unless", "elsif", "while", "until", "and", "or", "not", "return",
    ],
    arguments: true,
    interpolates: true,
};

/// The rule of a language that writes regular expressions.
pub(crate) fn value_start(script: bool, name: &str) -> Option<ValueStart> {
    match (script, name) {
        (true, _) => Some(SCRIPT),
        (_, "Ruby") => Some(RUBY),
        _ => None,
    }
}

impl ValueStart {
    /// Whether a value may start after `before`: at the start of the line,
    /// or after an operator, an opening bracket or a word of the language,
    /// but never right after a `<` (`</p>` closes a tag). Only the end of
    /// `before` is read.
    pub(crate) fn after_operator(&self, before: &[u8]) -> bool {
        let text = before.trim_ascii_end();
        match text.last() {
            None => true,
            Some(b'<') if text.len() == before.len() => false,
            Some(b) if self.operators.contains(b) => true,
            Some(_) => self.words.iter().any(|word| ends_with_word(text, word)),
        }
    }

    /// Whether a value may start after `before` as the argument of a call
    /// without parentheses, `next` being the byte after the mark: after a
    /// space, when neither a space nor `=` follows (`split /,/`, while
    /// `a / b` and `x /= 2` divide).
    pub(crate) fn as_argument(&self, before: &[u8], next: Option<&u8>) -> bool {
        let spaced = before.last().is_some_and(u8::is_ascii_whitespace);
        self.arguments && spaced && next.is_some_and(|b| !b.is_ascii_whitespace() && *b != b'=')
    }

    /// Whether a `/` after `before`, with `next` after it, may open a
    /// regular expression.
    pub(crate) fn opens_regex(&self, before: &[u8], next: Option<&u8>) -> bool {
        self.after_operator(before) || self.as_argument(before, next)
    }
}

/// Whether `text` ends with `word`, and not with a longer name.
fn ends_with_word(text: &[u8], word: &str) -> bool {
    let Some(rest) = text.strip_suffix(word.as_bytes()) else {
        return false;
    };
    !rest
        .last()
        .is_some_and(|b| is_word_byte(*b) || !b.is_ascii())
}

/// The width of the regular expression literal that opens at `at`, if one
/// does: a `/` where a value may start after `before` that opens no comment
/// (`//`, `/*`) and closes on its line. One that interpolates (`/#{x}/`) is
/// left as code, as what `#{…}` holds may hold a `/`.
pub(crate) fn regex_width(
    bytes: &[u8],
    at: usize,
    before: &[u8],
    rule: ValueStart,
) -> Option<usize> {
    let next = bytes.get(at + 1);
    let opens =
        bytes[at] == b'/' && !matches!(next, Some(b'/' | b'*')) && rule.opens_regex(before, next);
    let end = opens.then(|| regex_end(bytes, at + 1)).flatten()?;
    let interpolated = rule.interpolates && holds_interpolation(&bytes[at + 1..end]);
    (!interpolated).then_some(end + 1 - at)
}

/// Whether `text` holds a `#{`.
pub(crate) fn holds_interpolation(text: &[u8]) -> bool {
    text.windows(2).any(|pair| pair == b"#{")
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
