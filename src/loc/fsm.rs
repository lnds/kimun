//! Finite state machine steps for source line classification.
//!
//! Implements the core character-by-character logic for detecting
//! comments, strings, and pragmas. Called by `counter.rs` which
//! drives the FSM line by line. Each `step_*` function examines the
//! current byte position and returns a `StepResult` indicating how
//! many bytes to advance, any state transition, and whether the
//! current line contains code or comment content.
use super::language::LanguageSpec;

/// The kind of string literal currently being parsed.
#[derive(Debug, PartialEq)]
pub(super) enum StringKind {
    Double,
    Single,
    TripleDouble,
    TripleSingle,
}

/// FSM state for the line classifier. Tracks whether we are in normal code,
/// inside a string literal, or inside a (possibly nested) block comment.
#[derive(Debug, PartialEq)]
pub(super) enum State {
    Normal,
    InString(StringKind),
    InBlockComment(usize), // nesting depth
    /// Inside a documentation attribute (Kaikai `#[doc( … )]`). The payload
    /// tracks the string literal being read, so that the closing `)]` is only
    /// recognised outside a string.
    InDocAttribute(Option<StringKind>),
}

/// Result of processing one FSM step: how many bytes to advance, optional
/// state transition, and flags for code/comment presence on the current line.
pub(super) struct StepResult {
    pub advance: usize,
    pub new_state: Option<State>,
    pub has_code: bool,
    pub has_comment: bool,
    pub break_line: bool,
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
    let len = bytes.len();
    let mut i = start;
    while i < len {
        if bytes_start_with(&bytes[i..], pclose) {
            return i + pclose.len();
        }
        i += 1;
    }
    i
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
    full_bytes: &[u8],
    pos: usize,
) -> StepResult {
    // String delimiters — triple quotes are matched before the single-byte
    // ones, so `"""` never reads as an empty string followed by a quote.
    if let Some((width, kind)) = string_open(rest, spec) {
        return StepResult::code(width, Some(State::InString(kind)));
    }

    // Documentation attribute (e.g. Kaikai `#[doc(`) — before the line comment
    // check, since the marker starts with the `#` comment character.
    if let Some((open, _)) = spec.doc_attribute
        && bytes_start_with(rest, open)
    {
        return StepResult::comment(open.len(), Some(State::InDocAttribute(None)));
    }

    // Pragma (e.g. Haskell {-# ... #-}) — must check before block comment
    if let Some((popen, pclose)) = spec.pragma
        && bytes_start_with(rest, popen)
    {
        let end = skip_pragma(full_bytes, pos + popen.len(), pclose);
        return StepResult::code(end - pos, None);
    }

    // Block comment open
    if let Some((open, _)) = spec.block_comment
        && bytes_start_with(rest, open)
    {
        return StepResult::comment(open.len(), Some(State::InBlockComment(1)));
    }

    // Line comment
    let is_line_comment = spec.line_comments.iter().any(|lc| {
        if !bytes_start_with(rest, lc) {
            return false;
        }
        if !spec.line_comment_not_before.is_empty()
            && let Some(&next_byte) = rest.get(lc.len())
            && spec.line_comment_not_before.as_bytes().contains(&next_byte)
        {
            return false;
        }
        true
    });
    if is_line_comment {
        return StepResult::line_comment();
    }

    StepResult {
        advance: 1,
        new_state: None,
        has_code: !rest[0].is_ascii_whitespace(),
        has_comment: false,
        break_line: false,
    }
}

/// The delimiter of a string kind and how many bytes it spans.
fn string_delimiter(kind: &StringKind) -> &'static [u8] {
    match kind {
        StringKind::TripleDouble => b"\"\"\"",
        StringKind::TripleSingle => b"'''",
        StringKind::Double => b"\"",
        StringKind::Single => b"'",
    }
}

/// Detect a string literal opening at `rest`, honouring the spec's opt-in
/// flags. Triple quotes win over single-byte ones so `"""` is not read as an
/// empty string. Returns the delimiter width and the kind that was opened.
fn string_open(rest: &[u8], spec: &LanguageSpec) -> Option<(usize, StringKind)> {
    let candidates = [
        (spec.triple_quote_strings, StringKind::TripleDouble),
        (
            spec.triple_quote_strings && spec.single_quote_strings,
            StringKind::TripleSingle,
        ),
        (true, StringKind::Double),
        (spec.single_quote_strings, StringKind::Single),
    ];
    candidates.into_iter().find_map(|(enabled, kind)| {
        let delim = string_delimiter(&kind);
        (enabled && rest.starts_with(delim)).then_some((delim.len(), kind))
    })
}

/// Scan one byte inside a string literal: how far to advance, and whether the
/// literal closes here. Escapes are only honoured in single-byte-delimited
/// strings; a triple-quoted body ends at its delimiter.
fn string_scan(rest: &[u8], kind: &StringKind) -> (usize, bool) {
    let delim = string_delimiter(kind);
    if rest.starts_with(delim) {
        return (delim.len(), true);
    }
    if delim.len() == 1 && rest[0] == b'\\' {
        return (rest.len().min(2), false);
    }
    (1, false)
}

/// Process one byte inside a string literal. All content inside strings is
/// classified as code.
pub(super) fn step_in_string(rest: &[u8], kind: &StringKind) -> StepResult {
    let (advance, closed) = string_scan(rest, kind);
    StepResult::code(advance, closed.then_some(State::Normal))
}

/// Process one byte inside a documentation attribute. The body is
/// documentation, so every byte counts as comment. String literals are
/// tracked because the closing delimiter (`)]`) is only meaningful outside
/// a string — a doc text may well contain `)]` itself.
pub(super) fn step_in_doc_attribute(
    rest: &[u8],
    spec: &LanguageSpec,
    inner: &Option<StringKind>,
) -> StepResult {
    if let Some(kind) = inner {
        let (advance, closed) = string_scan(rest, kind);
        return StepResult::comment(advance, closed.then_some(State::InDocAttribute(None)));
    }

    let close = spec.doc_attribute.map(|(_, c)| c).unwrap_or(")]");
    if bytes_start_with(rest, close) {
        return StepResult::comment(close.len(), Some(State::Normal));
    }

    match string_open(rest, spec) {
        Some((width, kind)) => StepResult::comment(width, Some(State::InDocAttribute(Some(kind)))),
        None => StepResult::comment(1, None),
    }
}

/// Process one byte inside a block comment. Tracks nesting depth when
/// `nested_block_comments` is enabled (Rust, Haskell). Checks for nested
/// open before close to correctly handle `/* /* */ */` patterns.
pub(super) fn step_in_block_comment(rest: &[u8], spec: &LanguageSpec, depth: usize) -> StepResult {
    // Check for nested open (before checking close)
    if spec.nested_block_comments
        && let Some((open, _)) = spec.block_comment
        && bytes_start_with(rest, open)
    {
        return StepResult::comment(open.len(), Some(State::InBlockComment(depth + 1)));
    }

    if let Some((_, close)) = spec.block_comment
        && bytes_start_with(rest, close)
    {
        let new_state = if depth <= 1 {
            State::Normal
        } else {
            State::InBlockComment(depth - 1)
        };
        return StepResult::comment(close.len(), Some(new_state));
    }

    StepResult::comment(1, None)
}
