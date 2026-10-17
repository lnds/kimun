//! Code of a language that ignores case, as the analyzers read it: in lower
//! case, so that `IF`, `If` and `if` are one keyword and one name.
//!
//! Lines keep their length, byte for byte, so a name found in a folded line
//! is at the same place in the line as written.

use crate::loc::language::{LanguageSpec, is_fortran};

/// Constructs Fortran closes with `end` and their keyword, which may be
/// written apart (`end if`) or together (`endif`).
const FORTRAN_CONSTRUCTS: &[&str] = &[
    "if",
    "do",
    "select",
    "where",
    "forall",
    "associate",
    "block",
    "critical",
];

/// `lines` as the analyzers of `spec` read them: in lower case when the
/// language ignores case, as they are otherwise. In Fortran, the end of a construct written apart
/// is joined (`end if` reads `endif`), so that it does not read as the
/// construct it closes.
pub fn folded(lines: &[String], spec: &LanguageSpec) -> Vec<String> {
    if !spec.case_insensitive {
        return lines.to_vec();
    }
    let fortran = is_fortran(spec.name);
    lines
        .iter()
        .map(|line| {
            let line = line.to_ascii_lowercase();
            if fortran { joined_ends(&line) } else { line }
        })
        .collect()
}

/// `line` with each `end` and the construct keyword after it joined, the
/// spaces between them moved behind.
fn joined_ends(line: &str) -> String {
    let mut bytes = line.as_bytes().to_vec();
    let mut at = 0;
    while let Some(found) = line[at..].find("end") {
        let start = at + found;
        at = start + 3;
        if start > 0 && is_word(bytes[start - 1]) {
            continue;
        }
        let gap = bytes[at..].iter().take_while(|b| **b == b' ').count();
        let word_at = at + gap;
        let word_len = bytes[word_at..].iter().take_while(|b| is_word(**b)).count();
        let word = &line[word_at..word_at + word_len];
        if gap > 0 && FORTRAN_CONSTRUCTS.contains(&word) {
            bytes.copy_within(word_at..word_at + word_len, at);
            bytes[at + word_len..word_at + word_len].fill(b' ');
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| line.to_string())
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The name `folded` read in a line, as the line writes it.
fn as_written(name: &str, line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    match lower.find(name) {
        Some(at) => line[at..at + name.len()].to_string(),
        None => name.to_string(),
    }
}

/// Give each function, as its name and the line (1-based) that declares
/// it, the name `lines` write there, in a language read in lower case.
pub fn names_as_written<'f>(
    functions: impl Iterator<Item = (&'f mut String, usize)>,
    lines: &[String],
    spec: &LanguageSpec,
) {
    if spec.case_insensitive {
        for (name, start_line) in functions {
            *name = name_at(name, lines, start_line);
        }
    }
}

/// The name of a function declared at `start_line` (1-based) of `lines`,
/// as written there.
pub fn name_at(name: &str, lines: &[String], start_line: usize) -> String {
    match start_line.checked_sub(1).and_then(|at| lines.get(at)) {
        Some(line) => as_written(name, line),
        None => name.to_string(),
    }
}

#[cfg(test)]
#[path = "case_fold_test.rs"]
mod tests;
