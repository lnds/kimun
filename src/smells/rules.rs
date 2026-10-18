//! Pure detection functions for each code smell.

use crate::detection::{FunctionDetectionMarkers, detect_function_bodies};
use crate::loc::counter::LineKind;

use super::analyzer::{SmellInstance, SmellKind};

/// Detect functions longer than `max_lines` code lines.
///
/// The count excludes the function signature line(s) and the closing brace/end,
/// so `--max-lines 50` means "50 lines of body code".
pub fn detect_long_functions(
    lines: &[String],
    kinds: &[LineKind],
    markers: &dyn FunctionDetectionMarkers,
    max_lines: usize,
) -> Vec<SmellInstance> {
    let code_lines = crate::detection::code_lines(lines, kinds, markers);

    let functions = detect_function_bodies(lines, &code_lines, markers);
    let mut smells = Vec::new();

    for func in &functions {
        let total = func.code_lines.len();
        // Subtract signature (first line) and closing brace (last line) for
        // brace-scoped languages. For indent-scoped, subtract only the signature.
        let overhead = if markers.brace_scoped() {
            2.min(total)
        } else {
            1.min(total)
        };
        let body_len = total.saturating_sub(overhead);
        if body_len > max_lines {
            smells.push(SmellInstance {
                kind: SmellKind::LongFunction,
                line: func.start_line,
                detail: format!(
                    "function `{}` has {} lines (max {})",
                    func.name, body_len, max_lines
                ),
            });
        }
    }

    smells
}

/// Debt markers matched case-insensitively — these are jargon tokens unlikely
/// to appear inside natural-English prose (nobody writes "my todo list" or
/// "the fixme approach" or "an xxx-rated joke" in a code comment about
/// production behavior).
const DEBT_KEYWORDS_CI: &[&str] = &["TODO", "FIXME", "HACK", "XXX"];

/// Debt markers matched case-sensitively (uppercase only) — `BUG` is a common
/// English word (`a bug report`, `a buggy release`), so case-folding produces
/// false positives on comments that discuss bugs in prose rather than mark
/// technical debt. Canonical debt-marker usage is `# BUG: ...` in uppercase.
const DEBT_KEYWORDS_CS: &[&str] = &["BUG"];

/// Detect TODO/FIXME debt in comment lines.
///
/// Matches whole-word debt-marker tokens. TODO/FIXME/HACK/XXX are matched
/// case-insensitively; BUG is matched case-sensitively (uppercase only)
/// because it's a common English word. In both branches the match uses
/// whole-word boundaries, so `debugging`, `todolist`, `hackathon`, `buggy`
/// don't fire.
pub fn detect_todo_debt(lines: &[String], kinds: &[LineKind]) -> Vec<SmellInstance> {
    let mut smells = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if kinds.get(i) != Some(&LineKind::Comment) {
            continue;
        }
        let upper = line.to_uppercase();
        let mut matched: Option<&&str> = None;
        for kw in DEBT_KEYWORDS_CI {
            if contains_word(&upper, kw) {
                matched = Some(kw);
                break;
            }
        }
        if matched.is_none() {
            for kw in DEBT_KEYWORDS_CS {
                if contains_word(line, kw) {
                    matched = Some(kw);
                    break;
                }
            }
        }
        if let Some(kw) = matched {
            smells.push(SmellInstance {
                kind: SmellKind::TodoDebt,
                line: i + 1,
                detail: format!("{kw} comment"),
            });
        }
    }

    smells
}

/// Whole-word substring match: returns true when `needle` appears in `haystack`
/// with both boundaries being non-word chars (or string ends). "Word char"
/// here is ASCII alphanumeric or `_`, matching the common regex `\b` behavior.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let nlen = needle.len();
    if nlen == 0 || nlen > bytes.len() {
        return false;
    }
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let needle_bytes = needle.as_bytes();
    let mut i = 0;
    while i + nlen <= bytes.len() {
        if &bytes[i..i + nlen] == needle_bytes {
            let left_ok = i == 0 || !is_word(bytes[i - 1]);
            let right_ok = i + nlen == bytes.len() || !is_word(bytes[i + nlen]);
            if left_ok && right_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
#[path = "rules_test.rs"]
mod tests;
