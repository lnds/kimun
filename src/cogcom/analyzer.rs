//! Cognitive complexity computation per file and per function.
//!
//! Implements the SonarSource cognitive complexity specification (2017).
//! Unlike cyclomatic complexity (which counts execution paths), cognitive
//! complexity measures the difficulty of *understanding* code by penalizing
//! nested control flow and rewarding linear structures.
//!
//! Rules:
//! - Structural keywords (if, for, while, match, catch): +1 + nesting depth
//! - Hybrid keywords (else if, elif): +1, no nesting increment
//! - Fundamental keywords (else): +1, increments nesting for body
//! - Boolean operator sequences: +1 per change in operator type
//!   (a && b && c = +1, a && b || c = +2)
//!
//! Levels: Simple (0-4), Moderate (5-9), Complex (10-14),
//! VeryComplex (15-24), Extreme (>=25).

use serde::Serialize;

use crate::loc::counter::LineKind;
use crate::util::indent_level as indent_spaces;
use crate::util::mask_strings;

use super::detection::detect_functions;
use super::markers::CognitiveMarkers;

/// Cognitive complexity level classification based on SonarQube thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveLevel {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
    Extreme,
}

impl CognitiveLevel {
    /// Map a numeric complexity value to a level classification.
    pub fn from_complexity(c: usize) -> Self {
        match c {
            0..=4 => Self::Simple,
            5..=9 => Self::Moderate,
            10..=14 => Self::Complex,
            15..=24 => Self::VeryComplex,
            _ => Self::Extreme,
        }
    }

    /// Human-readable label for display in reports and JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Moderate => "moderate",
            Self::Complex => "complex",
            Self::VeryComplex => "very complex",
            Self::Extreme => "extreme",
        }
    }
}

/// Per-function cognitive complexity result with source location.
#[derive(Debug, Clone)]
pub struct FunctionCognitive {
    pub name: String,
    /// 1-based line number where the function declaration starts.
    pub start_line: usize,
    pub complexity: usize,
    pub level: CognitiveLevel,
}

/// Aggregate cognitive complexity for an entire file.
#[derive(Debug, Clone)]
pub struct FileCognitive {
    pub functions: Vec<FunctionCognitive>,
    pub total_complexity: usize,
    pub max_complexity: usize,
    pub avg_complexity: f64,
    pub level: CognitiveLevel,
}

/// Build a `FileCognitive` for files where no functions were detected,
/// treating the entire file as a single implicit `<file>` function.
fn make_fallback_file_cognitive(complexity: usize) -> FileCognitive {
    let level = CognitiveLevel::from_complexity(complexity);
    FileCognitive {
        functions: vec![FunctionCognitive {
            name: "<file>".to_string(),
            start_line: 1,
            complexity,
            level,
        }],
        total_complexity: complexity,
        max_complexity: complexity,
        avg_complexity: complexity as f64,
        level,
    }
}

/// Aggregate per-function cognitive complexity results into a `FileCognitive`.
fn aggregate_functions(functions: Vec<FunctionCognitive>) -> FileCognitive {
    let total: usize = functions.iter().map(|f| f.complexity).sum();
    let max = functions.iter().map(|f| f.complexity).max().unwrap_or(0);
    let avg = total as f64 / functions.len() as f64;
    let level = CognitiveLevel::from_complexity(max);
    FileCognitive {
        functions,
        total_complexity: total,
        max_complexity: max,
        avg_complexity: avg,
        level,
    }
}

/// Analyze cognitive complexity for a file's lines.
///
/// Detects functions, then computes per-function cognitive complexity.
/// If no functions are detected, treats the entire file as a single
/// implicit `<file>` function.
///
/// Returns `None` if the input is empty or contains no code lines.
pub fn analyze(
    lines: &[String],
    kinds: &[LineKind],
    markers: &CognitiveMarkers,
) -> Option<FileCognitive> {
    if lines.is_empty() || kinds.is_empty() {
        return None;
    }

    let code_lines = crate::detection::code_lines(lines, kinds, markers);

    if code_lines.is_empty() {
        return None;
    }

    let functions = detect_functions(lines, &code_lines, markers);

    if functions.is_empty() {
        let complexity = count_cognitive_for_lines(&code_lines, markers);
        return Some(make_fallback_file_cognitive(complexity));
    }

    Some(aggregate_functions(functions))
}

/// What kind of brace context we're tracking.
#[derive(Debug, Clone, Copy, PartialEq)]
enum BraceContext {
    /// Brace opened by a control-flow keyword (if, for, while, etc.)
    FlowControl,
    /// Any other brace (function body, struct, etc.)
    Other,
}

