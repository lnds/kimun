use super::language::LanguageSpec;

#[derive(Debug, PartialEq)]
pub(super) enum StringKind {
    Double,
    Single,
    TripleDouble,
    TripleSingle,
}

#[derive(Debug, PartialEq)]
pub(super) enum State {
    Normal,
    InString(StringKind),
    InBlockComment(usize), // nesting depth
}

pub(super) struct StepResult {
    pub advance: usize,
    pub new_state: Option<State>,
    pub has_code: bool,
    pub has_comment: bool,
    pub break_line: bool,
}

fn bytes_start_with(haystack: &[u8], needle: &str) -> bool {
    haystack.starts_with(needle.as_bytes())
}

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

pub(super) fn step_normal(
    rest: &[u8],
    spec: &LanguageSpec,
    full_bytes: &[u8],
    pos: usize,
) -> StepResult {
    // Triple-quote strings (before regular quotes)
    if spec.triple_quote_strings {
        if rest.len() >= 3 && &rest[..3] == b"\"\"\"" {
            return StepResult {
                advance: 3,
                new_state: Some(State::InString(StringKind::TripleDouble)),
                has_code: true,
                has_comment: false,
                break_line: false,
            };
        }
        if spec.single_quote_strings && rest.len() >= 3 && &rest[..3] == b"'''" {
            return StepResult {
                advance: 3,
                new_state: Some(State::InString(StringKind::TripleSingle)),
                has_code: true,
                has_comment: false,
                break_line: false,
            };
        }
    }

    // Pragma (e.g. Haskell {-# ... #-}) — must check before block comment
    if let Some((popen, pclose)) = spec.pragma
        && bytes_start_with(rest, popen)
    {
        let end = skip_pragma(full_bytes, pos + popen.len(), pclose);
        return StepResult {
            advance: end - pos,
            new_state: None,
            has_code: true,
            has_comment: false,
            break_line: false,
        };
    }

    // Block comment open
    if let Some((open, _)) = spec.block_comment
        && bytes_start_with(rest, open)
    {
        return StepResult {
            advance: open.len(),
            new_state: Some(State::InBlockComment(1)),
            has_code: false,
            has_comment: true,
            break_line: false,
        };
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
        return StepResult {
            advance: 0,
            new_state: None,
            has_code: false,
            has_comment: true,
            break_line: true,
        };
    }

    let ch = rest[0];

    // Double-quote string
    if ch == b'"' {
        return StepResult {
            advance: 1,
            new_state: Some(State::InString(StringKind::Double)),
            has_code: true,
            has_comment: false,
            break_line: false,
        };
    }

    // Single-quote string
    if spec.single_quote_strings && ch == b'\'' {
        return StepResult {
            advance: 1,
            new_state: Some(State::InString(StringKind::Single)),
            has_code: true,
            has_comment: false,
            break_line: false,
        };
    }

    StepResult {
        advance: 1,
        new_state: None,
        has_code: !ch.is_ascii_whitespace(),
        has_comment: false,
        break_line: false,
    }
}

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
                return StepResult {
                    advance: 3,
                    new_state: Some(State::Normal),
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
        }
        StringKind::TripleSingle => {
            if rest.len() >= 3 && &rest[..3] == b"'''" {
                return StepResult {
                    advance: 3,
                    new_state: Some(State::Normal),
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
        }
        StringKind::Double => {
            if ch == b'\\' {
                return StepResult {
                    advance: (pos + 2).min(len) - pos,
                    new_state: None,
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
            if ch == b'"' {
                return StepResult {
                    advance: 1,
                    new_state: Some(State::Normal),
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
        }
        StringKind::Single => {
            if ch == b'\\' {
                return StepResult {
                    advance: (pos + 2).min(len) - pos,
                    new_state: None,
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
            if ch == b'\'' {
                return StepResult {
                    advance: 1,
                    new_state: Some(State::Normal),
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
        }
    }
    StepResult {
        advance: 1,
        new_state: None,
        has_code: true,
        has_comment: false,
        break_line: false,
    }
}

pub(super) fn step_in_block_comment(rest: &[u8], spec: &LanguageSpec, depth: usize) -> StepResult {
    // Check for nested open (before checking close)
    if spec.nested_block_comments
        && let Some((open, _)) = spec.block_comment
        && bytes_start_with(rest, open)
    {
        return StepResult {
            advance: open.len(),
            new_state: Some(State::InBlockComment(depth + 1)),
            has_code: false,
            has_comment: true,
            break_line: false,
        };
    }

    if let Some((_, close)) = spec.block_comment
        && bytes_start_with(rest, close)
    {
        let new_state = if depth <= 1 {
            State::Normal
        } else {
            State::InBlockComment(depth - 1)
        };
        return StepResult {
            advance: close.len(),
            new_state: Some(new_state),
            has_code: false,
            has_comment: true,
            break_line: false,
        };
    }

    StepResult {
        advance: 1,
        new_state: None,
        has_code: false,
        has_comment: true,
        break_line: false,
    }
}
