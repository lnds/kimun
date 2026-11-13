//! What reads as a branch in a line of code, the same for cyclomatic and
//! cognitive complexity.

/// Ternary operators of a line: a `?` with a space on each side, other
/// than a wildcard of Java generics (`<?`, `, ? extends T`).
pub fn count_ternaries(stripped: &str) -> usize {
    stripped
        .match_indices(" ? ")
        .filter(|(at, _)| {
            let after = stripped[at + 3..].trim_start();
            !(after.starts_with(['>', ','])
                || after.starts_with("extends")
                || after.starts_with("super"))
        })
        .count()
}

/// Whether the word of `len` bytes at `at` names something rather than
/// being a keyword: a method after a single `.` (`p.catch(f)`), or with
/// `keys` a symbol (`:if`) or a key (`if:`, but not `if::`).
pub fn names_something(line: &[u8], at: usize, len: usize, keys: bool) -> bool {
    let (before, after) = (&line[..at], &line[at + len..]);
    let ends_alone = |c: u8| before.ends_with(&[c]) && !before.ends_with(&[c, c]);
    let key = after.starts_with(b":") && !after.starts_with(b"::");
    ends_alone(b'.') || (keys && (ends_alone(b':') || key))
}

/// Brackets a line opens minus those it closes.
pub fn bracket_balance(line: &str) -> isize {
    let count = |set: &str| line.chars().filter(|c| set.contains(*c)).count() as isize;
    count("([{") - count(")]}")
}

#[cfg(test)]
#[path = "branches_test.rs"]
mod tests;
