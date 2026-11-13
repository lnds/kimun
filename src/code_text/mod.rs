//! The code of each line of a source, without its comments and the text of
//! its literals, read by the rules of its language: what analyzers of code
//! measure. The file is read whole, so a string or a comment that spans
//! lines is known on every line it covers.
//!
//! Each line keeps its length and columns. A comment becomes spaces; a
//! literal becomes spaces within its quotes, so `"…"` or `'…'` stays where
//! one is written and is closed on the line it opens, and nothing is left of
//! it on the lines it continues on.

mod cursor;

use crate::loc::language::LanguageSpec;
use crate::loc::literals::character_len;
use crate::loc::literals::heredoc::{self, Heredoc, terminator_end};
use crate::loc::literals::openers::{Delimited, Literal, opener_at};
use crate::loc::literals::regex::{regex_end, value_may_start};
use crate::loc::literals::syntax::Syntax;
use cursor::Cursor;

/// A line as code: its text, whether code is left on it, whether a
/// literal opens on it, and whether it lies inside one that opened before
/// it and goes on after it.
#[derive(Debug, PartialEq)]
pub struct CodeLine {
    pub text: String,
    pub code: bool,
    pub literal: bool,
    pub inside: bool,
}

/// Each of `lines` as code.
pub fn read(lines: &[String], spec: &LanguageSpec) -> Vec<CodeLine> {
    let mut lexer = Lexer::new(spec);
    lines.iter().map(|line| lexer.line(line)).collect()
}

/// A block comment being read.
#[derive(Clone, Copy, Debug)]
struct Comment {
    open: &'static str,
    close: &'static str,
    depth: usize,
}

#[derive(Debug)]
enum State {
    Code,
    Literal(Literal),
    Comment(Comment),
    Heredoc(Heredoc),
}

struct Lexer<'s> {
    spec: &'s LanguageSpec,
    syntax: Syntax,
    state: State,
    /// The braces open in each `${…}` of a template literal being read.
    templates: Vec<usize>,
    /// A heredoc whose text starts on the next line.
    pending: Option<Heredoc>,
}

impl<'s> Lexer<'s> {
    fn new(spec: &'s LanguageSpec) -> Self {
        Self {
            spec,
            syntax: Syntax::of(spec),
            state: State::Code,
            templates: Vec::new(),
            pending: None,
        }
    }

    fn line(&mut self, line: &str) -> CodeLine {
        let started_inside = self.in_literal();
        let mut cur = Cursor::new(line);
        if let State::Heredoc(heredoc) = &self.state {
            let end = terminator_end(line, heredoc);
            cur.blank_n(end.unwrap_or(line.len()));
            if end.is_some() {
                self.state = State::Code;
            }
        }
        while !cur.done() {
            self.step(&mut cur);
        }
        self.end_line(&mut cur);
        cur.finish(started_inside && self.in_literal())
    }

    fn in_literal(&self) -> bool {
        matches!(self.state, State::Literal(_) | State::Heredoc(_))
    }

    fn step(&mut self, cur: &mut Cursor) {
        match std::mem::replace(&mut self.state, State::Code) {
            State::Code => self.code_step(cur),
            State::Literal(Literal::Template) => self.template_step(cur),
            State::Literal(Literal::Delimited(d)) => self.delimited_step(cur, d),
            State::Comment(c) => self.comment_step(cur, c),
            State::Heredoc(h) => self.state = State::Heredoc(h),
        }
    }

    fn code_step(&mut self, cur: &mut Cursor) {
        if let Some(comment) = self.comment_at(cur) {
            cur.blank_n(comment.open.len());
            self.state = State::Comment(comment);
        } else if self.line_comment_at(cur) {
            cur.blank_n(cur.left());
        } else if let Some((literal, len)) =
            opener_at(cur.bytes, cur.i, &self.syntax, self.spec.backslash_escapes)
        {
            cur.open(len);
            self.state = State::Literal(literal);
        } else if !(self.quote_mark(cur)
            || self.regex(cur)
            || self.heredoc(cur)
            || self.template_close(cur))
        {
            cur.keep();
        }
    }

    fn comment_at(&self, cur: &Cursor) -> Option<Comment> {
        [self.spec.block_comment, self.spec.other_block_comment]
            .into_iter()
            .flatten()
            .find(|(open, _)| cur.rest().starts_with(open.as_bytes()))
            .map(|(open, close)| Comment {
                open,
                close,
                depth: 1,
            })
    }

