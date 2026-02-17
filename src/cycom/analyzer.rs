use serde::Serialize;

use crate::loc::counter::LineKind;
use crate::util::mask_strings;

use super::detection::detect_functions;
use super::markers::ComplexityMarkers;

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
    pub fn from_complexity(c: usize) -> Self {
        match c {
            0..=5 => Self::Simple,
            6..=10 => Self::Moderate,
            11..=20 => Self::Complex,
            21..=50 => Self::HighlyComplex,
            _ => Self::Extreme,
        }
    }

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

#[derive(Debug, Clone)]
pub struct FunctionComplexity {
    pub name: String,
    pub start_line: usize, // 1-based
    pub complexity: usize,
    pub level: CyclomaticLevel,
}

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
/// `match`, and operators like `&&`, `||`). If no functions are detected,
/// treats the entire file as a single implicit `<file>` function.
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

    let code_lines: Vec<(usize, &str)> = lines
        .iter()
        .zip(kinds)
        .enumerate()
        .filter(|(_, (_, k))| **k == LineKind::Code)
        .map(|(i, (line, _))| (i, line.as_str()))
        .collect();

    if code_lines.is_empty() {
        return None;
    }

    let functions = detect_functions(lines, &code_lines, markers);

    if functions.is_empty() {
        let complexity = count_complexity_for_lines(&code_lines, markers);
        let level = CyclomaticLevel::from_complexity(complexity);
        return Some(FileComplexity {
            functions: vec![FunctionComplexity {
                name: "<file>".to_string(),
                start_line: 1,
                complexity,
                level,
            }],
            total_complexity: complexity,
            max_complexity: complexity,
            avg_complexity: complexity as f64,
            level,
        });
    }

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
            count += count_keyword(masked_line, kw);
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
    let (mw_count, masked) = count_multiword_keywords(&stripped, markers);
    mw_count
        + count_singleword_keywords(&masked, markers)
        + count_operators_in_line(&stripped, markers)
}

/// Count whole-word occurrences of a keyword in a line.
///
/// Uses byte-level scanning with word-boundary checks: a match is only counted
/// when the characters immediately before and after the keyword are not
/// alphanumeric or underscore. This prevents `notify` from matching `if`.
fn count_keyword(line: &str, keyword: &str) -> usize {
    let kw_bytes = keyword.as_bytes();
    let kw_len = kw_bytes.len();
    let line_bytes = line.as_bytes();
    let line_len = line_bytes.len();
    let mut count = 0;
    let mut i = 0;

    while i + kw_len <= line_len {
        if &line_bytes[i..i + kw_len] == kw_bytes {
            let before_ok = i == 0 || !is_word_char(line_bytes[i - 1]);
            let after_ok = i + kw_len >= line_len || !is_word_char(line_bytes[i + kw_len]);
            if before_ok && after_ok {
                count += 1;
                i += kw_len;
                continue;
            }
        }
        i += 1;
    }

    count
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
