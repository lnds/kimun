//! What JavaScript and TypeScript write: functions bound to a name
//! (`f = function`, `f = (x) => {`) and regular expression literals.

use super::*;

/// The name a JavaScript or TypeScript function is assigned to or keyed
/// by, when the signature is one: `name = function`, `name = (x) => {`,
/// `name: Type = <T>(x) => {`, `name: function`, `name: async (x) => {`.
pub(super) fn script_binding(sig: &str) -> Option<String> {
    let paren = sig.find('(').unwrap_or(sig.len());
    let at = plain_assign(sig).or_else(|| sig.find(':').filter(|&c| c < paren))?;
    let value = sig[at + 1..].trim_start();
    let value = value.strip_prefix("async").map_or(value, str::trim_start);
    let arrow = value.contains("=>");
    let is_function = value.starts_with("function")
        || (arrow && sig.ends_with('{') && value.starts_with(['(', '<', '_']))
        || (arrow && is_identifier(value.split("=>").next().unwrap_or("").trim()));
    if !is_function {
        return None;
    }
    let target = sig[..at].split(':').next().unwrap_or("").trim();
    let name = target
        .split_whitespace()
        .next_back()?
        .trim_end_matches(['?', '!']);
    is_identifier(name.trim_start_matches('#').replace('.', "").as_str()).then(|| name.to_string())
}

/// Where the first plain `=` of a signature is, outside parentheses,
/// brackets, braces and type arguments, and before the arrow or the brace
/// of a body: what assigns a value to a name.
pub(super) fn plain_assign(sig: &str) -> Option<usize> {
    let bytes = sig.as_bytes();
    let mut depth = 0;
    for i in 0..bytes.len() {
        if depth == 0 && ends_binding(bytes, i) {
            return None;
        }
        if depth == 0 && bytes[i] == b'=' && is_plain_assign(bytes, i) {
            return Some(i);
        }
        depth += nesting(bytes, i);
    }
    None
}

/// Whether what binds a name ends at `i`: an arrow, or the brace of a body.
fn ends_binding(bytes: &[u8], i: usize) -> bool {
    bytes[i] == b'{' || (bytes[i] == b'=' && bytes.get(i + 1) == Some(&b'>'))
}

/// How the byte at `i` changes the depth of parentheses, brackets, braces
/// and type arguments; the `>` of an arrow changes nothing.
fn nesting(bytes: &[u8], i: usize) -> isize {
    match bytes[i] {
        b'(' | b'[' | b'{' | b'<' => 1,
        b')' | b']' | b'}' => -1,
        b'>' if i > 0 && bytes[i - 1] != b'=' => -1,
        _ => 0,
    }
}

pub(super) fn is_identifier(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

/// The name of a JavaScript or TypeScript function: after `function`,
/// else the name it is bound to, else the method name before `(`. A
/// function passed as an argument has none.
pub(super) fn script_name(sig: &str) -> String {
    let named = sig
        .find("function")
        .map(|at| sig[at + "function".len()..].trim_start_matches(['*', ' ']));
    let after_keyword = named.and_then(|rest| {
        let name: String = rest
            .chars()
            .take_while(|c| is_identifier(&c.to_string()))
            .collect();
        (!name.is_empty()).then_some(name)
    });
    after_keyword
        .or_else(|| script_binding(sig))
        .or_else(|| named.is_none().then(|| name_before_paren(sig)).flatten())
        .unwrap_or_else(|| "<anonymous>".to_string())
}

/// Blank the contents of each regular expression literal: a `/` where a
/// value may start (after an operator, an opening bracket, a keyword such
/// as `return`, or at the start of the line) up to the `/` that closes it,
/// outside any character class.
pub(super) fn without_regex_literals(line: &str) -> String {
    let mut bytes = line.as_bytes().to_vec();
    let mut i = 0;
    while i < bytes.len() {
        let opens = bytes[i] == b'/'
            && !matches!(bytes.get(i + 1), Some(b'/' | b'*'))
            && value_may_start(&bytes[..i]);
        match opens.then(|| regex_end(&bytes, i + 1)).flatten() {
            Some(end) => {
                bytes[i + 1..end].fill(b' ');
                i = end + 1;
            }
            None => i += 1,
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| line.to_string())
}

/// Whether a value, rather than an operand of a division, may start after
/// `before`.
pub(super) fn value_may_start(before: &[u8]) -> bool {
    let text = String::from_utf8_lossy(before);
    let text = text.trim_end();
    match text.bytes().last() {
        None => true,
        Some(b) if b"(,=:[!&|?{};+-*%<>~^".contains(&b) => true,
        Some(_) => ["return", "typeof", "case", "in", "of"].iter().any(|kw| {
            text.ends_with(kw)
                && !text[..text.len() - kw.len()]
                    .ends_with(|c: char| c.is_alphanumeric() || c == '_')
        }),
    }
}

/// Where the regular expression whose body starts at `i` closes.
pub(super) fn regex_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut class = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'[' => class = true,
            b']' => class = false,
            b'/' if !class => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}
