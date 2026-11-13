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
//! - Ternary operators: +1 + nesting depth
//! - A jump to a label (`goto`, `break label`, `continue label`): +1
//! - A lambda or closure nests its body, with no increment of its own
//!
//! Levels: Simple (0-4), Moderate (5-9), Complex (10-14),
//! VeryComplex (15-24), Extreme (>=25).

use serde::Serialize;

use crate::branches::{count_ternaries, names_something};
use crate::detection::FunctionSyntax;
use crate::loc::counter::LineKind;
use crate::util::indent_level as indent_spaces;
use crate::util::mask_strings;

use super::booleans::Booleans;
pub(super) use super::booleans::count_boolean_sequences;
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
/// The code outside functions is one more, `<file>`: always when no
/// function is detected, else when it adds complexity.
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

    Some(aggregate_functions(detect_functions(
        lines,
        &code_lines,
        markers,
    )))
}

/// What kind of brace context we're tracking.
#[derive(Debug, Clone, Copy, PartialEq)]
enum BraceContext {
    /// Brace that nests: the block of a control-flow keyword (if, for,
    /// while, etc.), or the body of a lambda or closure.
    Nesting,
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

    match markers.function_syntax {
        FunctionSyntax::Fortran => super::fortran::count(func_lines, markers),
        FunctionSyntax::Pascal => count_indent_scoped(func_lines, markers, 2),
        _ if markers.brace_scoped => count_brace_scoped(func_lines, markers),
        _ => super::indented::count_function(func_lines, markers),
    }
}

/// Count the cognitive complexity of the code outside functions. It has
/// no declaration: its first brace may open a block of control flow, and
/// a statement at the left margin is not nested.
pub fn count_cognitive_outside(lines: &[(usize, &str)], markers: &CognitiveMarkers) -> usize {
    match markers.function_syntax {
        FunctionSyntax::Fortran => super::fortran::count(lines, markers),
        FunctionSyntax::Pascal => count_indented(lines, markers, 2, 0),
        _ if markers.brace_scoped => count_braces(lines, markers, BraceScan::outside()),
        _ => super::indented::count_outside(lines, markers),
    }
}

/// The increment a line's classification adds at `nesting_depth`.
pub(super) fn increment(line_result: LineClassification, nesting_depth: usize) -> usize {
    match line_result {
        LineClassification::Structural => 1 + nesting_depth,
        LineClassification::Hybrid | LineClassification::Fundamental => 1,
        LineClassification::None => 0,
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
    /// A scan of code outside any function body.
    fn outside() -> Self {
        Self {
            in_body: true,
            ..Self::default()
        }
    }

    /// A keyword of control flow: its increment, and the block it opens.
    fn keyword(&mut self, kind: LineClassification) {
        self.complexity += match kind {
            LineClassification::Structural => 1 + self.nesting_depth,
            _ => 1,
        };
        self.pending_flow = true;
    }

    /// A `{`: the block of a pending keyword when it stands outside
    /// parentheses, as a pattern's or a literal's inside them is no block
    /// (`if let Some(Foo { a }) = x {`); the body of a lambda nests too.
    fn open_brace(&mut self, brace: Brace) {
        let nests = self.in_body && (brace.lambda || (self.pending_flow && !brace.in_parens));
        self.in_body = true;
        // Only the first brace after a keyword is its block; later ones on
        // the line are closures or literals.
        self.pending_flow &= brace.in_parens;
        self.nesting_depth += usize::from(nests);
        self.brace_stack.push(if nests {
            BraceContext::Nesting
        } else {
            BraceContext::Other
        });
    }

    fn close_brace(&mut self) {
        if self.brace_stack.pop() == Some(BraceContext::Nesting) {
            self.nesting_depth = self.nesting_depth.saturating_sub(1);
        }
    }

    /// A line: its keywords and braces in order, then what it adds as a
    /// whole at the nesting it started at.
    fn read_line(&mut self, stripped: &str, markers: &CognitiveMarkers, booleans: &mut Booleans) {
        let nesting = self.nesting_depth;
        for token in line_tokens(stripped, markers) {
            match token {
                Token::Keyword(kind) => self.keyword(kind),
                Token::Open(brace) => self.open_brace(brace),
                Token::Close => self.close_brace(),
            }
        }
        let ternaries = if markers.ternary {
            count_ternaries(stripped)
        } else {
            0
        };
        self.complexity += booleans.line(stripped, markers)
            + jumps_to_labels(stripped, markers)
            + ternaries * (1 + nesting);
        self.end_line(stripped);
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
    count_braces(func_lines, markers, BraceScan::default())
}

fn count_braces(
    func_lines: &[(usize, &str)],
    markers: &CognitiveMarkers,
    mut scan: BraceScan,
) -> usize {
    let mut booleans = Booleans::default();
    for &(_, line) in func_lines {
        let stripped = mask_strings(line.trim(), markers.line_comments);
        let stripped = crate::detection::without_impl_for(&stripped);
        scan.read_line(&stripped, markers, &mut booleans);
    }
    scan.complexity
}

/// Jumps to a label: `goto`, and `break` or `continue` naming a label
/// (`break outer;`, `continue@outer`). A Rust `break` may carry a value
/// instead, so there only `goto` counts.
fn jumps_to_labels(stripped: &str, markers: &CognitiveMarkers) -> usize {
    let labels = markers.function_syntax != FunctionSyntax::KeywordWithLifetimes;
    // Words and the empty gaps between separators: a label is the word one
    // separator after its jump.
    let words: Vec<&str> = stripped
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '@'))
        .collect();
    let names_label = |at: usize| {
        let next = words.get(at + 1);
        labels && next.is_some_and(|w| w.starts_with(char::is_alphabetic))
    };
    words
        .iter()
        .enumerate()
        .filter(|(at, word)| match **word {
            "goto" => true,
            "break" | "continue" => names_label(*at),
            w => w.starts_with("break@") || w.starts_with("continue@"),
        })
        .count()
}

