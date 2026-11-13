//! Cognitive complexity of indent-scoped code (Python, Ruby, Elixir and
//! the like). The nesting of a line is the number of blocks around it that
//! nest, whatever the width of its indentation: a control structure, an
//! `else`, a nested function or a block passed to a call nest, while `try:`,
//! `with` or `begin` do not. A line that goes on inside open brackets keeps
//! the nesting of the line it continues.

use crate::branches::{bracket_balance, count_ternaries};
use crate::util::indent_level;
use crate::util::mask_strings;

use super::analyzer::{LineClassification, classify_line, increment};
use super::booleans::Booleans;
use super::markers::CognitiveMarkers;

/// A block that holds the lines indented deeper than its first line.
struct Block {
    indent: usize,
    nests: bool,
}

/// The count of a function: its declaration, and any line of a signature
/// that goes on from it, adds nothing.
pub(super) fn count_function(lines: &[(usize, &str)], markers: &CognitiveMarkers) -> usize {
    count(lines, markers, true)
}

/// The count of the code outside functions.
pub(super) fn count_outside(lines: &[(usize, &str)], markers: &CognitiveMarkers) -> usize {
    count(lines, markers, false)
}

fn count(lines: &[(usize, &str)], markers: &CognitiveMarkers, declared: bool) -> usize {
    let mut blocks: Vec<Block> = Vec::new();
    let mut open = 0isize;
    let mut nesting = 0;
    let mut booleans = Booleans::default();
    let mut complexity = 0;
    for (k, &(_, line)) in lines.iter().enumerate() {
        let stripped = mask_strings(line.trim(), markers.line_comments);
        if stripped.trim().is_empty() {
            continue;
        }
        let statement = open <= 0;
        let indent = indent_level(line);
        if statement {
            while blocks.last().is_some_and(|b| b.indent >= indent) {
                blocks.pop();
            }
            nesting = blocks.iter().filter(|b| b.nests).count();
        }
        open = (open + bracket_balance(&stripped)).max(0);
        if declared && k == 0 {
            continue;
        }
        let kind = classify_line(&stripped, markers);
        complexity += increment(kind, nesting) + booleans.line(&stripped, markers);
        if markers.ternary {
            complexity += count_ternaries(&stripped) * (1 + nesting);
        }
        if statement {
            let nests = kind != LineClassification::None || opens_nested(&stripped);
            blocks.push(Block { indent, nests });
        }
    }
    complexity
}

/// Whether a line that is no control structure opens a block that nests: a
/// nested function (`def`, `lambda`), a block passed to a call (`do`, `do
/// |x|`, Elixir's `fn x ->`), or a `when` clause.
fn opens_nested(stripped: &str) -> bool {
    let line = stripped.trim();
    let first = line.split_whitespace().next().unwrap_or("");
    let ends_with_do = line.ends_with(" do") || (line.ends_with('|') && line.contains(" do |"));
    let anonymous = line.ends_with("->") && has_word(line, "fn");
    matches!(first, "def" | "async" | "lambda" | "when") || ends_with_do || anonymous
}

fn has_word(line: &str, word: &str) -> bool {
    line.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|w| w == word)
}
