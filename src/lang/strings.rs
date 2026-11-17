//! String literals as `km loc` reads them: where one opens and each step
//! inside it, by the rules of `literals`, which the analyzers of code share.

use super::language::LanguageSpec;
use super::literals::openers::{Delimited, Literal, opener_at};
use super::literals::syntax::Syntax;

/// The literal being read.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum StringKind {
    Delimited(Delimited),
    /// A template literal, with the `${…}` open inside it.
    Template(Vec<Frame>),
}

/// A level of a template literal: its text, or the code of a `${…}` with
/// the braces open in it.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Frame {
    Text,
    Code(usize),
}

impl StringKind {
    /// Whether it is triple-quoted, as Python's docstrings are.
    pub(super) fn is_triple(&self) -> bool {
        matches!(self, Self::Delimited(d) if d.close == b"\"\"\"" || d.close == b"'''")
    }

    /// Whether it ends with a line that does not go on with a `\`.
    pub(super) fn ends_with_line(&self, goes_on: bool) -> bool {
        matches!(self, Self::Delimited(d) if !d.spans && !goes_on)
    }
}

/// The literal that opens at `pos`, with the width of its opening.
pub(super) fn literal_open(
    bytes: &[u8],
    pos: usize,
    spec: &LanguageSpec,
    syntax: &Syntax,
) -> Option<(usize, StringKind)> {
    let (literal, width) = opener_at(bytes, pos, syntax, spec.backslash_escapes)?;
    let kind = match literal {
        Literal::Delimited(d) => StringKind::Delimited(d),
        Literal::Template => StringKind::Template(vec![Frame::Text]),
    };
    Some((width, kind))
}

/// One step inside a literal: how far to advance, and whether it closes.
pub(super) fn scan(rest: &[u8], kind: &mut StringKind) -> (usize, bool) {
    match kind {
        StringKind::Delimited(d) => delimited_scan(rest, d),
        StringKind::Template(frames) => {
            let advance = template_step(rest, frames);
            (advance, frames.is_empty())
        }
    }
}

fn delimited_scan(rest: &[u8], d: &Delimited) -> (usize, bool) {
    let len = d.close.len();
    let closes = rest.starts_with(&d.close);
    if d.escapes && rest[0] == b'\\' {
        (rest.len().min(2), false)
    } else if closes && d.doubled && rest[len..].starts_with(&d.close) {
        (2 * len, false)
    } else {
        (if closes { len } else { 1 }, closes)
    }
}

fn template_step(rest: &[u8], frames: &mut Vec<Frame>) -> usize {
    match frames.last() {
        Some(Frame::Text) => text_step(rest, frames),
        Some(Frame::Code(depth)) => code_step(rest, *depth, frames),
        None => 1,
    }
}

fn text_step(rest: &[u8], frames: &mut Vec<Frame>) -> usize {
    match rest {
        [b'\\', ..] => rest.len().min(2),
        [b'`', ..] => {
            frames.pop();
            1
        }
        [b'$', b'{', ..] => {
            frames.push(Frame::Code(0));
            2
        }
        _ => 1,
    }
}

fn code_step(rest: &[u8], depth: usize, frames: &mut Vec<Frame>) -> usize {
    let top = frames.len() - 1;
    match rest[0] {
        b'`' => frames.push(Frame::Text),
        b'{' => frames[top] = Frame::Code(depth + 1),
        b'}' if depth == 0 => {
            frames.pop();
        }
        b'}' => frames[top] = Frame::Code(depth - 1),
        b'"' | b'\'' => return quoted_len(rest),
        _ => {}
    }
    1
}

/// The length of the quoted string that starts `rest`, up to the end of
/// the line.
fn quoted_len(rest: &[u8]) -> usize {
    let quote = rest[0];
    let mut i = 1;
    while i < rest.len() && rest[i] != quote {
        i += if rest[i] == b'\\' { 2 } else { 1 };
    }
    (i + 1).min(rest.len())
}

#[cfg(test)]
#[path = "strings_test.rs"]
mod tests;
