use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use super::fsm::{State, StringKind, step_in_block_comment, step_in_string, step_normal};
use super::language::LanguageSpec;
use crate::util::is_binary_reader;

#[derive(Debug, Default, Clone)]
pub struct FileStats {
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Blank,
    Comment,
    Code,
}

/// Count lines of code, comments, and blanks in a source file.
///
/// Returns `None` if the file contains null bytes (binary detection).
/// Opens the file, checks for binary content, then delegates to `count_reader`.
pub fn count_lines(path: &Path, spec: &LanguageSpec) -> io::Result<Option<FileStats>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if is_binary_reader(&mut reader)? {
        return Ok(None);
    }

    Ok(Some(count_reader(reader, spec)))
}

/// Classify each line of source code as Blank, Comment, or Code.
pub fn classify_reader<R: BufRead>(reader: R, spec: &LanguageSpec) -> Vec<LineKind> {
    process_lines(reader, spec)
}

/// Count lines from a buffered reader, aggregating per-line classifications
/// into blank, comment, and code totals.
pub fn count_reader<R: BufRead>(reader: R, spec: &LanguageSpec) -> FileStats {
    let mut stats = FileStats::default();
    for kind in process_lines(reader, spec) {
        match kind {
            LineKind::Blank => stats.blank += 1,
            LineKind::Comment => stats.comment += 1,
            LineKind::Code => stats.code += 1,
        }
    }
    stats
}

/// Classify a single non-blank, non-shebang line given the current FSM state.
/// Updates `state` in place and returns the line classification.
fn classify_line(line: &str, state: &mut State, spec: &LanguageSpec) -> LineKind {
    let mut has_code = matches!(state, State::InString(_));
    let mut has_comment = matches!(state, State::InBlockComment(_));
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let result = match &state {
            State::Normal => step_normal(&bytes[i..], spec, bytes, i),
            State::InString(kind) => step_in_string(&bytes[i..], bytes[i], kind, len, i),
            State::InBlockComment(depth) => step_in_block_comment(&bytes[i..], spec, *depth),
        };
        has_code |= result.has_code;
        has_comment |= result.has_comment;
        if let Some(new_state) = result.new_state {
            *state = new_state;
        }
        i += result.advance;
        if result.break_line {
            break;
        }
    }

    // Reset InString at end of line for non-triple-quote strings
    if matches!(
        state,
        State::InString(StringKind::Double | StringKind::Single)
    ) {
        *state = State::Normal;
    }

    if has_code {
        LineKind::Code
    } else if has_comment {
        LineKind::Comment
    } else {
        LineKind::Blank
    }
}

/// Process all lines from a reader through the FSM, returning a classification
/// for each line. Handles shebang detection on the first line and blank line
/// short-circuiting outside of block comments.
fn process_lines<R: BufRead>(reader: R, spec: &LanguageSpec) -> Vec<LineKind> {
    let mut kinds = Vec::new();
    let mut state = State::Normal;
    let mut is_first_line = true;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        if is_first_line {
            is_first_line = false;
            if line.starts_with("#!") {
                kinds.push(LineKind::Code);
                continue;
            }
        }

        if line.trim().is_empty() && !matches!(state, State::InBlockComment(_)) {
            kinds.push(LineKind::Blank);
            continue;
        }

        kinds.push(classify_line(&line, &mut state, spec));
    }

    kinds
}

#[cfg(test)]
#[path = "counter_test.rs"]
mod tests;
