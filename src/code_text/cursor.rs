//! A walk along one line, writing what is code of it.

use super::CodeLine;

pub(super) struct Cursor<'l> {
    line: &'l str,
    pub bytes: &'l [u8],
    pub out: Vec<u8>,
    pub i: usize,
    code: bool,
    literal: bool,
    /// Where the literal being read opened on this line, and its quote.
    opened: Option<(usize, u8)>,
}

impl<'l> Cursor<'l> {
    pub fn new(line: &'l str) -> Self {
        Self {
            line,
            bytes: line.as_bytes(),
            out: line.as_bytes().to_vec(),
            i: 0,
            code: false,
            literal: false,
            opened: None,
        }
    }

    pub fn done(&self) -> bool {
        self.i >= self.bytes.len()
    }

    pub fn rest(&self) -> &'l [u8] {
        &self.bytes[self.i..]
    }

    pub fn left(&self) -> usize {
        self.bytes.len() - self.i
    }

    /// The byte at the cursor is code.
    pub fn keep(&mut self) {
        self.code |= !self.bytes[self.i].is_ascii_whitespace();
        self.i += 1;
    }

    /// The next `n` bytes are no code.
    pub fn blank_n(&mut self, n: usize) {
        let end = (self.i + n).min(self.bytes.len());
        self.out[self.i..end].fill(b' ');
        self.i = end;
    }

    /// A literal opens with the next `n` bytes: its quote, then spaces.
    pub fn open(&mut self, n: usize) {
        let quote = if self.bytes[self.i] == b'\'' {
            b'\''
        } else {
            b'"'
        };
        self.opened = Some((self.i, quote));
        self.literal = true;
        self.blank_n(n);
        self.out[self.opened.map_or(0, |(at, _)| at)] = quote;
    }

    /// The literal closes with the next `n` bytes: its quote when it opened
    /// on this line, nothing otherwise.
    pub fn close(&mut self, n: usize) {
        let at = self.i;
        self.blank_n(n);
        if let Some((_, quote)) = self.opened.take() {
            self.out[at] = quote;
        }
    }

    /// A character literal of `len` bytes past its opening quote, which
    /// stays as `'…'`; a lone quote when `len` is 0.
    pub fn char_literal(&mut self, len: usize) {
        if len == 0 {
            self.blank_n(1);
            return;
        }
        self.literal = true;
        self.i += 1;
        self.blank_n(len - 1);
        self.i += 1;
    }

    /// A marker of `n` bytes standing for a literal written elsewhere: `""`.
    pub fn marker(&mut self, n: usize) {
        let at = self.i;
        self.literal = true;
        self.blank_n(n);
        self.out[at..at + 2].fill(b'"');
    }

    /// A literal still open at the end of the line is closed on it.
    pub fn close_at_end(&mut self) {
        if let Some((at, quote)) = self.opened.take()
            && at + 1 < self.out.len()
        {
            self.out[at + 1] = quote;
        }
    }

    pub fn finish(self, inside: bool) -> CodeLine {
        let text = String::from_utf8(self.out).unwrap_or_else(|_| self.line.to_string());
        CodeLine {
            text: text.trim_end().to_string(),
            code: self.code,
            literal: self.literal,
            inside,
        }
    }
}
