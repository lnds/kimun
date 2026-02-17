/// Finite state machine steps for source line classification.
///
/// Implements the core character-by-character logic for detecting
/// comments, strings, and pragmas. Called by `counter.rs` which
/// drives the FSM line by line.
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
/// 1. Triple-quote strings (Python `"""` / `'''`)
/// 2. Pragmas (Haskell `{-# ... #-}`) — before block comments
/// 3. Block comment open (`/*`, `{-`)
/// 4. Line comments (`//`, `--`, `#`) with `not_before` guard
/// 5. String delimiters (`"`, `'`)
/// 6. Regular character (code if non-whitespace)
pub(super) fn step_normal(
    rest: &[u8],
    spec: &LanguageSpec,
    full_bytes: &[u8],
    pos: usize,
) -> StepResult {
    // Triple-quote strings (before regular quotes)
    if spec.triple_quote_strings {
        if rest.len() >= 3 && &rest[..3] == b"\"\"\"" {
            return StepResult::code(3, Some(State::InString(StringKind::TripleDouble)));
        }
        if spec.single_quote_strings && rest.len() >= 3 && &rest[..3] == b"'''" {
            return StepResult::code(3, Some(State::InString(StringKind::TripleSingle)));
        }
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

    let ch = rest[0];

    // Double-quote string
    if ch == b'"' {
        return StepResult::code(1, Some(State::InString(StringKind::Double)));
    }

    // Single-quote string
    if spec.single_quote_strings && ch == b'\'' {
        return StepResult::code(1, Some(State::InString(StringKind::Single)));
    }

    StepResult {
        advance: 1,
        new_state: None,
        has_code: !ch.is_ascii_whitespace(),
        has_comment: false,
        break_line: false,
    }
}

/// Process one byte inside a string literal. Handles escape sequences for
/// single/double quotes and closing delimiters for triple-quote strings.
/// All content inside strings is classified as code.
pub(super) fn step_in_string(
    rest: &[u8],
    ch: u8,
    kind: &StringKind,
    len: usize,
    pos: usize,
) -> StepResult {
    match kind {
        StringKind::TripleDouble => {
            if rest.len() >= 3 && &rest[..3] == b"\"\"\"" {
                return StepResult::code(3, Some(State::Normal));
            }
        }
        StringKind::TripleSingle => {
            if rest.len() >= 3 && &rest[..3] == b"'''" {
                return StepResult::code(3, Some(State::Normal));
            }
        }
        StringKind::Double => {
            if ch == b'\\' {
                return StepResult::code((pos + 2).min(len) - pos, None);
            }
            if ch == b'"' {
                return StepResult::code(1, Some(State::Normal));
            }
        }
        StringKind::Single => {
            if ch == b'\\' {
                return StepResult::code((pos + 2).min(len) - pos, None);
            }
            if ch == b'\'' {
                return StepResult::code(1, Some(State::Normal));
            }
        }
    }
    StepResult::code(1, None)
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
