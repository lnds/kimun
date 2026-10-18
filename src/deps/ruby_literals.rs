//! The code of Ruby as tokens, line by line, without comments and with each
//! literal as one token.
//!
//! Comments (`#`, `=begin` … `=end`) and what follows `__END__` give no
//! tokens. A string, a heredoc, a percent literal, a regular expression and
//! a character are one `Str` token on the line they start on, with the text
//! they hold outside interpolation; the code a string interpolates (`#{…}`)
//! is read as code. The lines a heredoc or a string spans after the first
//! have no tokens of their own.

/// A token of code.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// An identifier or a keyword, with the `?` or `!` it may end in.
    Word(String),
    /// A literal and its text; `true` when it interpolates code.
    Str(String, bool),
    Symbol(String),
    /// `::`.
    Scope,
    /// An instance, class or global variable, or a number.
    Value,
    Punct(u8),
}

/// The tokens of one line.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Line {
    /// The column of its first token.
    pub indent: usize,
    pub tokens: Vec<Tok>,
}

/// Words after which an operand starts, as after an operator.
const LEADING: &[&str] = &[
    "if", "unless", "while", "until", "and", "or", "not", "return", "when", "in", "case", "elsif",
    "then", "do", "else", "yield", "begin", "puts", "p", "raise",
];

/// What came before, to tell a literal from an operator: `/` divides after
/// a value and opens a regular expression after an operator.
#[derive(Clone, Copy, PartialEq)]
enum Before {
    /// Start of input, an operator, or a word an operand follows.
    Operand,
    /// An identifier: a method a literal may be passed to.
    Ident,
    Value,
}

/// What ends a literal: its closing byte, outside nested pairs and, in a
/// regular expression, outside a character class.
struct Delimiters {
    open: u8,
    close: u8,
    depth: usize,
    class: bool,
}

impl Delimiters {
    /// Whether `c` closes the literal, noting the pairs it opens or closes.
    fn closes(&mut self, c: u8) -> bool {
        if self.open == b'/' && matches!(c, b'[' | b']') {
            self.class = c == b'[';
            return false;
        }
        if self.class {
            return false;
        }
        if c == self.close && self.depth == 0 {
            return true;
        }
        if self.open != self.close {
            if c == self.open {
                self.depth += 1;
            } else if c == self.close {
                self.depth -= 1;
            }
        }
        false
    }
}

/// The opening of a heredoc: `<<~ID`, `<<-'ID'`, `<<ID`.
struct Opener {
    id: String,
    indented: bool,
    quote: Option<u8>,
    /// Its length in bytes.
    len: usize,
}

/// A heredoc whose body starts on the next line.
struct Heredoc {
    id: String,
    /// `<<~` or `<<-`: the terminator may be indented.
    indented: bool,
    /// Where its token is, to fill in the body.
    line: usize,
    token: usize,
}

struct Lexer<'s> {
    src: &'s [u8],
    at: usize,
    line: usize,
    line_start: usize,
    lines: Vec<Line>,
    before: Before,
    /// Whether whitespace precedes the current byte.
    spaced: bool,
    heredocs: Vec<Heredoc>,
}

/// The tokens of `source`, one entry per line.
pub fn lines(source: &str) -> Vec<Line> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut lexer = Lexer {
        src: source.as_bytes(),
        at: 0,
        line: 0,
        line_start: 0,
        lines: vec![Line::default()],
        before: Before::Operand,
        spaced: false,
        heredocs: Vec::new(),
    };
    lexer.code(false);
    lexer.lines
}

fn is_word_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c >= 0x80
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80
}

/// The byte that closes a literal opened by `open`.
fn closing(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        other => other,
    }
}

