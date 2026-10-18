//! The commented_out_code smell: comment lines that read as code.

use crate::loc::counter::LineKind;

use super::analyzer::{SmellInstance, SmellKind};

/// Strong code-like patterns: statement terminators, braces, control flow.
/// These rarely appear in documentation comments.
const STRONG_CODE_PATTERNS: &[&str] = &[";", "{", "}", "return ", "if(", "for(", "while(", "else{"];

/// Weaker code-like patterns: keywords that also appear in documentation.
/// These need to co-occur with strong patterns to be meaningful.
const WEAK_CODE_PATTERNS: &[&str] = &[
    "if ",
    "for ",
    "while ",
    "else ",
    "let ",
    "var ",
    "const ",
    "fn ",
    "def ",
    "func ",
    "function ",
    "class ",
    "= ",
    "=>",
    "->",
];

/// Openers of documentation comments: Rust `///` and `//!`, C# and Rust
/// `///`, JSDoc, Javadoc and Rust `/** */` and `/*! */`. Code in them is an
/// example (in Rust, a doctest), not dead code.
const DOC_LINE_MARKERS: &[&str] = &["///", "//!"];
const DOC_BLOCK_MARKERS: &[&str] = &["/**", "/*!"];

/// Emit a `CommentedOutCode` smell if `run_len >= 2` and `run_start` is set.
fn flush_code_run(run_start: Option<usize>, run_len: usize, smells: &mut Vec<SmellInstance>) {
    if run_len >= 2
        && let Some(start) = run_start
    {
        smells.push(SmellInstance {
            kind: SmellKind::CommentedOutCode,
            line: start + 1,
            detail: format!("{run_len} lines of commented-out code"),
        });
    }
}

/// Detect consecutive comment lines that look like commented-out code.
///
/// A comment line is considered "code-like" if it contains at least 1 strong
/// pattern plus 1 more pattern (strong or weak), totaling >= 2 pattern hits
/// with at least one being strong. Documentation comments are left out
/// (`documentation_lines`).
///
/// Triggers when >= 2 consecutive code-like comment lines are found.
pub fn detect_commented_out_code(lines: &[String], kinds: &[LineKind]) -> Vec<SmellInstance> {
    let documentation = documentation_lines(lines, kinds);
    let mut smells = Vec::new();
    let mut run_start: Option<usize> = None;
    let mut run_len = 0;

    for (i, line) in lines.iter().enumerate() {
        let is_comment = kinds.get(i) == Some(&LineKind::Comment) && !documentation[i];
        if is_comment && has_code_patterns(line) {
            run_start.get_or_insert(i);
            run_len += 1;
        } else {
            flush_code_run(run_start, run_len, &mut smells);
            run_start = None;
            run_len = 0;
        }
    }
    flush_code_run(run_start, run_len, &mut smells);
    smells
}

/// Which lines are documentation: `///` and `//!` lines, the lines of a
/// `/** */` or `/*! */` block, and an example indented with a tab in the
/// comment right above a declaration (Go doc comments).
fn documentation_lines(lines: &[String], kinds: &[LineKind]) -> Vec<bool> {
    let mut doc = vec![false; lines.len()];
    let mut in_block = false;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let opens = DOC_BLOCK_MARKERS.iter().any(|m| trimmed.starts_with(m));
        in_block |= opens;
        doc[i] = in_block || DOC_LINE_MARKERS.iter().any(|m| trimmed.starts_with(m));
        if in_block && trimmed.contains("*/") {
            in_block = false;
        }
    }
    mark_doc_examples(lines, kinds, &mut doc);
    doc
}

/// Declarations a Go doc comment sits right above.
const GO_DECLARATIONS: &[&str] = &["func ", "type ", "var ", "const ", "package "];

/// Mark the comment blocks right above a declaration whose lines include an
/// example indented with a tab after `//`, as Go doc comments write them.
fn mark_doc_examples(lines: &[String], kinds: &[LineKind], doc: &mut [bool]) {
    let mut block_start = None;
    for (i, line) in lines.iter().enumerate() {
        match kinds.get(i) {
            Some(LineKind::Comment) => {
                block_start.get_or_insert(i);
            }
            Some(LineKind::Code) => {
                let declares = GO_DECLARATIONS.iter().any(|d| line.starts_with(d));
                if let Some(start) = block_start.take()
                    && declares
                    && lines[start..i]
                        .iter()
                        .any(|l| l.trim_start().starts_with("//\t"))
                {
                    doc[start..i].fill(true);
                }
            }
            _ => block_start = None,
        }
    }
}

/// Check if a comment line contains enough code-like patterns to suggest
/// it is commented-out code rather than a documentation comment.
///
/// Requires at least 1 strong pattern and a total of >= 2 pattern matches.
fn has_code_patterns(line: &str) -> bool {
    // Strip common comment prefixes
    let stripped = line
        .trim()
        .trim_start_matches("///")
        .trim_start_matches("//")
        .trim_start_matches('#')
        .trim_start_matches("--")
        .trim_start_matches('%')
        .trim_start_matches("/*")
        .trim_start_matches('*')
        .trim();

    let strong = STRONG_CODE_PATTERNS
        .iter()
        .filter(|p| stripped.contains(**p))
        .count();
    if strong == 0 {
        return false;
    }
    let weak = WEAK_CODE_PATTERNS
        .iter()
        .filter(|p| stripped.contains(**p))
        .count();
    strong + weak >= 2
}

#[cfg(test)]
#[path = "commented_test.rs"]
mod tests;