/// A `{`: whether it stands inside parentheses, and whether it opens the
/// body of a lambda or a closure (after `=>`, `->` or `|args|`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Brace {
    in_parens: bool,
    lambda: bool,
}

/// A token of a line that the brace-scoped count reads.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Keyword(LineClassification),
    Open(Brace),
    Close,
}

/// The keywords and braces of a line, in order. A hybrid keyword (`else
/// if`) is one token, not an `else` and an `if`; the `while` that closes a
/// `do` block (`} while (x);`) is none.
fn line_tokens(stripped: &str, markers: &CognitiveMarkers) -> Vec<Token> {
    let mut reader = Tokens {
        stripped,
        markers,
        parens: 0,
        tokens: Vec::new(),
    };
    let mut i = 0;
    while i < stripped.len() {
        i += reader.read(i);
    }
    reader.tokens
}

/// The tokens of a line being read, with the parentheses open.
struct Tokens<'l> {
    stripped: &'l str,
    markers: &'l CognitiveMarkers,
    parens: usize,
    tokens: Vec<Token>,
}

impl Tokens<'_> {
    /// Read what starts at `i`; gives how many bytes it takes.
    fn read(&mut self, i: usize) -> usize {
        match self.stripped.as_bytes()[i] {
            b'(' => self.parens += 1,
            b')' => self.parens = self.parens.saturating_sub(1),
            b'{' => self.tokens.push(Token::Open(Brace {
                in_parens: self.parens > 0,
                lambda: opens_lambda(&self.stripped[..i], self.markers),
            })),
            b'}' => self.tokens.push(Token::Close),
            _ => return self.keyword(i),
        }
        1
    }

    fn keyword(&mut self, i: usize) -> usize {
        let Some((kind, len)) = keyword_at(self.stripped, i, self.markers) else {
            return 1;
        };
        if !is_do_tail(&self.stripped[..i], &self.stripped[i..i + len]) {
            self.tokens.push(Token::Keyword(kind));
        }
        len
    }
}

