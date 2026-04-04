//! Pure detection functions for each code smell.

use crate::detection::{FunctionDetectionMarkers, detect_function_bodies};
use crate::loc::counter::LineKind;
use crate::util::mask_strings;

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
    let code_lines: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| kinds.get(*i) == Some(&LineKind::Code))
        .map(|(i, l)| (i, l.as_str()))
        .collect();

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

/// Detect functions with more parameters than `max_params`.
///
/// Joins multiple lines of the function signature (until the closing paren)
/// to handle multi-line parameter lists, which is the canonical formatting
/// style in Rust, Go, Python, etc.
pub fn detect_long_params(
    lines: &[String],
    kinds: &[LineKind],
    markers: &dyn FunctionDetectionMarkers,
    max_params: usize,
) -> Vec<SmellInstance> {
    let code_lines: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| kinds.get(*i) == Some(&LineKind::Code))
        .map(|(i, l)| (i, l.as_str()))
        .collect();

    let functions = detect_function_bodies(lines, &code_lines, markers);
    let mut smells = Vec::new();

    for func in &functions {
        if func.code_lines.is_empty() {
            continue;
        }

        // Join code lines until we find the closing paren of the signature.
        let signature = collect_signature(func, markers.line_comments());
        let Some(signature) = signature else {
            continue;
        };

        // Extract content between first ( and matching )
        let Some(open) = signature.find('(') else {
            continue;
        };
        let after_open = &signature[open + 1..];
        let close = find_matching_paren(after_open);
        if close >= after_open.len() {
            // No closing paren found — malformed signature, skip
            continue;
        }
        // Trim and strip trailing comma (common in multi-line style)
        let params_str = after_open[..close].trim().trim_end_matches(',').trim();

        if params_str.is_empty() {
            continue;
        }

        let param_count = params_str.matches(',').count() + 1;
        if param_count > max_params {
            smells.push(SmellInstance {
                kind: SmellKind::LongParameterList,
                line: func.start_line,
                detail: format!(
                    "function `{}` has {} params (max {})",
                    func.name, param_count, max_params
                ),
            });
        }
    }

    smells
}

/// Collect the function signature by joining code lines until the parameter
/// list is closed (matching paren found). Returns `None` if no `(` is found.
fn collect_signature(
    func: &crate::detection::FunctionBody<'_>,
    line_comments: &[&str],
) -> Option<String> {
    let mut sig = String::new();
    for &(_, code_line) in &func.code_lines {
        let masked = mask_strings(code_line, line_comments);
        if sig.is_empty() && !masked.contains('(') {
            continue;
        }
        sig.push_str(&masked);
        sig.push(' ');

        // Check if we've closed the paren
        if let Some(open) = sig.find('(') {
            let after = &sig[open + 1..];
            let close = find_matching_paren(after);
            if close < after.len() {
                return Some(sig);
            }
        }
    }
    if sig.is_empty() { None } else { Some(sig) }
}

/// Find the position of the matching closing paren, handling nesting.
/// Returns `s.len()` if no matching paren is found.
fn find_matching_paren(s: &str) -> usize {
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return i;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    s.len()
}

/// TODO/FIXME/HACK/XXX/BUG keywords in comment lines.
const DEBT_KEYWORDS: &[&str] = &["TODO", "FIXME", "HACK", "XXX", "BUG"];

/// Detect TODO/FIXME debt in comment lines.
pub fn detect_todo_debt(lines: &[String], kinds: &[LineKind]) -> Vec<SmellInstance> {
    let mut smells = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if kinds.get(i) != Some(&LineKind::Comment) {
            continue;
        }
        let upper = line.to_uppercase();
        for &kw in DEBT_KEYWORDS {
            if upper.contains(kw) {
                smells.push(SmellInstance {
                    kind: SmellKind::TodoDebt,
                    line: i + 1,
                    detail: format!("{kw} comment"),
                });
                break;
            }
        }
    }

    smells
}

/// Declaration keywords that exclude a line from magic number detection.
/// Matched against the first token of the line (case-insensitive).
const DECL_KEYWORDS: &[&str] = &["const", "let", "static", "final", "val", "#define", "enum"];

/// Trivial numeric values that are not considered magic numbers.
const TRIVIAL_NUMBERS: &[&str] = &["0", "1", "2", "-1"];

/// Check whether the first token of a line is a declaration keyword.
fn is_declaration_line(trimmed: &str) -> bool {
    let lower = trimmed.to_lowercase();
    let first_token = lower.split_whitespace().next().unwrap_or("");
    DECL_KEYWORDS.contains(&first_token)
}

