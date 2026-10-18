//! C# extension methods, read lexically. A static class declares one with
//! `this` before its first parameter, and a file calls it on a value
//! (`t.IsValueType()`) without naming the class. A call reaches the file
//! that declares a method of that name, taking as many arguments, in a
//! namespace the caller sees. The receiver's type is not known: when
//! several files declare such a method, the call is ambiguous and reaches
//! none of them.

use std::collections::{BTreeSet, HashMap};
use std::ops::Range;
use std::path::{Path, PathBuf};

use super::c_like::Token;
use super::types::nearest_all;

/// An extension method as its declaration gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct Extension {
    pub namespace: String,
    pub method: String,
    /// How many arguments a call passes, besides the receiver.
    pub arity: Range<usize>,
}

/// A method called after a dot, with the number of its arguments.
pub type Call = (String, usize);

/// What a file declares and calls that may be extension methods.
#[derive(Debug, Default, PartialEq)]
pub struct Members {
    /// The extension methods it declares.
    pub declared: Vec<Extension>,
    /// The methods it calls after a dot.
    pub called: BTreeSet<Call>,
}

impl Members {
    /// Note what the `(` at `open` opens; `namespace` is the one around
    /// when it is in the body of a top-level type, where extension methods
    /// are declared.
    pub fn read(&mut self, tokens: &[Token], open: usize, namespace: Option<String>) {
        match (parenthesis(tokens, open), namespace) {
            (Some(Parenthesis::Call(call)), _) => {
                self.called.insert(call);
            }
            (Some(Parenthesis::Extension(method, arity)), Some(namespace)) => {
                self.declared.push(Extension {
                    namespace,
                    method,
                    arity,
                });
            }
            _ => {}
        }
    }
}

/// What a `(` after a word opens.
#[derive(Debug, PartialEq)]
pub enum Parenthesis {
    /// The arguments of a call after a dot.
    Call(Call),
    /// The parameters of an extension method, with its arity.
    Extension(String, Range<usize>),
}

/// What the `(` at `open` opens, when it is a call after a dot or the
/// parameters of an extension method others can call.
pub fn parenthesis(tokens: &[Token], open: usize) -> Option<Parenthesis> {
    let name = called_at(tokens, open)?;
    let Token::Word(method) = tokens[name] else {
        return None;
    };
    if name > 0 && tokens[name - 1] == Token::Punct(b'.') {
        let count = call_arguments(tokens, open)?;
        return Some(Parenthesis::Call((method.to_string(), count)));
    }
    let arity = extension_arity(tokens, open, name)?;
    Some(Parenthesis::Extension(method.to_string(), arity))
}

/// The word a `(` at `open` follows, past type arguments (`Cast<T>(`).
pub fn called_at(tokens: &[Token], open: usize) -> Option<usize> {
    let before = open.checked_sub(1)?;
    let name = match tokens[before] {
        Token::Word(_) => before,
        Token::Punct(b'>') => type_arguments_start(tokens, before)?.checked_sub(1)?,
        _ => return None,
    };
    matches!(tokens[name], Token::Word(_)).then_some(name)
}

/// The `<` that opens the type arguments closed by the `>` at `close`.
fn type_arguments_start(tokens: &[Token], close: usize) -> Option<usize> {
    let mut depth = 0;
    for at in (0..=close).rev() {
        match tokens[at] {
            Token::Word(_) | Token::Punct(b'.' | b',' | b'[' | b']' | b'?') => {}
            Token::Punct(b'>') => depth += 1,
            Token::Punct(b'<') if depth == 1 => return Some(at),
            Token::Punct(b'<') => depth -= 1,
            Token::Punct(_) => return None,
        }
    }
    None
}

/// How a token changes the nesting of brackets. Angle brackets nest only
/// in a declaration, where `<` is never a comparison.
fn nesting(token: &Token, angles: bool) -> i32 {
    match token {
        Token::Punct(b'(' | b'[' | b'{') => 1,
        Token::Punct(b')' | b']' | b'}') => -1,
        Token::Punct(b'<') if angles => 1,
        Token::Punct(b'>') if angles => -1,
        _ => 0,
    }
}