    fn line_comment_at(&self, cur: &Cursor) -> bool {
        let rest = cur.rest();
        let Some(marker) = self
            .spec
            .line_comments
            .iter()
            .find(|m| rest.starts_with(m.as_bytes()))
        else {
            return false;
        };
        let not_before = self.spec.line_comment_not_before.as_bytes();
        let blocked = rest
            .get(marker.len())
            .is_some_and(|b| not_before.contains(b));
        let code = *marker == "#" && self.syntax.hash_is_code(cur.bytes, cur.i);
        !(blocked || code)
    }

    fn comment_step(&mut self, cur: &mut Cursor, mut comment: Comment) {
        let rest = cur.rest();
        if self.spec.nested_block_comments && rest.starts_with(comment.open.as_bytes()) {
            comment.depth += 1;
            cur.blank_n(comment.open.len());
        } else if rest.starts_with(comment.close.as_bytes()) {
            cur.blank_n(comment.close.len());
            comment.depth -= 1;
            if comment.depth == 0 {
                return;
            }
        } else {
            cur.blank_n(1);
        }
        self.state = State::Comment(comment);
    }

    fn delimited_step(&mut self, cur: &mut Cursor, d: Delimited) {
        let rest = cur.rest();
        let len = d.close.len();
        if d.escapes && rest[0] == b'\\' {
            cur.blank_n(2);
        } else if d.doubled && rest.starts_with(&d.close) && rest[len..].starts_with(&d.close) {
            cur.blank_n(2 * len);
        } else if rest.starts_with(&d.close) {
            cur.close(len);
            return;
        } else {
            cur.blank_n(1);
        }
        self.state = State::Literal(Literal::Delimited(d));
    }

    fn template_step(&mut self, cur: &mut Cursor) {
        let rest = cur.rest();
        if rest[0] == b'\\' {
            cur.blank_n(2);
        } else if rest[0] == b'`' {
            cur.close(1);
            return;
        } else if rest.starts_with(b"${") {
            cur.close(2);
            self.templates.push(0);
            return;
        } else {
            cur.blank_n(1);
        }
        self.state = State::Literal(Literal::Template);
    }

    /// The `}` that ends a `${…}`: the template literal goes on.
    fn template_close(&mut self, cur: &mut Cursor) -> bool {
        let Some(depth) = self.templates.last_mut() else {
            return false;
        };
        match cur.bytes[cur.i] {
            b'{' => *depth += 1,
            b'}' if *depth > 0 => *depth -= 1,
            b'}' => {
                self.templates.pop();
                cur.open(1);
                self.state = State::Literal(Literal::Template);
                return true;
            }
            _ => {}
        }
        false
    }

    /// A quote that opens no string: a character literal, the quote of a
    /// lifetime or a label, or the code of a quote character (`?"`).
    fn quote_mark(&self, cur: &mut Cursor) -> bool {
        if cur.bytes[cur.i] == b'\'' && !self.syntax.single_quotes {
            cur.char_literal(character_len(&cur.rest()[1..]));
            return true;
        }
        let Some(len) = self.syntax.char_code_len(cur.bytes, cur.i) else {
            return false;
        };
        cur.keep();
        cur.blank_n(len - 1);
        true
    }

    fn regex(&self, cur: &mut Cursor) -> bool {
        let opens =
            self.syntax.regex && cur.bytes[cur.i] == b'/' && value_may_start(&cur.out[..cur.i]);
        let Some(end) = opens.then(|| regex_end(cur.bytes, cur.i + 1)).flatten() else {
            return false;
        };
        cur.keep();
        cur.blank_n(end - cur.i);
        cur.keep();
        true
    }

    fn heredoc(&mut self, cur: &mut Cursor) -> bool {
        if self.pending.is_some() {
            return false;
        }
        let Some((heredoc, len)) = heredoc::heredoc_at(cur.bytes, cur.i, self.syntax.heredocs)
        else {
            return false;
        };
        self.pending = Some(heredoc);
        cur.marker(len);
        true
    }

    fn end_line(&mut self, cur: &mut Cursor) {
        if let State::Literal(Literal::Delimited(d)) = &self.state
            && !d.spans
            && !cur.bytes.ends_with(b"\\")
        {
            self.state = State::Code;
        }
        cur.close_at_end();
        if let Some(heredoc) = self.pending.take() {
            self.state = State::Heredoc(heredoc);
        }
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
