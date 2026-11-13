//! Finite state machine steps for source line classification.
//!
//! Implements the core character-by-character logic for detecting
//! comments, strings, and pragmas. Called by `counter.rs` which
//! drives the FSM line by line. Each `step_*` function examines the
//! current byte position and returns a `StepResult` indicating how
//! many bytes to advance, any state transition, and whether the
//! current line contains code or comment content.
use super::language::LanguageSpec;
use super::literals::character_len;
use super::literals::heredoc::{Heredoc, heredoc_at};
use super::literals::syntax::Syntax;
pub(super) use super::strings::StringKind;
use super::strings::{literal_open, scan};

/// The opening and closing delimiters of a block comment.
pub(super) type Block = (&'static str, &'static str);

/// FSM state for the line classifier. Tracks whether we are in normal code,
/// inside a string literal, or inside a (possibly nested) block comment.
#[derive(Debug, PartialEq)]
pub(super) enum State {
    Normal,
    InString(StringKind),
    /// Inside a block comment, with its delimiters and nesting depth.
    InBlockComment(Block, usize),
    /// Inside a documentation attribute (Kaikai `#[doc( … )]`), with the
    /// delimiter that closes it. The payload also tracks the string literal
    /// being read, so that the closing `)]` is only recognised outside a
    /// string.
    InDocAttribute(&'static str, Option<StringKind>),
    /// Inside a triple-quoted string that counts as comment (Python's
    /// docstrings, as cloc counts them), with the kind that closes it.
    InDocString(StringKind),
    /// Inside the text of a heredoc, which ends at a line holding its
    /// terminator.
    InHeredoc(Heredoc),
}

/// Result of processing one FSM step: how many bytes to advance, optional
/// state transition, and flags for code/comment presence on the current line.
pub(super) struct StepResult {
    pub advance: usize,
    pub new_state: Option<State>,
    pub has_code: bool,
    pub has_comment: bool,
    pub break_line: bool,
    /// A heredoc whose text starts on the next line.
    pub heredoc: Option<Heredoc>,
}

impl StepResult {
    /// Advance past code content (non-comment, non-whitespace).
    fn code(advance: usize, new_state: Option<State>) -> Self {
        Self {
            advance,
            new_state,
            has_code: true,
            has_comment: false,
            break_line: false,
            heredoc: None,
        }
    }

    /// Advance past comment content (block comment body or delimiter).
    fn comment(advance: usize, new_state: Option<State>) -> Self {
        Self {
            advance,
            new_state,
            has_code: false,
            has_comment: true,
            break_line: false,
            heredoc: None,
        }
    }

    /// Signal that a line comment was found — mark as comment and stop processing.
    fn line_comment() -> Self {
        Self {
            advance: 0,
            new_state: None,
            has_code: false,
            has_comment: true,
            break_line: true,
            heredoc: None,
        }
    }
}

/// Check if a byte slice starts with a string pattern.
fn bytes_start_with(haystack: &[u8], needle: &str) -> bool {
    haystack.starts_with(needle.as_bytes())
}

/// Scan forward from `start` to find the pragma closing delimiter, returning
/// the position just past it. If no closing delimiter is found, returns the
/// end of the byte slice (treating the rest of the line as pragma content).
fn skip_pragma(bytes: &[u8], start: usize, pclose: &str) -> usize {
    bytes[start..]
        .windows(pclose.len())
        .position(|window| window == pclose.as_bytes())
        .map_or(bytes.len(), |at| start + at + pclose.len())
}

/// Process one byte in Normal state. Detection priority:
///
/// 1. **Triple-quote strings** (Python `"""` / `'''`) — checked before
///    regular quotes so that `"""` is not parsed as empty string + quote.
/// 2. **Pragmas** (Haskell `{-# ... #-}`) — must come before block
///    comments because `{-#` is a prefix of `{-`. Without this ordering,
///    pragmas would be misclassified as block comments.
/// 3. **Block comment open** (`/*`, `{-`) — transitions to InBlockComment.
/// 4. **Line comments** (`//`, `--`, `#`) with `not_before` guard — the
///    guard prevents Haskell's `-->` operator from matching as `--` comment.
/// 5. **String delimiters** (`"`, `'`) — single-quote strings are opt-in
///    via `single_quote_strings` to avoid Rust lifetime annotations (`'a`)
///    being parsed as string delimiters.
/// 6. **Regular character** — code if non-whitespace, ignored otherwise.
pub(super) fn step_normal(
    rest: &[u8],
    spec: &LanguageSpec,
    syntax: &Syntax,
    full_bytes: &[u8],
    pos: usize,
) -> StepResult {
    let at = At {
        bytes: full_bytes,
        pos,
    };
    literal_step(at, spec, syntax)
        .or_else(|| comment_step(at, spec, syntax))
        .unwrap_or_else(|| StepResult {
            advance: 1,
            new_state: None,
            has_code: !rest[0].is_ascii_whitespace(),
            has_comment: false,
            break_line: false,
            heredoc: None,
        })
}

/// A position on a line.
#[derive(Clone, Copy)]
struct At<'l> {
    bytes: &'l [u8],
    pos: usize,
}

