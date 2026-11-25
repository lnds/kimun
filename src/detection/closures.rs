//! Closures without parameters, in a language whose closures `|` heads
//! (Rust, `FunctionSyntax::closure_bars`): a `||` where an operand would
//! start opens one, and is no boolean operator.

use super::FunctionSyntax;

/// Whether the `||` at byte `i` heads a closure in a language of `syntax`,
/// given whether the line before ended with an operand.
pub(crate) fn opens_closure(
    syntax: FunctionSyntax,
    bytes: &[u8],
    i: usize,
    after_operand: bool,
) -> bool {
    let before = bytes[..i].trim_ascii_end();
    let operand_before = before.is_empty() && after_operand || ends_with_operand(before);
    syntax.closure_bars() && bytes[i..].starts_with(b"||") && !operand_before
}

/// How many `||` of a line head a closure, read as `str::matches` finds
/// them.
pub(crate) fn count(syntax: FunctionSyntax, line: &str, after_operand: bool) -> usize {
    let bytes = line.as_bytes();
    let heads = |&(at, _): &(usize, &str)| opens_closure(syntax, bytes, at, after_operand);
    line.match_indices("||").filter(heads).count()
}

/// Whether `bytes` end with an operand: a name or a literal (not `move` or
/// `return`), a closing bracket or `?`.
pub(crate) fn ends_with_operand(bytes: &[u8]) -> bool {
    let word_byte = |b: &u8| b.is_ascii_alphanumeric() || *b == b'_';
    let word_start = bytes
        .iter()
        .rposition(|b| !word_byte(b))
        .map_or(0, |p| p + 1);
    match &bytes[word_start..] {
        b"" => bytes.last().is_some_and(|b| b")]?\"'".contains(b)),
        b"move" | b"return" => false,
        _ => true,
    }
}

#[cfg(test)]
#[path = "closures_test.rs"]
mod tests;
