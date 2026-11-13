//! Cyclomatic complexity computation per file and per function.
//!
//! Counts linearly independent paths through code by scanning for
//! decision-point keywords (`if`, `for`, `while`, `match`, `catch`)
//! and boolean operators (`&&`, `||`). Each function starts at a
//! baseline complexity of 1. String literals are masked before
//! scanning to avoid false positives from keywords in strings.
//!
//! Levels: Simple (1-5), Moderate (6-10), Complex (11-20),
//! HighlyComplex (21-50), Extreme (>50).

use serde::Serialize;

use crate::branches::{count_ternaries, names_something};
use crate::loc::counter::LineKind;
use crate::util::mask_strings;

use super::detection::detect_functions;
use super::markers::ComplexityMarkers;

/// Complexity level classification based on McCabe thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CyclomaticLevel {
    Simple,
    Moderate,
    Complex,
    HighlyComplex,
    Extreme,
}

impl CyclomaticLevel {
    /// Map a numeric complexity value to a level classification.
    pub fn from_complexity(c: usize) -> Self {
        match c {
            0..=5 => Self::Simple,
            6..=10 => Self::Moderate,
            11..=20 => Self::Complex,
            21..=50 => Self::HighlyComplex,
            _ => Self::Extreme,
        }
    }

    /// Human-readable label for display in reports and JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Moderate => "moderate",
            Self::Complex => "complex",
            Self::HighlyComplex => "highly complex",
            Self::Extreme => "extreme",
        }
    }
}

/// Per-function complexity result with source location.
#[derive(Debug, Clone)]
pub struct FunctionComplexity {
    pub name: String,
    /// 1-based line number where the function declaration starts.
    pub start_line: usize,
    pub complexity: usize,
    pub level: CyclomaticLevel,
}

/// Aggregate complexity for an entire file: per-function breakdown
/// plus summary statistics (total, max, average).
#[derive(Debug, Clone)]
pub struct FileComplexity {
    pub functions: Vec<FunctionComplexity>,
    pub total_complexity: usize,
    pub max_complexity: usize,
    pub avg_complexity: f64,
    pub level: CyclomaticLevel,
}

/// Analyze cyclomatic complexity for a file's lines.
///
/// Detects functions via language-specific markers, then computes per-function
/// complexity by counting decision points (keywords like `if`, `for`, `while`,
/// `match`, and operators like `&&`, `||`). The code outside functions is
/// one more, `<file>`: always when no function is detected, else when it
/// has a decision point.
///
/// Returns `None` if the input is empty or contains no code lines.
pub fn analyze(
    lines: &[String],
    kinds: &[LineKind],
    markers: &ComplexityMarkers,
) -> Option<FileComplexity> {
    if lines.is_empty() || kinds.is_empty() {
        return None;
    }

    let code_lines = crate::detection::code_lines(lines, kinds, markers);

    if code_lines.is_empty() {
        return None;
    }

    let functions = detect_functions(lines, &code_lines, markers);
    let total: usize = functions.iter().map(|f| f.complexity).sum();
    let max = functions.iter().map(|f| f.complexity).max().unwrap_or(0);
    let avg = total as f64 / functions.len() as f64;
    let level = CyclomaticLevel::from_complexity(max);

    Some(FileComplexity {
        functions,
        total_complexity: total,
        max_complexity: max,
        avg_complexity: avg,
        level,
    })
}

/// Count the cyclomatic complexity of a sequence of code lines.
///
/// Starts at a baseline of 1 (the function itself is one path) and adds 1
/// for each decision point found. String literals are masked before scanning
/// to avoid counting keywords inside strings.
pub fn count_complexity_for_lines(
    func_lines: &[(usize, &str)],
    markers: &ComplexityMarkers,
) -> usize {
    let mut complexity: usize = 1; // baseline

    for &(_, line) in func_lines {
        let trimmed = line.trim();
        complexity += count_line_complexity(trimmed, markers);
    }

    complexity
}