impl Lexer<'_> {
    fn peek(&self, ahead: usize) -> u8 {
        self.src.get(self.at + ahead).copied().unwrap_or(0)
    }

    fn push(&mut self, token: Tok) {
        let start = self.at;
        let column = start.saturating_sub(self.line_start);
        let line = &mut self.lines[self.line];
        if line.tokens.is_empty() {
            line.indent = column;
        }
        line.tokens.push(token);
    }

    fn text(&self, from: usize, to: usize) -> String {
        String::from_utf8_lossy(&self.src[from..to]).into_owned()
    }

    fn newline(&mut self) {
        self.at += 1;
        self.line += 1;
        self.line_start = self.at;
        self.lines.push(Line::default());
    }

    /// Count the newlines a literal crossed, without starting new lines of
    /// tokens until it ends.
    fn crossed(&mut self, from: usize) {
        let newlines = self.src[from..self.at].iter().filter(|&&c| c == b'\n');
        for _ in newlines {
            self.line += 1;
            self.lines.push(Line::default());
        }
        if let Some(last) = self.src[from..self.at].iter().rposition(|&c| c == b'\n') {
            self.line_start = from + last + 1;
        }
    }

    /// Whether a literal may start here, given what came before.
    fn operand_here(&self) -> bool {
        match self.before {
            Before::Operand => true,
            // `count /= 2` assigns.
            Before::Ident => {
                let assigns = self.peek(1) == b'=' && self.peek(2).is_ascii_whitespace();
                self.spaced && !self.peek(1).is_ascii_whitespace() && !assigns
            }
            Before::Value => false,
        }
    }

    /// Read code until the end, or until the `}` that closes an
    /// interpolation when `interpolated`.
    fn code(&mut self, interpolated: bool) {
        let mut depth = 0usize;
        while self.at < self.src.len() {
            let c = self.peek(0);
            if self.skipped(c) {
                continue;
            }
            if interpolated && c == b'}' && depth == 0 {
                self.at += 1;
                return;
            }
            match c {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.token(c);
            self.spaced = false;
        }
    }

    /// Skip what gives no token at `c`: a line end with the heredoc bodies
    /// it starts, whitespace, a block comment, the data after `__END__`.
    fn skipped(&mut self, c: u8) -> bool {
        let at_line_start = self.at == self.line_start;
        if c == b'\n' {
            self.newline();
            self.heredoc_bodies();
            self.spaced = true;
            // A line starts a statement: `/` there opens a regular expression.
            self.before = Before::Operand;
        } else if c.is_ascii_whitespace() || (c == b'\\' && self.peek(1) == b'\n') {
            self.at += 1;
            self.spaced = true;
        } else if at_line_start && self.src[self.at..].starts_with(b"=begin") {
            self.block_comment();
        } else if at_line_start && self.line_is(b"__END__") {
            self.at = self.src.len();
        } else {
            return false;
        }
        true
    }

    /// Whether the current line is `word` alone.
    fn line_is(&self, word: &[u8]) -> bool {
        let rest = &self.src[self.at..];
        let end = rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len());
        rest[..end].trim_ascii_end() == word
    }

    fn block_comment(&mut self) {
        let from = self.at;
        loop {
            let rest = &self.src[self.at..];
            let end = rest.iter().position(|&c| c == b'\n');
            let done = rest.starts_with(b"=end");
            match end {
                Some(end) if !done => self.at += end + 1,
                Some(end) => {
                    self.at += end;
                    break;
                }
                None => {
                    self.at = self.src.len();
                    break;
                }
            }
        }
        self.crossed(from);
    }

    /// One token starting at `c`, outside whitespace.
    fn token(&mut self, c: u8) {
        match c {
            b'#' => self.comment(),
            b'"' | b'`' => self.quoted(c, c, true),
            b'\'' => self.quoted(c, c, false),
            b':' => self.colon(),
            b'@' | b'$' => self.variable(c),
            b'0'..=b'9' => self.number(),
            b'/' if self.operand_here() => self.quoted(b'/', b'/', true),
            b'%' if self.operand_here() && self.percent() => {}
            b'<' if self.peek(1) == b'<' && self.heredoc() => {}
            b'?' if self.operand_here() && self.character() => {}
            c if is_word_start(c) => self.word(),
            c => self.punct(c),
        }
    }

    fn comment(&mut self) {
        let rest = &self.src[self.at..];
        self.at += rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len());
    }

    fn punct(&mut self, c: u8) {
        self.push(Tok::Punct(c));
        self.at += 1;
        self.before = match c {
            b')' | b']' | b'}' => Before::Value,
            _ => Before::Operand,
        };
    }

    /// Push `token` as starting at `from`, and go on from where it ends.
    fn push_at(&mut self, from: usize, token: Tok) {
        let end = self.at;
        self.at = from;
        self.push(token);
        self.at = end;
    }

    fn word(&mut self) {
        let from = self.at;
        while is_word(self.peek(0)) {
            self.at += 1;
        }
        let ends = matches!(self.peek(0), b'?' | b'!') && self.peek(1) != b'=';
        if ends && self.peek(1) != b':' {
            self.at += 1;
        }
        let word = self.text(from, self.at);
        self.before = match word.as_str() {
            w if LEADING.contains(&w) => Before::Operand,
            // A method may be named by an operator: `def /(other)`.
            "def" => Before::Value,
            _ => Before::Ident,
        };
        self.push_at(from, Tok::Word(word));
    }

    fn number(&mut self) {
        self.push(Tok::Value);
        while is_word(self.peek(0)) || (self.peek(0) == b'.' && self.peek(1).is_ascii_digit()) {
            self.at += 1;
        }
        self.before = Before::Value;
    }

    fn variable(&mut self, sigil: u8) {
        self.push(Tok::Value);
        self.at += 1;
        if sigil == b'@' && self.peek(0) == b'@' {
            self.at += 1;
        }
        if is_word_start(self.peek(0)) {
            while is_word(self.peek(0)) {
                self.at += 1;
            }
        } else if sigil == b'$' {
            // `$'`, `$"`, `$-w`: a special global, whatever it is spelled with.
            self.at += if self.peek(0) == b'-' { 2 } else { 1 };
        }
        self.before = Before::Value;
    }

    /// `::`, a symbol, or a colon.
    fn colon(&mut self) {
        match self.peek(1) {
            b':' => {
                self.push(Tok::Scope);
                self.at += 2;
                self.before = Before::Operand;
            }
            c if is_word_start(c) => self.named_symbol(),
            b'"' | b'\'' => {
                self.at += 1;
                let quote = self.peek(0);
                self.quoted(quote, quote, quote == b'"');
            }
            _ if self.operand_here() && self.operator_symbol() => {}
            _ => self.punct(b':'),
        }
    }

    /// `:name`, `:name?`, `:name=`.
    fn named_symbol(&mut self) {
        let from = self.at;
        self.at += 1;
        while is_word(self.peek(0)) {
            self.at += 1;
        }
        let marked = matches!(self.peek(0), b'?' | b'!' | b'=');
        if marked && !matches!(self.peek(1), b'=' | b'>' | b'~') {
            self.at += 1;
        }
        let name = self.text(from + 1, self.at);
        self.push_at(from, Tok::Symbol(name));
        self.before = Before::Value;
    }

    /// `:/`, `:<=>`, `:[]=`: a symbol that names an operator.
    fn operator_symbol(&mut self) -> bool {
        const OPERATORS: &[&[u8]] = &[
            b"[]=", b"<=>", b"===", b"[]", b"**", b"==", b"=~", b"!=", b"!~", b"<<", b">>", b"<=",
            b">=", b"+@", b"-@", b"+", b"-", b"*", b"/", b"%", b"<", b">", b"!", b"~", b"^", b"&",
            b"|",
        ];
        let rest = &self.src[self.at + 1..];
        let Some(operator) = OPERATORS.iter().find(|op| rest.starts_with(op)) else {
            return false;
        };
        let name = String::from_utf8_lossy(operator).into_owned();
        self.push(Tok::Symbol(name));
        self.at += 1 + operator.len();
        self.before = Before::Value;
        true
    }

    /// `?a`, `?\n`: a character, when what follows is not a word.
    fn character(&mut self) -> bool {
        let next = self.peek(1);
        if next.is_ascii_whitespace() || next == 0 {
            return false;
        }
        let len = if next == b'\\' { 3 } else { 2 };
        if is_word(self.peek(len)) {
            return false;
        }
        self.push(Tok::Str(String::new(), false));
        self.at += len;
        self.before = Before::Value;
        true
    }

    /// A percent literal (`%w[…]`, `%q(…)`, `%(…)`), if one starts here.
    fn percent(&mut self) -> bool {
        let (kind, open) = match (self.peek(1), self.peek(2)) {
            (k, o) if b"qQwWiIrsx".contains(&k) && !o.is_ascii_alphanumeric() && o > b' ' => (k, o),
            (o, _) if !o.is_ascii_alphanumeric() && o > b' ' && o != b'=' => (b'Q', o),
            _ => return false,
        };
        let interpolates = b"QWIrx".contains(&kind);
        self.push(Tok::Str(String::new(), false));
        self.at += if open == self.peek(1) { 1 } else { 2 };
        self.body(open, closing(open), interpolates);
        true
    }

    /// A literal opened by `open` at the current byte.
    fn quoted(&mut self, open: u8, close: u8, interpolates: bool) {
        self.push(Tok::Str(String::new(), false));
        self.body(open, close, interpolates);
    }

    /// The body of a literal whose opening byte is at `self.at`, its token
    /// already pushed; nested pairs of brackets balance.
    fn body(&mut self, open: u8, close: u8, interpolates: bool) {
        let (line, token) = (self.line, self.lines[self.line].tokens.len() - 1);
        self.at += 1;
        let mut text = Vec::new();
        let mut interpolated = false;
        let mut delimiters = Delimiters {
            open,
            close,
            depth: 0,
            class: false,
        };
        while self.at < self.src.len() {
            let c = self.peek(0);
            if c == b'\\' {
                text.push(self.peek(1));
                self.at += 2;
            } else if interpolates && c == b'#' && self.peek(1) == b'{' {
                interpolated = true;
                self.interpolation();
            } else if delimiters.closes(c) {
                self.at += 1;
                break;
            } else {
                if c == b'\n' {
                    self.crossed_byte();
                }
                text.push(c);
                self.at += 1;
            }
        }
        if open == b'/' {
            while self.peek(0).is_ascii_alphabetic() {
                self.at += 1;
            }
        }
        self.lines[line].tokens[token] =
            Tok::Str(String::from_utf8_lossy(&text).into(), interpolated);
        self.before = Before::Value;
    }

    /// The code of `#{…}`, read as code.
    fn interpolation(&mut self) {
        self.at += 2;
        let before = self.before;
        self.before = Before::Operand;
        self.code(true);
        self.before = before;
    }

    /// A newline inside a literal: a line with no tokens of its own.
    fn crossed_byte(&mut self) {
        self.line += 1;
        self.line_start = self.at + 1;
        self.lines.push(Line::default());
    }

    /// A heredoc (`<<~ID`, `<<-'ID'`, `<<ID`), if one starts here. Its body
    /// is read when the line ends.
    fn heredoc(&mut self) -> bool {
        let Some(opener) = self.heredoc_opener() else {
            return false;
        };
        let allowed = if opener.indented {
            self.before != Before::Value || self.spaced
        } else {
            (opener.quote.is_some() || opener.id.starts_with(|c: char| c.is_ascii_uppercase()))
                && self.operand_here()
        };
        if !allowed {
            return false;
        }
        self.push(Tok::Str(String::new(), opener.quote != Some(b'\'')));
        self.heredocs.push(Heredoc {
            id: opener.id,
            indented: opener.indented,
            line: self.line,
            token: self.lines[self.line].tokens.len() - 1,
        });
        self.at += opener.len;
        self.before = Before::Value;
        true
    }

    /// What follows `<<` here, if it can open a heredoc.
    fn heredoc_opener(&self) -> Option<Opener> {
        let indented = matches!(self.peek(2), b'~' | b'-');
        let at = 2 + usize::from(indented);
        let quote = Some(self.peek(at)).filter(|q| matches!(q, b'"' | b'\'' | b'`'));
        let start = at + usize::from(quote.is_some());
        let end = (start..).find(|&i| !is_word(self.peek(i)))?;
        let closed = quote.is_none_or(|q| self.peek(end) == q);
        if end == start || !closed {
            return None;
        }
        Some(Opener {
            id: self.text(self.at + start, self.at + end),
            indented,
            quote,
            len: end + usize::from(quote.is_some()),
        })
    }

    /// The bodies of the heredocs opened on the line just ended.
    fn heredoc_bodies(&mut self) {
        for heredoc in std::mem::take(&mut self.heredocs) {
            let from = self.at;
            let body = self.heredoc_body(&heredoc);
            let token = &mut self.lines[heredoc.line].tokens[heredoc.token];
            let interpolated = matches!(token, Tok::Str(_, true)) && body.contains("#{");
            *token = Tok::Str(body, interpolated);
            self.crossed(from);
            self.line_start = self.at;
        }
    }

    /// The lines of `heredoc` up to its terminator, which is skipped.
    fn heredoc_body(&mut self, heredoc: &Heredoc) -> String {
        let mut body = String::new();
        while self.at < self.src.len() {
            let rest = &self.src[self.at..];
            let len = rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len());
            let text = &rest[..len];
            let text = text.strip_suffix(b"\r").unwrap_or(text);
            let ends = heredoc.ends_at(text);
            let line = String::from_utf8_lossy(text).into_owned();
            self.at = (self.at + len + 1).min(self.src.len());
            if ends {
                break;
            }
            body.push_str(&line);
            body.push('\n');
        }
        body
    }
}

impl Heredoc {
    /// Whether the line `text` ends the heredoc.
    fn ends_at(&self, text: &[u8]) -> bool {
        let text = if self.indented {
            text.trim_ascii()
        } else {
            text
        };
        text == self.id.as_bytes()
    }
}

#[cfg(test)]
#[path = "ruby_literals_test.rs"]
mod tests;
