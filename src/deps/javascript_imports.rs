//! The import forms of JavaScript and TypeScript over the tokens of a
//! source, each with how it loads its module: a type-only import or
//! re-export never does, `import()` does when it runs, the rest with the
//! importer.

use super::graph::EdgeKind;
use super::javascript::Token;

/// A specifier, how it loads, and the token after the form.
type Form = (String, EdgeKind, usize);

/// The specifiers of the import forms among `tokens`, in order.
pub fn read(tokens: &[Token]) -> Vec<(String, EdgeKind)> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        at = match form(tokens, at) {
            Some((specifier, kind, next)) => {
                found.push((specifier, kind));
                next
            }
            None => at + 1,
        };
    }
    found
}

/// The import form that starts at `at`, if one does. A word after a dot is
/// a property, not a keyword.
fn form(tokens: &[Token], at: usize) -> Option<Form> {
    if at > 0 && matches!(tokens[at - 1], Token::Punct(b'.')) {
        return None;
    }
    match tokens[at] {
        Token::Word("import") => import(tokens, at + 1),
        Token::Word("export") => reexport(tokens, at + 1),
        Token::Word("require") => call(tokens, at + 1, EdgeKind::Load),
        Token::Word("from") => specifier(tokens, at + 1).map(|s| (s, EdgeKind::Load, at + 2)),
        _ => None,
    }
}

/// The plain text of the string at `at`, if a string is there.
fn specifier(tokens: &[Token], at: usize) -> Option<String> {
    match tokens.get(at)? {
        Token::Str(Some(text)) => Some(text.clone()),
        _ => None,
    }
}

fn is_punct(tokens: &[Token], at: usize, c: u8) -> bool {
    matches!(tokens.get(at), Some(Token::Punct(p)) if *p == c)
}

/// `('…')` at `at`.
fn call(tokens: &[Token], at: usize, kind: EdgeKind) -> Option<Form> {
    if !is_punct(tokens, at, b'(') {
        return None;
    }
    specifier(tokens, at + 1).map(|s| (s, kind, at + 2))
}

/// What follows `import`: a call, a bare specifier, or bindings and `from`.
fn import(tokens: &[Token], at: usize) -> Option<Form> {
    if is_punct(tokens, at, b'(') {
        return call(tokens, at, EdgeKind::Deferred);
    }
    if let Some(text) = specifier(tokens, at) {
        return Some((text, EdgeKind::Load, at + 1));
    }
    // `type` is a modifier unless it is the name bound: `import type from`,
    // `import type, { a } from`.
    let modifier = matches!(tokens.get(at), Some(Token::Word("type")))
        && match tokens.get(at + 1) {
            Some(Token::Word("from")) => !matches!(tokens.get(at + 2), Some(Token::Str(_))),
            Some(Token::Word(_)) => true,
            _ => is_punct(tokens, at + 1, b'{') || is_punct(tokens, at + 1, b'*'),
        };
    clause(tokens, at + usize::from(modifier), modifier)
}

/// What follows `export`, when it re-exports from another module:
/// `export { … } from`, `export * from`, either with `type`.
fn reexport(tokens: &[Token], at: usize) -> Option<Form> {
    let list = |at| is_punct(tokens, at, b'{') || is_punct(tokens, at, b'*');
    let modifier = matches!(tokens.get(at), Some(Token::Word("type"))) && list(at + 1);
    let at = at + usize::from(modifier);
    list(at).then(|| clause(tokens, at, modifier)).flatten()
}

/// The bindings of an import or a re-export, up to its `from` and
/// specifier. It takes only types when `modifier` says so, or when it
/// binds nothing but names marked `type`.
fn clause(tokens: &[Token], mut at: usize, modifier: bool) -> Option<Form> {
    let mut value = false;
    let mut typed = 0;
    loop {
        match tokens.get(at)? {
            Token::Word("from") => {
                let text = specifier(tokens, at + 1)?;
                let types = modifier || (!value && typed > 0);
                let kind = if types {
                    EdgeKind::Type
                } else {
                    EdgeKind::Load
                };
                return Some((text, kind, at + 2));
            }
            Token::Punct(b'{') => {
                let (values, types, next) = braces(tokens, at + 1)?;
                value |= values;
                typed += types;
                at = next;
            }
            Token::Word("import" | "export" | "require") => return None,
            Token::Word("as") | Token::Punct(b',') => at += 1,
            Token::Word(_) | Token::Punct(b'*') => {
                value = true;
                at += 1;
            }
            _ => return None,
        }
    }
}

/// The names of a `{ … }` list that starts at `at`: whether one is a
/// value, how many are marked `type`, and the token after the list.
fn braces(tokens: &[Token], at: usize) -> Option<(bool, usize, usize)> {
    let rest = tokens.get(at..)?;
    let close = at + rest.iter().position(|t| matches!(t, Token::Punct(b'}')))?;
    let mut names: Vec<Vec<&str>> = vec![Vec::new()];
    for token in &tokens[at..close] {
        match token {
            Token::Punct(b',') => names.push(Vec::new()),
            Token::Word(word) => names.last_mut()?.push(word),
            Token::Str(_) => names.last_mut()?.push(""),
            _ => return None,
        }
    }
    names.retain(|name| !name.is_empty());
    let typed = names.iter().filter(|name| is_marked(name)).count();
    Some((typed < names.len(), typed, close + 1))
}

/// Whether a name of a list is marked `type`: `type a` is, while
/// `type as b` renames a name `type`.
fn is_marked(name: &[&str]) -> bool {
    matches!(name, ["type", second, ..] if *second != "as")
}

#[cfg(test)]
#[path = "javascript_imports_test.rs"]
mod tests;
