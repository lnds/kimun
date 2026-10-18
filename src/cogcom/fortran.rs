//! Cognitive complexity of Fortran, whose nesting is read from the
//! constructs a statement opens and the `end` that closes each, not from
//! indentation, which fixed form lacks.

use crate::unit_bodies::fortran_statement;
use crate::util::mask_strings;

use super::analyzer::{classify_line, count_boolean_sequences, increment};
use super::markers::CognitiveMarkers;

/// Constructs a Fortran statement may open, after a construct name.
const FORTRAN_OPENERS: &[&str] = &[
    "do",
    "if",
    "select",
    "where",
    "forall",
    "block",
    "associate",
];

/// Whether a statement starts with the name of a construct (`outer: do`).
fn is_construct_name(stripped: &str) -> bool {
    stripped.split_once(':').is_some_and(|(name, rest)| {
        let name = name.trim();
        !rest.starts_with(':')
            && !name.is_empty()
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    })
}

/// What a Fortran statement opens: a construct, with the label that ends
/// it when it is a labelled `do` (`do 10 i = 1, n`).
fn fortran_opens(stripped: &str, words: &[String]) -> Option<Option<String>> {
    let words = if is_construct_name(stripped) {
        words.get(1..)?
    } else {
        words
    };
    let first = words.first()?.as_str();
    let opens = match first {
        "do" | "select" => true,
        "if" => words.last().is_some_and(|w| w == "then"),
        "where" | "forall" | "block" | "associate" => words.len() == 1,
        _ => false,
    };
    let label = words
        .get(1)
        .filter(|w| first == "do" && w.bytes().all(|b| b.is_ascii_digit()));
    opens.then(|| label.cloned())
}

/// Whether a Fortran statement closes a construct, its `end` joined.
fn fortran_closes(words: &[String]) -> bool {
    words.first().is_some_and(|w| {
        FORTRAN_OPENERS
            .iter()
            .any(|o| w.strip_prefix("end") == Some(*o))
    })
}

/// Count cognitive complexity for Fortran. Nesting is the depth of the
/// constructs open: `if … then`, `do`, `select`, `where` and `forall` open
/// one, its `end` closes it, and a labelled `do` closes at the statement
/// that carries its label.
pub(super) fn count(func_lines: &[(usize, &str)], markers: &CognitiveMarkers) -> usize {
    let mut complexity = 0;
    let mut open: Vec<Option<String>> = Vec::new();
    for &(_, line) in func_lines.iter().skip(1) {
        let stripped = mask_strings(line.trim(), markers.line_comments);
        complexity += increment(classify_line(&stripped, markers), open.len());
        complexity += count_boolean_sequences(&stripped, markers);
        let (label, words) = fortran_statement(line);
        if let Some(opened) = fortran_opens(&stripped, &words) {
            open.push(opened);
        } else if fortran_closes(&words) {
            open.pop();
        }
        while label.is_some() && open.last() == Some(&label) {
            open.pop();
        }
    }
    complexity
}
