//! Where a string literal opens, and what closes it.

use super::heredoc::is_word_byte;
use super::syntax::{Backticks, Syntax};

/// A literal that may span lines.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Literal {
    Delimited(Delimited),
    /// A JavaScript template literal, whose `${…}` holds code.
    Template,
}

/// A literal closed by a fixed delimiter.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Delimited {
    pub close: Vec<u8>,
    /// A backslash escapes the next character.
    pub escapes: bool,
    /// The delimiter doubled stands for itself (C#'s `""` in `@"…"`).
    pub doubled: bool,
    /// It goes on past the end of a line without a `\` there.
    pub spans: bool,
}

/// The literal that opens at `at`, with the length of its opening.
pub(super) fn opener_at(
    bytes: &[u8],
    at: usize,
    syntax: &Syntax,
    escapes: bool,
) -> Option<(Literal, usize)> {
    let rest = &bytes[at..];
    let starts_word = at == 0 || !is_word_byte(bytes[at - 1]);
    let prefixed = (syntax.rust_raw && starts_word)
        .then(|| rust_raw(rest))
        .flatten()
        .or_else(|| {
            (syntax.cpp_raw && starts_word)
                .then(|| cpp_raw(rest))
                .flatten()
        })
        .or_else(|| syntax.verbatim.then(|| verbatim(rest)).flatten());
    prefixed
        .or_else(|| triple(rest, syntax, escapes))
        .or_else(|| quoted(rest, syntax, escapes))
}

fn delimited(close: &[u8], escapes: bool, spans: bool) -> Literal {
    Literal::Delimited(Delimited {
        close: close.to_vec(),
        escapes,
        doubled: false,
        spans,
    })
}

fn triple(rest: &[u8], syntax: &Syntax, escapes: bool) -> Option<(Literal, usize)> {
    let opens = rest.starts_with(b"\"\"\"") || (syntax.single_quotes && rest.starts_with(b"'''"));
    (syntax.triple_quotes && opens).then(|| (delimited(&rest[..3], escapes, true), 3))
}

fn quoted(rest: &[u8], syntax: &Syntax, escapes: bool) -> Option<(Literal, usize)> {
    let literal = match rest.first()? {
        b'"' => delimited(b"\"", escapes, syntax.quotes_span_lines),
        b'\'' if syntax.single_quotes => delimited(b"'", escapes, syntax.quotes_span_lines),
        b'`' if syntax.backticks == Backticks::Raw => delimited(b"`", false, true),
        b'`' if syntax.backticks == Backticks::Template => Literal::Template,
        _ => return None,
    };
    Some((literal, 1))
}

/// `r"…"`, `r#"…"#`, `br"…"`, `cr#"…"#`.
fn rust_raw(rest: &[u8]) -> Option<(Literal, usize)> {
    let prefix = [&b"r"[..], b"br", b"cr"]
        .into_iter()
        .find(|p| rest.starts_with(p))?
        .len();
    let hashes = rest[prefix..].iter().take_while(|b| **b == b'#').count();
    if rest.get(prefix + hashes) != Some(&b'"') {
        return None;
    }
    let mut close = vec![b'"'];
    close.extend(std::iter::repeat_n(b'#', hashes));
    Some((delimited(&close, false, true), prefix + hashes + 1))
}

/// `R"delim(…)delim"`, with an encoding prefix too.
fn cpp_raw(rest: &[u8]) -> Option<(Literal, usize)> {
    let prefix = [&b"R\""[..], b"u8R\"", b"uR\"", b"UR\"", b"LR\""]
        .into_iter()
        .find(|p| rest.starts_with(p))?
        .len();
    let delim = rest[prefix..].iter().take(17).position(|b| *b == b'(')?;
    let tag = &rest[prefix..prefix + delim];
    if tag.iter().any(|b| b" )\\\t\"".contains(b)) {
        return None;
    }
    let mut close = vec![b')'];
    close.extend_from_slice(tag);
    close.push(b'"');
    Some((delimited(&close, false, true), prefix + delim + 1))
}

/// `@"…"`, `$@"…"`, `@$"…"`.
fn verbatim(rest: &[u8]) -> Option<(Literal, usize)> {
    let len = [&b"@\""[..], b"$@\"", b"@$\""]
        .into_iter()
        .find(|p| rest.starts_with(p))?
        .len();
    let literal = Literal::Delimited(Delimited {
        close: b"\"".to_vec(),
        escapes: false,
        doubled: true,
        spans: true,
    });
    Some((literal, len))
}