/// Whether a `{` after `before` opens the body of a lambda or a closure:
/// `before` ends with an arrow of the language, which is no `||`, on a line
/// that is no `case` (Java's `case 1 -> {`).
fn opens_lambda(before: &str, markers: &CognitiveMarkers) -> bool {
    let before = before.trim_end();
    let arrow = markers
        .lambda_arrows
        .iter()
        .any(|a| before.ends_with(a) && !before.ends_with("||"));
    arrow && !contains_keyword(before, "case", false)
}

/// Whether `keyword` after `before` is the `while` that closes a `do` block.
fn is_do_tail(before: &str, keyword: &str) -> bool {
    keyword == "while" && before.trim_end().ends_with('}')
}

/// The keyword that starts at byte `i`, as a whole word, with its length.
fn keyword_at(
    stripped: &str,
    i: usize,
    markers: &CognitiveMarkers,
) -> Option<(LineClassification, usize)> {
    let kinds = keyword_kinds(markers);
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
                    && !names_something(bytes, i, kw.len(), markers.keys)
            })
            .map(|kw| (*kind, kw.len()))
    })
}

/// Count cognitive complexity for indent-scoped languages.
/// Uses indentation level relative to the function's base indent, in
/// steps of `indent_unit` columns.
fn count_indent_scoped(
    func_lines: &[(usize, &str)],
    markers: &CognitiveMarkers,
    indent_unit: usize,
) -> usize {
    // The body sits one level deeper than the declaration, which is skipped.
    let Some((&(_, declaration), body)) = func_lines.split_first() else {
        return 0;
    };
    let body_indent = indent_spaces(declaration) + indent_unit;
    count_indented(body, markers, indent_unit, body_indent)
}

/// Count cognitive complexity over lines whose nesting is their
/// indentation past `body_indent`, in steps of `indent_unit` columns.
fn count_indented(
    lines: &[(usize, &str)],
    markers: &CognitiveMarkers,
    indent_unit: usize,
    body_indent: usize,
) -> usize {
    let mut complexity: usize = 0;
    for &(_, line) in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let stripped = mask_strings(trimmed, markers.line_comments);
        let relative_indent = indent_spaces(line).saturating_sub(body_indent);
        let nesting_depth = relative_indent / indent_unit;

        let line_result = classify_line(&stripped, markers);

        complexity += increment(line_result, nesting_depth);

        complexity += count_boolean_sequences(&stripped, markers);
    }

    complexity
}

/// The keywords of each classification, hybrid first, as `else if` holds an
/// `if`.
fn keyword_kinds(markers: &CognitiveMarkers) -> [(&'static [&'static str], LineClassification); 3] {
    [
        (markers.hybrid_keywords, LineClassification::Hybrid),
        (markers.structural_keywords, LineClassification::Structural),
        (
            markers.fundamental_keywords,
            LineClassification::Fundamental,
        ),
    ]
}

/// Classification result for a single line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum LineClassification {
    Structural,
    Hybrid,
    Fundamental,
    None,
}

/// Classify a line as structural, hybrid, fundamental, or none.
/// Checks hybrid keywords first (multi-word like "else if") to avoid
/// partial matches with structural/fundamental keywords.
pub(super) fn classify_line(stripped: &str, markers: &CognitiveMarkers) -> LineClassification {
    // Check hybrid keywords first (multi-word, e.g. "else if", "elif")
    let kinds = keyword_kinds(markers);
    kinds
        .into_iter()
        .find(|(keywords, _)| {
            keywords
                .iter()
                .any(|kw| contains_keyword(stripped, kw, markers.keys))
        })
        .map_or(LineClassification::None, |(_, kind)| kind)
}

/// Check whether a keyword appears as a whole word in the line, other than
/// as a method, a key or a symbol (`names_something`).
fn contains_keyword(line: &str, keyword: &str, keys: bool) -> bool {
    let bytes = line.as_bytes();
    let len = keyword.len();
    line.match_indices(keyword).any(|(at, _)| {
        let before_ok = at == 0 || !is_word_char(bytes[at - 1]);
        let after_ok = !bytes.get(at + len).is_some_and(|b| is_word_char(*b));
        before_ok && after_ok && !names_something(bytes, at, len, keys)
    })
}

/// Check whether a byte is a word character (alphanumeric or underscore).
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