/// Count the cognitive complexity of a sequence of code lines.
///
/// Tracks nesting depth via brace counting for brace-scoped languages.
/// For indent-scoped languages, uses indentation level relative to function base.
pub fn count_cognitive_for_lines(
    func_lines: &[(usize, &str)],
    markers: &CognitiveMarkers,
) -> usize {
    if func_lines.is_empty() {
        return 0;
    }

    if markers.brace_scoped {
        count_brace_scoped(func_lines, markers)
    } else {
        count_indent_scoped(func_lines, markers)
    }
}

/// What the brace-scoped count knows between one token and the next.
#[derive(Default)]
struct BraceScan {
    complexity: usize,
    brace_stack: Vec<BraceContext>,
    nesting_depth: usize,
    /// The function body has opened: its brace is no nesting.
    in_body: bool,
    /// A control-flow keyword was read and its block has not opened yet:
    /// the next `{` opens it, on this line or, Allman style, the next.
    pending_flow: bool,
}

impl BraceScan {
    /// A keyword of control flow: its increment, and the block it opens.
    fn keyword(&mut self, kind: LineClassification) {
        self.complexity += match kind {
            LineClassification::Structural => 1 + self.nesting_depth,
            _ => 1,
        };
        self.pending_flow = true;
    }

    fn open_brace(&mut self) {
        let context = if !self.in_body {
            self.in_body = true;
            BraceContext::Other
        } else if self.pending_flow {
            self.nesting_depth += 1;
            BraceContext::FlowControl
        } else {
            BraceContext::Other
        };
        // Only the first brace after a keyword is its block; later ones on
        // the line are closures or literals.
        self.pending_flow = false;
        self.brace_stack.push(context);
    }

    fn close_brace(&mut self) {
        if self.brace_stack.pop() == Some(BraceContext::FlowControl) {
            self.nesting_depth = self.nesting_depth.saturating_sub(1);
        }
    }

    /// The end of a line: a keyword whose body ended on it, as
    /// `if (x) return;`, opens no block on the next.
    fn end_line(&mut self, stripped: &str) {
        if stripped.trim_end().ends_with([';', '}']) {
            self.pending_flow = false;
        }
    }
}

/// Count cognitive complexity for brace-scoped languages. Keywords and
/// braces are read in order along each line, so that code written on one
/// line counts as it does on several.
fn count_brace_scoped(func_lines: &[(usize, &str)], markers: &CognitiveMarkers) -> usize {
    let mut scan = BraceScan::default();
    for &(_, line) in func_lines {
        let stripped = mask_strings(line.trim(), markers.line_comments);
        for token in line_tokens(&stripped, markers) {
            match token {
                Token::Keyword(kind) => scan.keyword(kind),
                Token::Open => scan.open_brace(),
                Token::Close => scan.close_brace(),
            }
        }
        scan.complexity += count_boolean_sequences(&stripped, markers);
        scan.end_line(&stripped);
    }
    scan.complexity
}

/// A token of a line that the brace-scoped count reads.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Keyword(LineClassification),
    Open,
    Close,
}

/// The keywords and braces of a line, in order. A hybrid keyword (`else
/// if`) is one token, not an `else` and an `if`.
fn line_tokens(stripped: &str, markers: &CognitiveMarkers) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < stripped.len() {
        match stripped.as_bytes()[i] {
            b'{' => tokens.push(Token::Open),
            b'}' => tokens.push(Token::Close),
            _ => {
                if let Some((kind, len)) = keyword_at(stripped, i, markers) {
                    tokens.push(Token::Keyword(kind));
                    i += len;
                    continue;
                }
            }
        }
        i += 1;
    }
    tokens
}

/// The keyword that starts at byte `i`, as a whole word, with its length.
fn keyword_at(
    stripped: &str,
    i: usize,
    markers: &CognitiveMarkers,
) -> Option<(LineClassification, usize)> {
    let kinds = [
        (markers.hybrid_keywords, LineClassification::Hybrid),
        (markers.structural_keywords, LineClassification::Structural),
        (
            markers.fundamental_keywords,
            LineClassification::Fundamental,
        ),
    ];
    let bytes = stripped.as_bytes();
    let starts_word = i == 0 || !is_word_char(bytes[i - 1]);
    if !starts_word {
        return None;
    }
    kinds.iter().find_map(|(keywords, kind)| {
        keywords
            .iter()
            .find(|kw| {
                bytes[i..].starts_with(kw.as_bytes())
                    && !bytes.get(i + kw.len()).is_some_and(|b| is_word_char(*b))
            })
            .map(|kw| (*kind, kw.len()))
    })
}

