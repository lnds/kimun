//! Reading the tokens of Java and C# one construct at a time: the chains
//! of names they write, and the walk both readers share.

use super::c_like::Token;

/// The chain of names that starts at `tokens[at]`, and where it ends: the
/// words joined by dots. A `.*` that closes it is told apart.
pub fn chain_at(tokens: &[Token], at: usize) -> (Vec<String>, usize, bool) {
    let mut names = Vec::new();
    let mut i = at;
    while let Some(Token::Word(word)) = tokens.get(i) {
        names.push(word.to_string());
        if tokens.get(i + 1) != Some(&Token::Punct(b'.')) {
            return (names, i + 1, false);
        }
        if tokens.get(i + 2) == Some(&Token::Punct(b'*')) {
            return (names, i + 3, true);
        }
        i += 2;
    }
    (names, i, false)
}

/// Where a reader is among the tokens of a source.
#[derive(Clone, Copy)]
pub struct Cursor<'t> {
    pub tokens: &'t [Token<'t>],
    pub at: usize,
}

/// A reader of the tokens of a source, one construct at a time. What is
/// read outside every type (a package, an import, a declaration) differs
/// by language; a chain of names is read anywhere, but never from a word
/// that follows a dot, which is the rest of a chain already read.
pub trait Reader<'t> {
    fn cursor(&mut self) -> &mut Cursor<'t>;
    /// Note a punctuation mark: a brace opens or closes something.
    fn punct(&mut self, mark: u8);
    /// Whether the reader is outside every type and body.
    fn outside(&self) -> bool;
    /// Read what a word starts outside every type; where the next starts.
    fn top_level(&mut self, word: &'t str) -> usize;
    /// Read the chain of names that starts here; where the next starts.
    fn name(&mut self) -> usize;

    /// Read what starts at the current token; where the next starts.
    fn step(&mut self) -> usize {
        let Cursor { tokens, at } = *self.cursor();
        let after_dot = at > 0 && tokens[at - 1] == Token::Punct(b'.');
        match tokens[at] {
            Token::Punct(mark) => {
                self.punct(mark);
                at + 1
            }
            Token::Word(_) if after_dot => at + 1,
            Token::Word(word) if self.outside() => self.top_level(word),
            Token::Word(_) => self.name(),
        }
    }

    /// Read every token.
    fn read_all(&mut self) {
        while self.cursor().at < self.cursor().tokens.len() {
            let next = self.step();
            self.cursor().at = next;
        }
    }
}