impl<'l> At<'l> {
    fn rest(&self) -> &'l [u8] {
        &self.bytes[self.pos..]
    }
}

/// The step of a literal that starts at `pos`: a string (triple quotes are
/// matched before the single-byte ones, so `"""` never reads as an empty
/// string followed by a quote), a quote that opens none, or the marker of a
/// heredoc.
fn literal_step(at: At, spec: &LanguageSpec, syntax: &Syntax) -> Option<StepResult> {
    let (rest, full_bytes, pos) = (at.rest(), at.bytes, at.pos);
    if let Some((width, kind)) = literal_open(full_bytes, pos, spec, syntax) {
        return Some(open_literal(width, kind, spec));
    }
    if let Some(width) = quote_as_code(rest, syntax, full_bytes, pos) {
        return Some(StepResult::code(width, None));
    }
    let (heredoc, width) = heredoc_at(full_bytes, pos, syntax.heredocs)?;
    let mut step = StepResult::code(width, None);
    step.heredoc = Some(heredoc);
    Some(step)
}

/// The step of what opens at `pos` among a documentation attribute (before
/// line comments, as Kaikai's `#[doc(` starts with `#`), a pragma (before
/// block comments, as `{-#` starts with `{-`), a block comment and a line
/// comment.
fn comment_step(at: At, spec: &LanguageSpec, syntax: &Syntax) -> Option<StepResult> {
    let (rest, full_bytes, pos) = (at.rest(), at.bytes, at.pos);
    if let Some((open, close)) = spec
        .doc_attributes
        .iter()
        .find(|(open, _)| bytes_start_with(rest, open))
    {
        return Some(StepResult::comment(
            open.len(),
            Some(State::InDocAttribute(close, None)),
        ));
    }
    if let Some((popen, pclose)) = spec.pragma
        && bytes_start_with(rest, popen)
    {
        let end = skip_pragma(full_bytes, pos + popen.len(), pclose);
        return Some(StepResult::code(end - pos, None));
    }
    let mut blocks = spec
        .block_comment
        .into_iter()
        .chain(spec.other_block_comment);
    if let Some(block) = blocks.find(|(open, _)| bytes_start_with(rest, open)) {
        return Some(StepResult::comment(
            block.0.len(),
            Some(State::InBlockComment(block, 1)),
        ));
    }
    let comments = spec.line_comments.iter();
    let mut markers = comments.filter(|marker| bytes_start_with(rest, marker));
    markers
        .any(|marker| opens_line_comment(rest, marker, spec, syntax.hash_is_code(full_bytes, pos)))
        .then(StepResult::line_comment)
}

/// Whether `marker`, which `rest` starts with, opens a line comment: not
/// when a character that makes an operator of it follows (Haskell's `-->`),
/// when a directive follows, nor for a `#` that is code.
fn opens_line_comment(rest: &[u8], marker: &str, spec: &LanguageSpec, hash_is_code: bool) -> bool {
    let after = &rest[marker.len()..];
    let operator = after
        .first()
        .is_some_and(|b| spec.line_comment_not_before.as_bytes().contains(b));
    !(operator || is_directive(after, spec) || (marker == "#" && hash_is_code))
}