/// Count cognitive complexity for indent-scoped languages.
/// Uses indentation level relative to the function's base indent.
fn count_indent_scoped(func_lines: &[(usize, &str)], markers: &CognitiveMarkers) -> usize {
    let mut complexity: usize = 0;

    // Base indent is the function declaration line's indent
    let base_indent = if let Some(&(_, first_line)) = func_lines.first() {
        indent_spaces(first_line)
    } else {
        return 0;
    };

    let indent_unit = 4; // standard indent unit

    for &(_, line) in func_lines.iter().skip(1) {
        // skip function declaration line
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let stripped = mask_strings(trimmed, markers.line_comments);
        let line_indent = indent_spaces(line);
        let relative_indent = line_indent.saturating_sub(base_indent + indent_unit);
        let nesting_depth = relative_indent / indent_unit;

        let line_result = classify_line(&stripped, markers);

        match line_result {
            LineClassification::Structural => {
                complexity += 1 + nesting_depth;
            }
            LineClassification::Hybrid => {
                complexity += 1;
            }
            LineClassification::Fundamental => {
                complexity += 1;
            }
            LineClassification::None => {}
        }

        complexity += count_boolean_sequences(&stripped, markers);
    }

    complexity
}

/// Classification result for a single line.
#[derive(Debug, Clone, Copy, PartialEq)]
enum LineClassification {
    Structural,
    Hybrid,
    Fundamental,
    None,
}

/// Classify a line as structural, hybrid, fundamental, or none.
/// Checks hybrid keywords first (multi-word like "else if") to avoid
/// partial matches with structural/fundamental keywords.
fn classify_line(stripped: &str, markers: &CognitiveMarkers) -> LineClassification {
    // Check hybrid keywords first (multi-word, e.g. "else if", "elif")
    for kw in markers.hybrid_keywords {
        if contains_keyword(stripped, kw) {
            return LineClassification::Hybrid;
        }
    }

    // Check structural keywords
    for kw in markers.structural_keywords {
        if contains_keyword(stripped, kw) {
            return LineClassification::Structural;
        }
    }

    // Check fundamental keywords
    for kw in markers.fundamental_keywords {
        if contains_keyword(stripped, kw) {
            return LineClassification::Fundamental;
        }
    }

    LineClassification::None
}

/// Check whether a keyword appears as a whole word in the line.
fn contains_keyword(line: &str, keyword: &str) -> bool {
    let kw_bytes = keyword.as_bytes();
    let kw_len = kw_bytes.len();
    let line_bytes = line.as_bytes();
    let line_len = line_bytes.len();
    let mut i = 0;

    while i + kw_len <= line_len {
        if &line_bytes[i..i + kw_len] == kw_bytes {
            let before_ok = i == 0 || !is_word_char(line_bytes[i - 1]);
            let after_ok = i + kw_len >= line_len || !is_word_char(line_bytes[i + kw_len]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }

    false
}

/// Check whether a byte is a word character (alphanumeric or underscore).
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Count boolean operator sequence changes.
///
/// Per the SonarSource spec: +1 for the first operator in a sequence,
/// +1 each time the operator type changes.
/// `a && b && c` = 1 (one sequence of &&)
/// `a && b || c` = 2 (first && = +1, change to || = +1)
/// `a || b || c && d` = 2 (first || = +1, change to && = +1)
fn count_boolean_sequences(stripped: &str, markers: &CognitiveMarkers) -> usize {
    let mut count = 0;
    let mut last_op: Option<&str> = None;
    let bytes = stripped.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let Some(op) = operator_at(bytes, i, markers) else {
            // A bracket, `;` or `,` ends the boolean expression.
            if b"(){};,".contains(&bytes[i]) {
                last_op = None;
            }
            i += 1;
            continue;
        };
        // The first operator of a sequence, or a change of operator.
        if last_op != Some(op) {
            count += 1;
            last_op = Some(op);
        }
        i += op.len();
    }
    count
}

/// The boolean operator that starts at byte `i`, if any; a word operator
/// (`and`, `or`) only as a whole word.
fn operator_at(bytes: &[u8], i: usize, markers: &CognitiveMarkers) -> Option<&'static str> {
    let op = *markers
        .boolean_operators
        .iter()
        .find(|op| bytes[i..].starts_with(op.as_bytes()))?;
    let is_word = op.starts_with(|c: char| c.is_alphabetic());
    let bounded = (i == 0 || !is_word_char(bytes[i - 1]))
        && !bytes.get(i + op.len()).is_some_and(|b| is_word_char(*b));
    (!is_word || bounded).then_some(op)
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
