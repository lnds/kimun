//! The package clause and the imports of a Go source, read from its tokens
//! so that comments and literals hide what they hold: a package comment
//! may show an example program, `import` line included.
//!
//! `import` is a keyword, so wherever it is a token it starts an import
//! declaration: one path, or a group of them in parentheses, each with an
//! optional name (`alias`, `.` or `_`) before it.

use super::c_like::{find, line_len, quoted_len};

/// A token of Go: a word, a string literal by its content, or a mark.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Token<'s> {
    Word(&'s str),
    Str(&'s str),
    Punct(u8),
}

/// What a Go source declares about its packages.
#[derive(Debug, Default, PartialEq)]
pub struct GoFile {
    /// The name of its package clause.
    pub package: Option<String>,
    /// The import paths, as written.
    pub imports: Vec<String>,
}

impl GoFile {
    /// Whether the file is an external test: a package of its own beside
    /// the one it tests, which it may import.
    pub fn is_external_test(&self) -> bool {
        self.package.as_ref().is_some_and(|p| p.ends_with("_test"))
    }
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || !c.is_ascii()
}

/// What a stretch of source is to the reader.
enum Lexeme {
    /// Whitespace or a comment.
    Skip,
    /// A string or rune literal, quotes included.
    Literal,
    Code,
}

/// The lexeme that starts `rest`, and its length.
fn lexeme(rest: &[u8]) -> (usize, Lexeme) {
    let closed = |end: Option<usize>, extra: usize| end.map_or(rest.len(), |end| end + extra);
    match rest {
        [c, ..] if c.is_ascii_whitespace() => (1, Lexeme::Skip),
        [b'/', b'/', ..] => (line_len(rest), Lexeme::Skip),
        [b'/', b'*', ..] => (closed(find(rest, b"*/"), 2), Lexeme::Skip),
        // A raw string has no escapes and may span lines.
        [b'`', ..] => (closed(find(&rest[1..], b"`"), 2), Lexeme::Literal),
        [quote @ (b'"' | b'\''), ..] => (quoted_len(rest, *quote), Lexeme::Literal),
        [c, ..] if is_word(*c) => (
            rest.iter().take_while(|&&c| is_word(c)).count(),
            Lexeme::Code,
        ),
        _ => (1, Lexeme::Code),
    }
}

/// The tokens of `source`, without comments.
fn tokens(source: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < source.len() {
        let (len, kind) = lexeme(&source.as_bytes()[at..]);
        let text = source.get(at..at + len).unwrap_or("");
        let first = source.as_bytes()[at];
        let token = match kind {
            Lexeme::Skip => None,
            // A rune is no path.
            Lexeme::Literal if first == b'\'' => None,
            Lexeme::Literal => {
                let inner = text.get(1..).unwrap_or("");
                Some(Token::Str(
                    inner.strip_suffix(first as char).unwrap_or(inner),
                ))
            }
            Lexeme::Code if is_word(first) => Some(Token::Word(text)),
            Lexeme::Code => Some(Token::Punct(first)),
        };
        tokens.extend(token);
        at += len.max(1);
    }
    tokens
}

/// Read the package clause and the imports of a Go source.
pub fn read(source: &str) -> GoFile {
    let tokens = tokens(source);
    let mut file = GoFile::default();
    let mut at = 0;
    while at < tokens.len() {
        match (tokens[at], tokens.get(at + 1)) {
            (Token::Word("package"), Some(Token::Word(name))) if file.package.is_none() => {
                file.package = Some(name.to_string());
            }
            (Token::Word("import"), Some(Token::Punct(b'('))) => {
                at += 2;
                while let Some(&token) = tokens.get(at) {
                    match token {
                        Token::Punct(b')') => break,
                        Token::Str(path) => file.imports.push(path.to_string()),
                        _ => {}
                    }
                    at += 1;
                }
            }
            (Token::Word("import"), _) => {
                let path = tokens[at + 1..].iter().take(2).find_map(|t| match t {
                    Token::Str(path) => Some(path),
                    _ => None,
                });
                file.imports.extend(path.map(|p| p.to_string()));
            }
            _ => {}
        }
        at += 1;
    }
    file.imports.retain(|path| !path.is_empty());
    file
}

#[cfg(test)]
#[path = "golang_source_test.rs"]
mod tests;