/// The width of a quote that opens no string: a character literal where
/// `'` opens no string (`'"'`), the quote of a lifetime or a label, or the
/// code of a quote character (`?"`).
fn quote_as_code(rest: &[u8], syntax: &Syntax, full_bytes: &[u8], pos: usize) -> Option<usize> {
    if rest[0] == b'\'' && !syntax.single_quotes {
        return Some(1 + character_len(&rest[1..]));
    }
    syntax.char_code_len(full_bytes, pos)
}

/// The step that opens a string literal: code, or comment for a
/// triple-quoted string in a language that counts those as comment.
fn open_literal(width: usize, kind: StringKind, spec: &LanguageSpec) -> StepResult {
    if kind.is_triple() && spec.triple_quote_comments {
        StepResult::comment(width, Some(State::InDocString(kind)))
    } else {
        StepResult::code(width, Some(State::InString(kind)))
    }
}

/// Whether what follows a comment marker makes a directive of it.
pub(super) fn is_directive(rest: &[u8], spec: &LanguageSpec) -> bool {
    spec.directives.iter().any(|d| {
        rest.get(..d.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(d.as_bytes()))
    })
}

/// Whether the first column makes a comment of the whole line.
pub(super) fn is_column_one_comment(line: &str, spec: &LanguageSpec) -> bool {
    match line.as_bytes() {
        [first, rest @ ..] => {
            spec.column_one_comments.as_bytes().contains(first) && !is_directive(rest, spec)
        }
        [] => false,
    }
}

/// Process one byte inside a string literal: code, or comment inside a
/// triple-quoted string a language counts as comment (`doc`).
pub(super) fn step_in_string(rest: &[u8], kind: &mut StringKind, doc: bool) -> StepResult {
    let (advance, closed) = scan(rest, kind);
    let new_state = closed.then_some(State::Normal);
    if doc {
        StepResult::comment(advance, new_state)
    } else {
        StepResult::code(advance, new_state)
    }
}

/// Process one byte inside a documentation attribute. The body is
/// documentation, so every byte counts as comment. String literals are
/// tracked because the closing delimiter (`)]`) is only meaningful outside
/// a string — a doc text may well contain `)]` itself.
pub(super) fn step_in_doc_attribute(
    rest: &[u8],
    spec: &LanguageSpec,
    syntax: &Syntax,
    close: &'static str,
    inner: &mut Option<StringKind>,
) -> StepResult {
    if let Some(kind) = inner {
        let (advance, closed) = scan(rest, kind);
        return StepResult::comment(
            advance,
            closed.then_some(State::InDocAttribute(close, None)),
        );
    }

    if bytes_start_with(rest, close) {
        return StepResult::comment(close.len(), Some(State::Normal));
    }

    match literal_open(rest, 0, spec, syntax) {
        Some((width, kind)) => {
            StepResult::comment(width, Some(State::InDocAttribute(close, Some(kind))))
        }
        None => StepResult::comment(1, None),
    }
}

/// Process one byte inside a block comment. Tracks nesting depth when
/// `nested_block_comments` is enabled (Rust, Haskell). Checks for nested
/// open before close to correctly handle `/* /* */ */` patterns.
pub(super) fn step_in_block_comment(
    rest: &[u8],
    spec: &LanguageSpec,
    block: Block,
    depth: usize,
) -> StepResult {
    let (open, close) = block;
    // Check for nested open (before checking close)
    if spec.nested_block_comments && bytes_start_with(rest, open) {
        return StepResult::comment(open.len(), Some(State::InBlockComment(block, depth + 1)));
    }

    if bytes_start_with(rest, close) {
        let new_state = if depth <= 1 {
            State::Normal
        } else {
            State::InBlockComment(block, depth - 1)
        };
        return StepResult::comment(close.len(), Some(new_state));
    }

    StepResult::comment(1, None)
}
