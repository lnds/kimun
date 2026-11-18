//! Literals that a mark opens with a delimiter the code picks: Elixir's
//! sigils (`~r/…/`, `~c|…|`) and Ruby's percent literals (`%w[…]`,
//! `%r{…}`), in which brackets nest. And the quotes that open none: Ruby's
//! and Perl's `$'`, a shell's `\'`.

use super::openers::{Delimited, Literal};
use super::regex::holds_interpolation;
use super::syntax::Syntax;

/// What a language marks beyond its quotes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Marks {
    /// Sigils (Elixir).
    pub sigils: bool,
    /// Percent literals (Ruby).
    pub percent: bool,
    /// A quote after `$` names a variable (Ruby, Perl).
    pub dollar_quotes: bool,
    /// Shell quoting: outside a string a backslash escapes what follows,
    /// and inside single quotes nothing escapes.
    pub shell_quoting: bool,
}

impl Marks {
    pub(crate) fn of(name: &str, shell: bool) -> Self {
        let ruby = name == "Ruby";
        Self {
            sigils: matches!(name, "Elixir" | "Elixir Script"),
            percent: ruby,
            dollar_quotes: ruby || name == "Perl",
            shell_quoting: shell,
        }
    }
}

/// The literal a mark opens at `at`: a sigil, or a percent literal. One
/// opens where a value may start, with any delimiter, and as the argument
/// of a call without parentheses (`url.sub %r{…}`), as Ruby reads it, with
/// a bracket or one of the usual delimiters, so that `a %-b` stays a
/// modulo.
pub(crate) fn marked_at(bytes: &[u8], at: usize, syntax: &Syntax) -> Option<(Literal, usize)> {
    let rest = &bytes[at..];
    if syntax.marks.sigils {
        return sigil(rest);
    }
    let rule = syntax
        .regex
        .filter(|_| syntax.marks.percent && rest[0] == b'%')?;
    let before = &bytes[..at];
    if rule.after_operator(before) {
        percent(rest, b"!|/^~-:;,.@#$&*+?")
    } else {
        rule.as_argument(before, rest.get(1))
            .then(|| percent(rest, b"!|/^"))?
    }
}

/// Whether a backslash escapes inside the single quotes that open at `at`:
/// in a shell, only inside ANSI-C quotes (`$'…'`).
pub(crate) fn single_quotes_escape(bytes: &[u8], at: usize, marks: Marks) -> bool {
    !marks.shell_quoting || (at > 0 && bytes[at - 1] == b'$')
}

/// Whether the quote at `at` opens nothing: it names a variable (`$'`), or
/// a backslash escapes it outside a string (`\'`).
pub(crate) fn quote_opens_nothing(bytes: &[u8], at: usize, marks: Marks) -> bool {
    let before = &bytes[..at];
    let escaped = || before.iter().rev().take_while(|b| **b == b'\\').count() % 2 == 1;
    (marks.dollar_quotes && before.last() == Some(&b'$')) || (marks.shell_quoting && escaped())
}

/// A sigil whose delimiter is no quote: `~r/…/`, `~c|…|`, `~w(…)`. A
/// quote opens there what it always opens (`~s"…"`, `~S"""`), and one
/// that interpolates before its close on the line is read as before.
fn sigil(rest: &[u8]) -> Option<(Literal, usize)> {
    let name = rest.strip_prefix(b"~")?;
    let letters = name.iter().take_while(|b| b.is_ascii_alphabetic()).count();
    let close = closer(*name.get(letters)?, b"/|")?;
    let interpolated = name[0].is_ascii_lowercase() && interpolates(&name[letters + 1..], close);
    (letters > 0 && !interpolated).then(|| (delimited(close, None), letters + 2))
}

/// Whether `body`, the rest of the line after the opening of a literal that
/// closes with `close`, holds a `#{` before that close: what `#{…}` holds
/// may close it, so such a literal is not read as one.
fn interpolates(body: &[u8], close: u8) -> bool {
    let end = body.iter().position(|b| *b == close).unwrap_or(body.len());
    holds_interpolation(&body[..end])
}

/// A percent literal: `%w[…]`, `%r{…}`, `%q(…)`, `%(…)`, `%r!…!`, with a
/// bracket or one of `same`. One delimited by quotes is read as the string
/// they open. The brackets in a `#{…}` nest like the others; one whose
/// delimiter does not nest and that interpolates before its close on the
/// line is read as before.
fn percent(rest: &[u8], same: &[u8]) -> Option<(Literal, usize)> {
    let after = rest.strip_prefix(b"%")?;
    let typed = usize::from(after.first().is_some_and(|b| b"qQwWiIrsx".contains(b)));
    let open = *after.get(typed)?;
    let close = closer(open, same)?;
    let nests = (close != open).then_some((open, 0));
    let plain = after.first().is_some_and(|b| b"qwis".contains(b));
    if !plain && nests.is_none() && interpolates(&after[typed + 1..], close) {
        return None;
    }
    Some((delimited(close, nests), typed + 2))
}

/// What closes a literal that `open` opens: the other half of a bracket,
/// or `open` itself when it is one of `same`.
fn closer(open: u8, same: &[u8]) -> Option<u8> {
    match open {
        b'(' => Some(b')'),
        b'[' => Some(b']'),
        b'{' => Some(b'}'),
        b'<' => Some(b'>'),
        _ => same.contains(&open).then_some(open),
    }
}

fn delimited(close: u8, nests: Option<(u8, usize)>) -> Literal {
    Literal::Delimited(Delimited {
        close: vec![close],
        escapes: true,
        doubled: false,
        spans: true,
        nests,
    })
}

impl Delimited {
    /// Steps over the start of `rest` and tells whether it closes the
    /// literal: its close does, unless that closes a bracket nested in it;
    /// a bracket that opens inside nests one more.
    pub(crate) fn step_closes(&mut self, rest: &[u8]) -> bool {
        let closes = rest.starts_with(&self.close);
        match &mut self.nests {
            Some((open, depth)) if rest[0] == *open => *depth += 1,
            Some((_, depth)) if closes && *depth > 0 => *depth -= 1,
            _ => return closes,
        }
        false
    }
}

#[cfg(test)]
#[path = "marked_test.rs"]
mod tests;
