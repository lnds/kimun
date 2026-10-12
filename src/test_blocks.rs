//! Inline test blocks of Rust: the items under `#[cfg(test)]`.
//!
//! An analysis that leaves tests out skips those items and nothing else:
//! the code that follows a test module is production code.

use crate::deps::rust_literals::code_and_strings;
use crate::loc::counter::LineKind;

const ATTRIBUTE: &str = "#[cfg(test)]";

/// Which lines belong to an item under `#[cfg(test)]`, the attribute
/// included. Comments and literals are read apart, so an attribute or a
/// brace inside one counts for nothing.
pub fn test_block_mask(lines: &[String]) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    if !lines.iter().any(|line| line.contains(ATTRIBUTE)) {
        return mask;
    }
    let (code, _) = code_and_strings(&lines.join("\n"));
    let code: Vec<&str> = code.split('\n').collect();
    let mut i = 0;
    while i < code.len() {
        if !code[i].trim_start().starts_with(ATTRIBUTE) {
            i += 1;
            continue;
        }
        let end = item_end(&code, i);
        mask[i..=end].fill(true);
        i = end + 1;
    }
    mask
}

/// The last line of the item that starts at `from`: where its braces
/// balance, or where it ends without any (`mod tests;`, a `use`, a field).
fn item_end(code: &[&str], from: usize) -> usize {
    let mut braces = 0usize;
    let mut parens = 0usize;
    let mut where_clause = false;
    for (offset, line) in code[from..].iter().enumerate() {
        let at = from + offset;
        for ch in line.chars() {
            match ch {
                '(' | '[' => parens += 1,
                ')' | ']' => parens = parens.saturating_sub(1),
                '{' => braces += 1,
                // The block around the item closes: it ended a line before.
                '}' if braces == 0 => return at.saturating_sub(1).max(from),
                '}' => {
                    braces -= 1;
                    if braces == 0 {
                        return at;
                    }
                }
                _ => {}
            }
        }
        where_clause |= line.trim_start().starts_with("where");
        if braces == 0 && parens == 0 && closes_item(line, where_clause) {
            return at;
        }
    }
    code.len() - 1
}

/// Whether a line outside braces and parentheses ends an item: with `;`,
/// or with the `,` of a field or variant. The bounds of a `where` clause
/// end in commas too, and the body comes after them.
fn closes_item(line: &str, where_clause: bool) -> bool {
    let line = line.trim_end();
    line.ends_with(';') || (line.ends_with(',') && !where_clause)
}

/// `kinds` with the lines of inline test blocks turned into blanks, so
/// that an analysis of code lines skips them.
pub fn without_test_blocks(lines: &[String], kinds: &[LineKind]) -> Vec<LineKind> {
    let mask = test_block_mask(lines);
    let demote =
        |(kind, in_test): (&LineKind, &bool)| if *in_test { LineKind::Blank } else { *kind };
    kinds.iter().zip(&mask).map(demote).collect()
}

#[cfg(test)]
#[path = "test_blocks_test.rs"]
mod tests;
