//! TypeScript declarations that produce no runtime code: `interface`,
//! `type` aliases, `import type`, `export type { … }` and ambient `declare`
//! blocks. They describe values, so Halstead leaves them out: a file of
//! types alone has no operators or operands to measure.

/// Which of `lines` belong to a type-level declaration.
pub fn type_declaration_mask(lines: &[&str]) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    let mut i = 0;
    while i < lines.len() {
        if !starts_declaration(lines[i]) {
            i += 1;
            continue;
        }
        let end = declaration_end(lines, i);
        mask[i..=end].fill(true);
        i = end + 1;
    }
    mask
}

/// Whether a line opens a type-level declaration.
fn starts_declaration(line: &str) -> bool {
    let line = line.trim_start();
    let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
    if line.starts_with("import type ") || line.starts_with("type {") {
        return true;
    }
    // An ambient declaration (`declare const`, `declare module "x" { … }`)
    // only describes what exists elsewhere.
    if line.starts_with("declare ") {
        return true;
    }
    let is_alias = line
        .strip_prefix("type ")
        .is_some_and(|rest| rest.starts_with(|c: char| c.is_alphabetic() || c == '_'));
    is_alias || line.starts_with("interface ")
}

/// The last line of the declaration that starts at `from`: where its
/// brackets balance and nothing continues it.
fn declaration_end(lines: &[&str], from: usize) -> usize {
    let mut depth = 0isize;
    for (at, line) in lines.iter().enumerate().skip(from) {
        depth += bracket_balance(line);
        let next_continues = lines
            .get(at + 1)
            .is_some_and(|next| next.trim_start().starts_with(['|', '&', '?', ':', '.']));
        if depth <= 0 && !ends_open(line) && !next_continues {
            return at;
        }
    }
    lines.len() - 1
}

/// Opening minus closing brackets of a line, `{[(<` against `}])>`; the
/// `>` of an arrow is no bracket.
fn bracket_balance(line: &str) -> isize {
    let bytes = line.as_bytes();
    (0..bytes.len())
        .map(|i| match bytes[i] {
            b'{' | b'[' | b'(' | b'<' => 1,
            b'}' | b']' | b')' => -1,
            b'>' if i == 0 || bytes[i - 1] != b'=' => -1,
            _ => 0,
        })
        .sum()
}

/// Whether a line leaves its declaration to the next: it ends in an
/// operator or an opening bracket.
fn ends_open(line: &str) -> bool {
    line.trim_end()
        .ends_with(['=', '|', '&', ',', '<', '(', '{', '[', '?', ':'])
}

#[cfg(test)]
#[path = "type_decls_test.rs"]
mod tests;