/// Check whether a byte is part of a numeric literal (digit, hex letter, dot, prefix char).
fn is_numeric_char(b: u8) -> bool {
    b.is_ascii_digit()
        || b == b'.'
        || b == b'x'
        || b == b'X'
        || b == b'b'
        || b == b'B'
        || b == b'o'
        || b == b'O'
        || (b'a'..=b'f').contains(&b)
        || (b'A'..=b'F').contains(&b)
        || b == b'e'
        || b == b'E'
}

/// Detect bare numeric literals in code lines (excluding declarations and trivial values).
///
/// Recognizes decimal, hexadecimal (0x), binary (0b), octal (0o), and
/// floating-point with scientific notation (e.g. 3.14e-10).
pub fn detect_magic_numbers(
    lines: &[String],
    kinds: &[LineKind],
    line_comments: &[&str],
) -> Vec<SmellInstance> {
    let mut smells = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if kinds.get(i) != Some(&LineKind::Code) {
            continue;
        }

        let masked = mask_strings(line, line_comments);
        let trimmed = masked.trim();

        if is_declaration_line(trimmed) {
            continue;
        }

        if has_magic_number(trimmed) {
            smells.push(SmellInstance {
                kind: SmellKind::MagicNumber,
                line: i + 1,
                detail: "magic number in code".to_string(),
            });
        }
    }

    smells
}

/// Scan a masked code line for a bare numeric literal that isn't trivial.
fn has_magic_number(trimmed: &str) -> bool {
    let bytes = trimmed.as_bytes();
    let mut j = 0;

    while j < bytes.len() {
        // Look for a digit or a minus followed by a digit
        let is_neg = bytes[j] == b'-'
            && j + 1 < bytes.len()
            && bytes[j + 1].is_ascii_digit()
            && (j == 0 || !bytes[j - 1].is_ascii_alphanumeric());
        let start = j;
        if is_neg {
            j += 1;
        }
        if j < bytes.len() && bytes[j].is_ascii_digit() {
            // Skip if preceded by identifier char (e.g., var2)
            if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
                while j < bytes.len() && is_numeric_char(bytes[j]) {
                    j += 1;
                }
                continue;
            }

            // Collect the full numeric literal (digits, dots, hex chars, exponent)
            while j < bytes.len() && is_numeric_char(bytes[j]) {
                // Handle exponent sign: e+, e-, E+, E-
                if (bytes[j] == b'e' || bytes[j] == b'E')
                    && j + 1 < bytes.len()
                    && (bytes[j + 1] == b'+' || bytes[j + 1] == b'-')
                {
                    j += 2; // skip e and sign
                    continue;
                }
                j += 1;
            }
            // Skip if followed by identifier char (part of identifier)
            if j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
                continue;
            }

            let num_str = &trimmed[start..j];
            let num_clean = num_str.trim_end_matches('.');
            if !TRIVIAL_NUMBERS.contains(&num_clean) {
                return true;
            }
        } else {
            j += 1;
        }
    }

    false
}

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

/// Detect consecutive comment lines that look like commented-out code.
///
/// A comment line is considered "code-like" if it contains at least 1 strong
/// pattern plus 1 more pattern (strong or weak), totaling >= 2 pattern hits
/// with at least one being strong. This avoids false positives on doc comments
/// like `/// use std::io::Read;` which only match weak patterns.
///
/// Triggers when >= 2 consecutive code-like comment lines are found.
pub fn detect_commented_out_code(lines: &[String], kinds: &[LineKind]) -> Vec<SmellInstance> {
    let mut smells = Vec::new();
    let mut run_start: Option<usize> = None;
    let mut run_len = 0;

    for (i, line) in lines.iter().enumerate() {
        let is_comment = kinds.get(i) == Some(&LineKind::Comment);
        if is_comment && has_code_patterns(line) {
            if run_start.is_none() {
                run_start = Some(i);
            }
            run_len += 1;
        } else {
            if run_len >= 2
                && let Some(start) = run_start
            {
                smells.push(SmellInstance {
                    kind: SmellKind::CommentedOutCode,
                    line: start + 1,
                    detail: format!("{run_len} lines of commented-out code"),
                });
            }
            run_start = None;
            run_len = 0;
        }
    }

    // Handle a run that extends to the end
    if run_len >= 2
        && let Some(start) = run_start
    {
        smells.push(SmellInstance {
            kind: SmellKind::CommentedOutCode,
            line: start + 1,
            detail: format!("{run_len} lines of commented-out code"),
        });
    }

    smells
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
#[path = "rules_test.rs"]
mod tests;