/// The tokens of each argument or parameter in the parentheses opened at
/// `open`, and where they close.
fn arguments<'a>(
    tokens: &'a [Token<'a>],
    open: usize,
    angles: bool,
) -> (Vec<&'a [Token<'a>]>, usize) {
    let mut depth = 0;
    let mut cuts = vec![open];
    let mut close = tokens.len();
    for (at, token) in tokens.iter().enumerate().skip(open) {
        depth += nesting(token, angles);
        if depth == 0 {
            close = at;
            break;
        }
        if depth == 1 && *token == Token::Punct(b',') {
            cuts.push(at);
        }
    }
    if close == open + 1 {
        return (Vec::new(), close);
    }
    cuts.push(close);
    let parts = cuts.windows(2).map(|w| &tokens[w[0] + 1..w[1]]).collect();
    (parts, close)
}

/// How many arguments a call that opens at `open` passes. `None` when a
/// body follows the parentheses: then they declare (`IList.IndexOf(T x)
/// {`) or construct with an initializer.
pub fn call_arguments(tokens: &[Token], open: usize) -> Option<usize> {
    let (parts, close) = arguments(tokens, open, false);
    let body = matches!(
        tokens.get(close + 1..),
        Some([Token::Punct(b'{'), ..] | [Token::Punct(b'='), Token::Punct(b'>'), ..])
    );
    (!body).then_some(parts.len())
}

/// The arguments an extension method takes, when the word at `name`,
/// followed by the `(` at `open`, declares one others can call: `this
/// Type` opens its parameters and it is not private.
pub fn extension_arity(tokens: &[Token], open: usize, name: usize) -> Option<Range<usize>> {
    let this = matches!(
        tokens.get(open + 1..open + 3),
        Some([Token::Word("this"), Token::Word(_)])
    );
    let mut modifiers = tokens[..name]
        .iter()
        .rev()
        .take_while(|t| !matches!(t, Token::Punct(b';' | b'{' | b'}' | b']')));
    if !this || modifiers.any(|t| *t == Token::Word("private")) {
        return None;
    }
    let (parameters, _) = arguments(tokens, open, true);
    let others = parameters.get(1..).unwrap_or_default();
    let variadic = |p: &&[Token]| p.contains(&Token::Word("params"));
    let optional = |p: &&[Token]| p.contains(&Token::Punct(b'=')) || variadic(p);
    let required = others.iter().filter(|p| !optional(p)).count();
    let most = match others.iter().any(variadic) {
        true => usize::MAX - 1,
        false => others.len(),
    };
    Some(required..most + 1)
}

/// The files that declare each extension method, by name.
pub struct Extensions(HashMap<String, Vec<(Extension, usize)>>);

impl Extensions {
    /// The extension methods the files declare, by their index; `None`
    /// for a file of another language.
    pub fn of<'a>(files: impl Iterator<Item = Option<&'a Members>>) -> Self {
        let mut by_name: HashMap<String, Vec<(Extension, usize)>> = HashMap::new();
        for (file, members) in files.enumerate() {
            for extension in members.iter().flat_map(|m| &m.declared) {
                let declared = by_name.entry(extension.method.clone()).or_default();
                declared.push((extension.clone(), file));
            }
        }
        Extensions(by_name)
    }

    /// The files that declare a method `call` may be, in one of the
    /// namespaces `seen`.
    pub fn candidates(&self, (method, count): &Call, seen: &[String]) -> Vec<usize> {
        let declared = self.0.get(method).map_or(&[][..], Vec::as_slice);
        let mut files: Vec<usize> = declared
            .iter()
            .filter(|(e, _)| e.arity.contains(count) && seen.contains(&e.namespace))
            .map(|&(_, file)| file)
            .collect();
        files.sort_unstable();
        files.dedup();
        files
    }

    /// The file that declares the extension method `call` is to, when one
    /// alone does among those nearest to `from`.
    pub fn called(
        &self,
        call: &Call,
        seen: &[String],
        from: &Path,
        paths: &[PathBuf],
    ) -> Option<usize> {
        match nearest_all(&self.candidates(call, seen), from, paths)[..] {
            [file] => Some(file),
            _ => None,
        }
    }

    /// The files whose extension methods `members` calls.
    pub fn used_by(
        &self,
        members: &Members,
        seen: &[String],
        from: &Path,
        paths: &[PathBuf],
    ) -> Vec<usize> {
        let called = members.called.iter();
        called
            .filter_map(|call| self.called(call, seen, from, paths))
            .collect()
    }
}

#[cfg(test)]
#[path = "csharp_extensions_test.rs"]
mod tests;
