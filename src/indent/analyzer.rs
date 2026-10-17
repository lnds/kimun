use serde::Serialize;

use crate::loc::counter::LineKind;

/// Qualitative complexity classification based on indentation stddev
/// measured in logical indentation levels (4 spaces = 1 level).
///
/// Thresholds are initial heuristics inspired by Adam Tornhill's emphasis on
/// structural variance as a complexity signal ("Your Code as a Crime Scene",
/// Chapter 6). Tornhill's example: Configuration.java has sd=1.63 which he
/// calls "not too bad". They may need tuning based on real-world corpus analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplexityLevel {
    Low,
    Moderate,
    High,
    VeryHigh,
    /// Too few code lines for their spread to mean anything.
    Ungraded,
}

/// Fewer code lines than this leave a file ungraded.
pub const MIN_GRADED_LINES: usize = 10;

impl ComplexityLevel {
    pub fn from_stddev(stddev: f64) -> Self {
        if stddev < 1.0 {
            Self::Low
        } else if stddev < 1.5 {
            Self::Moderate
        } else if stddev < 2.0 {
            Self::High
        } else {
            Self::VeryHigh
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Moderate => "Moderate",
            Self::High => "High",
            Self::VeryHigh => "Very High",
            Self::Ungraded => "-",
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndentMetrics {
    pub code_lines: usize,
    pub stddev: f64,
    pub max_depth: usize,
    pub total_indent: usize,
    pub complexity: ComplexityLevel,
}

/// Count leading whitespace as logical indentation levels.
/// Each tab counts as one level; spaces are accumulated and divided by `tab_width`.
/// This matches Adam Tornhill's `complexity_analysis.py` approach where
/// "four spaces or one tab counts as one logical indentation."
pub fn indent_depth(line: &str, tab_width: usize) -> usize {
    let mut spaces = 0;
    for ch in line.chars() {
        match ch {
            ' ' => spaces += 1,
            '\t' => spaces += tab_width,
            _ => break,
        }
    }
    spaces / tab_width
}

/// Calculate indentation metrics from lines and their classifications.
/// Only considers `LineKind::Code` lines, at their structural depth
/// ([`structural_depths`]).
pub fn analyze(lines: &[String], kinds: &[LineKind], tab_width: usize) -> Option<IndentMetrics> {
    let code: Vec<&str> = lines
        .iter()
        .zip(kinds)
        .filter(|(_, k)| **k == LineKind::Code)
        .map(|(line, _)| line.as_str())
        .collect();
    let depths = structural_depths(&code, tab_width);

    if depths.is_empty() {
        return None;
    }

    let max_depth = *depths.iter().max().unwrap();
    let total_indent: usize = depths.iter().sum();
    let stddev = calculate_stddev(&depths);
    let complexity = if depths.len() < MIN_GRADED_LINES {
        ComplexityLevel::Ungraded
    } else {
        ComplexityLevel::from_stddev(stddev)
    };

    Some(IndentMetrics {
        code_lines: depths.len(),
        stddev,
        max_depth,
        total_indent,
        complexity,
    })
}

/// What a line sits inside: the parentheses or brackets opened on a line
/// of the given depth, or a block.
enum Open {
    Paren(usize),
    Block,
}

/// The depth of each code line as structure, not layout. A line that
/// continues a statement (inside parentheses or brackets, after a line
/// ending in an operator, or starting with one) is at most one level deeper
/// than the line it continues: arguments aligned under an opening
/// parenthesis are not nesting. Inside a block opened within parentheses,
/// such as the body of a closure, depths count again from that block.
pub fn structural_depths(code: &[&str], tab_width: usize) -> Vec<usize> {
    let mut stack: Vec<Open> = Vec::new();
    // Depth of the last line that started a statement.
    let mut base = 0;
    let mut continues = false;
    let mut depths = Vec::with_capacity(code.len());

    for line in code {
        let raw = indent_depth(line, tab_width);
        let text = code_text(line);
        let trimmed = text.trim();
        let closes = trimmed.starts_with([')', ']', '}']);
        if raw == 0 && !closes {
            // A new top-level statement: whatever was left open was misread.
            stack.clear();
        }

        let depth = match stack.last() {
            Some(Open::Paren(opener)) => raw.min(opener + 1),
            _ if continues || starts_with_operator(trimmed) => raw.min(base + 1),
            _ => {
                base = raw;
                raw
            }
        };
        depths.push(depth);

        for ch in trimmed.chars() {
            match ch {
                '(' | '[' => stack.push(Open::Paren(depth)),
                '{' => stack.push(Open::Block),
                ')' | ']' => pop_until(&mut stack, |o| matches!(o, Open::Paren(_))),
                '}' => pop_until(&mut stack, |o| matches!(o, Open::Block)),
                _ => {}
            }
        }
        if trimmed == "end" || trimmed.starts_with("end ") || trimmed.starts_with("end)") {
            if matches!(stack.last(), Some(Open::Block)) {
                stack.pop();
            }
        } else if opens_block(trimmed) {
            stack.push(Open::Block);
        }
        continues = ends_with_operator(trimmed);
    }
    depths
}

/// Pop `stack` down to and including the innermost entry `is_match` takes;
/// nothing when there is none.
fn pop_until(stack: &mut Vec<Open>, is_match: impl Fn(&Open) -> bool) {
    if let Some(at) = stack.iter().rposition(is_match) {
        stack.truncate(at);
    }
}

/// Whether a line ends opening a block whose body follows it, by a keyword
/// rather than a brace: `do`, `->`, `=>`.
fn opens_block(trimmed: &str) -> bool {
    trimmed == "do"
        || trimmed.ends_with(" do")
        || trimmed.ends_with("->")
        || trimmed.ends_with("=>")
}

/// Operators that, ending a line, leave the statement for the next one.
const TRAILING_OPERATORS: &[&str] = &["::", "&&", "||", "\\", "|>", "|"];

/// Operators that, starting a line, continue the statement before it.
const LEADING_OPERATORS: &[&str] = &[".", "|", "&&", "||"];

fn ends_with_operator(trimmed: &str) -> bool {
    TRAILING_OPERATORS.iter().any(|op| trimmed.ends_with(op))
}

fn starts_with_operator(trimmed: &str) -> bool {
    LEADING_OPERATORS.iter().any(|op| trimmed.starts_with(op)) && !trimmed.starts_with("..")
}

/// A line without the contents of its string and character literals and
/// without a trailing comment (`//`, or `#` followed by a space), so that
/// brackets and operators in them count for nothing.
fn code_text(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '"' => {
                out.push('"');
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += if chars[i] == '\\' { 2 } else { 1 };
                }
                out.push('"');
            }
            // A character literal: 'x' or '\x'. A lone quote (a Rust
            // lifetime) is kept as it is.
            '\'' if chars.get(i + 2) == Some(&'\'') => i += 2,
            '\'' if chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\'') => i += 3,
            '/' if chars.get(i + 1) == Some(&'/') => break,
            '#' if chars.get(i + 1) == Some(&' ') => break,
            _ => out.push(ch),
        }
        i += 1;
    }
    out
}

fn calculate_stddev(values: &[usize]) -> f64 {
    let n = values.len() as f64;
    if n <= 1.0 {
        return 0.0;
    }
    let mean = values.iter().sum::<usize>() as f64 / n;
    let variance = values
        .iter()
        .map(|&v| (v as f64 - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    variance.sqrt()
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
