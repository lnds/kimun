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
        let complexity = count_cognitive_for_lines(&code_lines, markers);
        let level = CognitiveLevel::from_complexity(complexity);
        return Some(FileCognitive {
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
        });
    }

    let total: usize = functions.iter().map(|f| f.complexity).sum();
    let max = functions.iter().map(|f| f.complexity).max().unwrap_or(0);
    let avg = total as f64 / functions.len() as f64;
    let level = CognitiveLevel::from_complexity(max);

    Some(FileCognitive {
        functions,
        total_complexity: total,
        max_complexity: max,
        avg_complexity: avg,
        level,
    })
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

/// Count cognitive complexity for brace-scoped languages.
/// Tracks nesting via a stack of brace contexts.
fn count_brace_scoped(func_lines: &[(usize, &str)], markers: &CognitiveMarkers) -> usize {
    let mut complexity: usize = 0;
    let mut brace_stack: Vec<BraceContext> = Vec::new();
    let mut nesting_depth: usize = 0;
    // Track if first line (function declaration) — skip its braces for nesting
    let mut is_first_line = true;

    for &(_, line) in func_lines {
        let trimmed = line.trim();
        let stripped = mask_strings(trimmed, markers.line_comments);

        // Detect keywords BEFORE counting braces on this line
        let line_result = classify_line(&stripped, markers);

        match line_result {
            LineClassification::Structural => {
                complexity += 1 + nesting_depth;
            }
            LineClassification::Hybrid => {
                // else if / elif: +1 only, no nesting change
                // But we need to handle the closing brace from the if-block
                // that just ended on this line too.
                complexity += 1;
            }
            LineClassification::Fundamental => {
                complexity += 1;
            }
            LineClassification::None => {}
        }

        // Count boolean operator sequences
        complexity += count_boolean_sequences(&stripped, markers);

        // Track braces for nesting depth
        let opens_flow = matches!(
            line_result,
            LineClassification::Structural | LineClassification::Fundamental
        );

        for ch in stripped.bytes() {
            if ch == b'{' {
                if is_first_line {
                    // First opening brace is the function body — don't increment nesting
                    brace_stack.push(BraceContext::Other);
                    is_first_line = false;
                } else if opens_flow {
                    brace_stack.push(BraceContext::FlowControl);
                    nesting_depth += 1;
                } else {
                    brace_stack.push(BraceContext::Other);
                }
            } else if ch == b'}'
                && let Some(ctx) = brace_stack.pop()
                && ctx == BraceContext::FlowControl
            {
                nesting_depth = nesting_depth.saturating_sub(1);
            }
        }

        // After processing braces, clear the flow flag
        // (only the first '{' on a structural line counts as flow)
        is_first_line = false;
    }

    complexity
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

/// Count indentation in spaces (tabs = 4 spaces).
fn indent_spaces(line: &str) -> usize {
    let mut spaces = 0;
    for ch in line.chars() {
        match ch {
            ' ' => spaces += 1,
            '\t' => spaces += 4,
            _ => break,
        }
    }
    spaces
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
    if markers.boolean_operators.is_empty() {
        return 0;
    }

    let mut count = 0;
    let mut last_op: Option<&str> = None;
    let bytes = stripped.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let mut matched_op: Option<&str> = None;

        for &op in markers.boolean_operators {
            let op_bytes = op.as_bytes();
            let op_len = op_bytes.len();
            if i + op_len <= len && &bytes[i..i + op_len] == op_bytes {
                // For word-based operators (and, or), check word boundaries
                if op.chars().next().is_some_and(|c| c.is_alphabetic()) {
                    let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
                    let after_ok = i + op_len >= len || !is_word_char(bytes[i + op_len]);
                    if before_ok && after_ok {
                        matched_op = Some(op);
                    }
                } else {
                    matched_op = Some(op);
                }
                break;
            }
        }

        if let Some(op) = matched_op {
            match last_op {
                None => {
                    // First operator in a sequence
                    count += 1;
                    last_op = Some(op);
                }
                Some(prev) if prev != op => {
                    // Operator type changed
                    count += 1;
                    last_op = Some(op);
                }
                _ => {
                    // Same operator, no increment
                }
            }
            i += op.len();
        } else {
            // Reset sequence when we encounter non-operator, non-whitespace,
            // non-identifier content that breaks the boolean expression
            let ch = bytes[i];
            if ch == b'(' || ch == b')' || ch == b';' || ch == b'{' || ch == b'}' || ch == b',' {
                last_op = None;
            }
            i += 1;
        }
    }

    count
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
