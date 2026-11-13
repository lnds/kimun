use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use super::fsm::{
    State, StepResult, StringKind, is_column_one_comment, step_in_block_comment,
    step_in_doc_attribute, step_in_string, step_normal,
};
use super::language::LanguageSpec;
use super::literals::heredoc::{Heredoc, terminator_end};
use super::literals::syntax::Syntax;
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

/// The lines of a reader without their line ends. Bytes that are not
/// UTF-8 are replaced, so a source in Latin-1 or Windows-1252 is read whole.
fn lossy_lines<R: BufRead>(reader: R) -> impl Iterator<Item = String> {
    reader.split(b'\n').map_while(Result::ok).map(|mut line| {
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        String::from_utf8_lossy(&line).into_owned()
    })
}

/// Classify each line of source code as Blank, Comment, or Code.
pub fn classify_reader<R: BufRead>(reader: R, spec: &LanguageSpec) -> Vec<LineKind> {
    let mut kinds = Vec::new();
    process_lines(lossy_lines(reader), spec, |kind| kinds.push(kind));
    kinds
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

/// A string that cannot span lines ends with the line, closed or not,
/// unless the line `goes_on` with a `\` at its end; a heredoc marked on the
/// line starts on the next.
fn end_line(state: &mut State, goes_on: bool, heredoc: Option<Heredoc>) {
    let ends = |kind: &StringKind| kind.ends_with_line(goes_on);
    match state {
        State::InString(kind) if ends(kind) => *state = State::Normal,
        State::InDocAttribute(_, inner) => drop(inner.take_if(|kind| ends(kind))),
        _ => {}
    }
    if let Some(heredoc) = heredoc
        && *state == State::Normal
    {
        *state = State::InHeredoc(heredoc);
    }
}

/// The line of a heredoc's text: code to its end, or the line of its
/// terminator, after which the line is read as any other. Gives where to
/// go on from, or `None` when the whole line is text.
fn heredoc_line(line: &str, state: &mut State) -> Option<usize> {
    let State::InHeredoc(heredoc) = state else {
        return Some(0);
    };
    let end = terminator_end(line, heredoc)?;
    *state = State::Normal;
    Some(end)
}

/// What a line holds, gathered step by step.
struct Seen {
    code: bool,
    comment: bool,
    heredoc: Option<Heredoc>,
}

impl Seen {
    /// What a line holds before its first step: code inside a string or
    /// after the terminator of a heredoc, comment inside a comment.
    fn at_start(state: &State, after_terminator: bool) -> Self {
        Self {
            code: after_terminator || matches!(state, State::InString(_)),
            comment: matches!(
                state,
                State::InBlockComment(..) | State::InDocAttribute(..) | State::InDocString(_)
            ),
            heredoc: None,
        }
    }

    fn add(&mut self, step: &mut StepResult) {
        self.code |= step.has_code;
        self.comment |= step.has_comment;
        self.heredoc = self.heredoc.take().or(step.heredoc.take());
    }

    fn kind(&self) -> LineKind {
        match (self.code, self.comment) {
            (true, _) => LineKind::Code,
            (false, true) => LineKind::Comment,
            (false, false) => LineKind::Blank,
        }
    }
}

/// One step of the FSM at `i`; none inside a heredoc, read whole apart.
fn step(
    state: &mut State,
    bytes: &[u8],
    i: usize,
    spec: &LanguageSpec,
    syntax: &Syntax,
) -> Option<StepResult> {
    let rest = &bytes[i..];
    Some(match state {
        State::Normal => step_normal(rest, spec, syntax, bytes, i),
        State::InString(kind) => step_in_string(rest, kind, false),
        State::InBlockComment(block, depth) => step_in_block_comment(rest, spec, *block, *depth),
        State::InDocAttribute(close, inner) => {
            step_in_doc_attribute(rest, spec, syntax, close, inner)
        }
        State::InDocString(kind) => step_in_string(rest, kind, true),
        State::InHeredoc(_) => return None,
    })
}

/// Classify a single non-blank, non-shebang line given the current FSM state.
/// Updates `state` in place and returns the line classification.
fn classify_line(line: &str, state: &mut State, spec: &LanguageSpec, syntax: &Syntax) -> LineKind {
    let Some(start) = heredoc_line(line, state) else {
        return LineKind::Code;
    };
    let mut seen = Seen::at_start(state, start > 0);
    let bytes = line.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        let Some(mut result) = step(state, bytes, i, spec, syntax) else {
            break;
        };
        seen.add(&mut result);
        if let Some(new_state) = result.new_state.take() {
            *state = new_state;
        }
        i += result.advance;
        if result.break_line {
            break;
        }
    }
    end_line(state, line.ends_with('\\'), seen.heredoc.take());
    seen.kind()
}

/// The kind of a line: a shebang on the first line is code, a blank line
/// outside a comment is blank, a comment in the first column is comment,
/// and any other line goes through the FSM.
fn line_kind(
    number: usize,
    line: &str,
    state: &mut State,
    spec: &LanguageSpec,
    syntax: &Syntax,
) -> LineKind {
    let in_comment = matches!(state, State::InBlockComment(..) | State::InDocAttribute(..));
    if number == 0 && line.starts_with("#!") {
        LineKind::Code
    } else if line.trim().is_empty() && !in_comment {
        LineKind::Blank
    } else if *state == State::Normal && is_column_one_comment(line, spec) {
        LineKind::Comment
    } else {
        classify_line(line, state, spec, syntax)
    }
}

/// Process all lines through the FSM, handing `each` the classification of
/// every line.
fn process_lines(
    lines: impl Iterator<Item = impl AsRef<str>>,
    spec: &LanguageSpec,
    mut each: impl FnMut(LineKind),
) {
    let mut state = State::Normal;
    let syntax = Syntax::of(spec);
    for (number, line) in lines.enumerate() {
        each(line_kind(number, line.as_ref(), &mut state, spec, &syntax));
    }
}

#[cfg(test)]
#[path = "counter_test.rs"]
mod tests;
