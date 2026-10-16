use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::ops::Range;
use std::path::Path;

use super::fsm::{
    State, StringKind, step_in_block_comment, step_in_doc_attribute, step_in_string, step_normal,
};
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

/// The part of a line that a raw string takes, delimiters included.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSpan {
    pub start: usize,
    pub end: usize,
    /// Whether the string opens here, rather than come from the line before.
    pub opens: bool,
}

/// Classify each line of source code as Blank, Comment, or Code.
pub fn classify_reader<R: BufRead>(reader: R, spec: &LanguageSpec) -> Vec<LineKind> {
    let mut kinds = Vec::new();
    process_lines(reader.lines().map_while(Result::ok), spec, |kind, _| {
        kinds.push(kind)
    });
    kinds
}

/// Where the raw strings of `lines` are, line by line.
pub fn raw_string_spans(lines: &[String], spec: &LanguageSpec) -> Vec<Vec<RawSpan>> {
    let mut spans = Vec::new();
    process_lines(lines.iter(), spec, |_, of_line| spans.push(of_line));
    spans
}

/// Count lines from a buffered reader, aggregating per-line classifications
/// into blank, comment, and code totals.
pub fn count_reader<R: BufRead>(reader: R, spec: &LanguageSpec) -> FileStats {
    let mut stats = FileStats::default();
    for kind in classify_reader(reader, spec) {
        match kind {
            LineKind::Blank => stats.blank += 1,
            LineKind::Comment => stats.comment += 1,
            LineKind::Code => stats.code += 1,
        }
    }
    stats
}

fn in_raw_string(state: &State) -> bool {
    matches!(state, State::InString(StringKind::Raw(_)))
}

/// Add the bytes at `at` to the raw strings of a line when they belong to
/// one: to the one being read when it `goes_on`, as a new one when it
/// opens.
fn note_raw(spans: &mut Vec<RawSpan>, at: Range<usize>, goes_on: bool, opened: bool) {
    match spans.last_mut() {
        Some(last) if goes_on && last.end == at.start => last.end = at.end,
        _ if goes_on || opened => spans.push(RawSpan {
            start: at.start,
            end: at.end,
            opens: !goes_on,
        }),
        _ => {}
    }
}

/// A string that cannot span lines ends with the line, closed or not.
fn end_line(state: &mut State) {
    use StringKind::{Double, Single};
    match state {
        State::InString(Double | Single) => *state = State::Normal,
        State::InDocAttribute(close, Some(Double | Single)) => {
            *state = State::InDocAttribute(close, None)
        }
        _ => {}
    }
}

/// Classify a single non-blank, non-shebang line given the current FSM state.
/// Updates `state` in place and returns the line classification.
fn classify_line(
    line: &str,
    state: &mut State,
    spec: &LanguageSpec,
    raw: &mut Vec<RawSpan>,
) -> LineKind {
    let mut has_code = matches!(state, State::InString(_));
    let mut has_comment = matches!(state, State::InBlockComment(_) | State::InDocAttribute(..));
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let was_raw = in_raw_string(state);
        let result = match &state {
            State::Normal => step_normal(&bytes[i..], spec, bytes, i),
            State::InString(kind) => step_in_string(&bytes[i..], kind),
            State::InBlockComment(depth) => step_in_block_comment(&bytes[i..], spec, *depth),
            State::InDocAttribute(close, inner) => {
                step_in_doc_attribute(&bytes[i..], spec, close, inner)
            }
        };
        has_code |= result.has_code;
        has_comment |= result.has_comment;
        if let Some(new_state) = result.new_state {
            *state = new_state;
        }
        note_raw(raw, i..i + result.advance, was_raw, in_raw_string(state));
        i += result.advance;
        if result.break_line {
            break;
        }
    }

    end_line(state);

    if has_code {
        LineKind::Code
    } else if has_comment {
        LineKind::Comment
    } else {
        LineKind::Blank
    }
}

/// Process all lines through the FSM, handing `each` the classification of
/// every line and the raw strings on it. Handles shebang detection on the
/// first line and blank line short-circuiting outside of block comments.
fn process_lines(
    lines: impl Iterator<Item = impl AsRef<str>>,
    spec: &LanguageSpec,
    mut each: impl FnMut(LineKind, Vec<RawSpan>),
) {
    let mut state = State::Normal;

    for (number, line) in lines.enumerate() {
        let line = line.as_ref();
        let mut raw = Vec::new();
        let kind = if number == 0 && line.starts_with("#!") {
            LineKind::Code
        } else if line.trim().is_empty()
            && !matches!(state, State::InBlockComment(_) | State::InDocAttribute(..))
        {
            LineKind::Blank
        } else {
            classify_line(line, &mut state, spec, &mut raw)
        };
        each(kind, raw);
    }
}

#[cfg(test)]
#[path = "counter_test.rs"]
mod tests;