/// Count multi-word keywords, masking matched regions to avoid double-counting.
/// Returns `(count, masked_line)` where `masked_line` has matches replaced with spaces.
fn count_multiword_keywords(line: &str, markers: &ComplexityMarkers) -> (usize, String) {
    let mut count = 0;
    let mut masked = line.to_string();
    for kw in markers.keywords {
        if kw.contains(' ') {
            count += count_keyword(&masked, kw);
            masked = masked.replace(kw, &" ".repeat(kw.len()));
        }
    }
    (count, masked)
}

/// Count single-word keywords using the (already multi-word-masked) line.
fn count_singleword_keywords(masked_line: &str, markers: &ComplexityMarkers) -> usize {
    let mut count = 0;
    for kw in markers.keywords {
        if !kw.contains(' ') {
            count += count_keyword_as(masked_line, kw, markers.keys);
        }
    }
    count
}

/// Count operator occurrences (substring match on the original stripped line).
fn count_operators_in_line(stripped: &str, markers: &ComplexityMarkers) -> usize {
    let mut count = 0;
    for op in markers.operators {
        count += count_operator(stripped, op);
    }
    count
}

/// Count decision points in a single line of code.
///
/// First masks string literals, then counts multi-word keywords (e.g. `else if`)
/// before single-word keywords to avoid double-counting. Finally counts
/// boolean operators (`&&`, `||`).
fn count_line_complexity(line: &str, markers: &ComplexityMarkers) -> usize {
    let stripped = mask_strings(line, markers.line_comments);
    let stripped = crate::detection::without_impl_for(&stripped);
    let (mw_count, masked) = count_multiword_keywords(&stripped, markers);
    mw_count
        + count_singleword_keywords(&masked, markers)
        + count_operators_in_line(&stripped, markers)
        + count_clause(&stripped, markers)
        + if markers.ternary {
            count_ternaries(&stripped)
        } else {
            0
        }
}

/// The clauses of a construct whose clauses branch on a line
/// (`ComplexityMarkers::clause_arrow`), one per arrow: not a catch-all
/// (`_`), as `default` is no branch, nor the arrow of an anonymous function
/// opened on the same line.
fn count_clause(stripped: &str, markers: &ComplexityMarkers) -> usize {
    let Some(arrow) = markers.clause_arrow else {
        return 0;
    };
    let mut from = 0;
    let mut count = 0;
    for (at, _) in stripped.match_indices(arrow) {
        let pattern = stripped[from..at].rsplit([',', '{']).next().unwrap_or("");
        let anonymous = count_keyword(&stripped[..at], "fn") > 0;
        count += usize::from(pattern.trim() != "_" && !anonymous);
        from = at + arrow.len();
    }
    count
}

/// Count whole-word occurrences of a keyword in a line.
///
/// Uses byte-level scanning with word-boundary checks: a match is only counted
/// when the characters immediately before and after the keyword are not
/// alphanumeric or underscore. This prevents `notify` from matching `if`.
fn count_keyword(line: &str, keyword: &str) -> usize {
    count_keyword_as(line, keyword, false)
}

/// Count whole-word occurrences of a keyword, other than a method of that
/// name (`.catch(`) and, with `keys`, a key or a symbol of that name
/// (`if:`, `:if`).
fn count_keyword_as(line: &str, keyword: &str, keys: bool) -> usize {
    let bytes = line.as_bytes();
    let len = keyword.len();
    line.match_indices(keyword)
        .filter(|(at, _)| is_word_at(bytes, *at, len) && !names_something(bytes, *at, len, keys))
        .count()
}

/// Whether the `len` bytes at `at` are a whole word.
fn is_word_at(line: &[u8], at: usize, len: usize) -> bool {
    let before = at.checked_sub(1).map(|b| line[b]);
    !before.is_some_and(is_word_char) && !line.get(at + len).is_some_and(|b| is_word_char(*b))
}

/// Check whether a byte is a word character (alphanumeric or underscore).
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Count non-overlapping occurrences of an operator substring in a line.
fn count_operator(line: &str, operator: &str) -> usize {
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = line[start..].find(operator) {
        count += 1;
        start += pos + operator.len();
    }
    count
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
