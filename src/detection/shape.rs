//! Declarations recognised by their shape: the C-family heuristic, and the
//! lines that continue a statement rather than start one.

use super::*;

/// Control-flow keywords that should NOT be treated as function definitions
/// in the C-family heuristic. If the first word of a line matches one of
/// these, it is treated as a control statement rather than a function.
pub(super) const CONTROL_KEYWORDS: &[&str] = &[
    "if",
    "for",
    "foreach",
    "while",
    "switch",
    "else",
    "do",
    "catch",
    "return",
    "case",
    "new",
    "throw",
    "await",
    "yield",
    "typeof",
    "sizeof",
    "using",
    "lock",
    "fixed",
    "synchronized",
    "try",
    "when",
];

/// Heuristic for C/C++/Java/C#/Bash function detection on a signature,
/// joined over its lines: it contains '(' and ends with '{', or is followed
/// by a line opening with '{' (`next_opens`); its first word, after any
/// closing braces, is NOT a control keyword; and there is no '=' before
/// the first '(' (which would make it an assignment such as `x = foo(...)`
/// or a shell command substitution `VAR=$(cmd ...)`).
///
/// Known limitations:
/// - Function pointers (e.g., `void (*fp)(int)`) may be misdetected.
pub(super) fn is_c_family_function(sig: &str, next_opens: bool) -> bool {
    if declares_no_function(sig) {
        return false;
    }
    let sig = &without_operator_symbols(sig);
    let paren_pos = match sig.find('(') {
        Some(p) => p,
        None => return false,
    };
    let opens = sig.ends_with('{') || (next_opens && !sig.ends_with([';', ',', '=']));
    if !opens {
        return false;
    }

    // Assignment before the first '(' means this is a variable assignment
    // or a shell command substitution, not a function definition. Examples:
    //   x = foo(bar)             (C-family)
    //   VAR=$(cmd | filter)      (Bash command substitution)
    //   local FOO=$(...)         (Bash local assignment)
    // The `=` is checked in the prefix only; a `=` inside the parens
    // (default-argument, comparison) is unrelated.
    if sig[..paren_pos].contains('=') || plain_assign(sig).is_some() {
        return false;
    }

    let first_word = sig
        .trim_start_matches(|c: char| c == '}' || c.is_whitespace())
        .split(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("");
    let first_word = first_word.trim_start_matches('*');
    !CONTROL_KEYWORDS.contains(&first_word)
}

/// The signature of the function the line at `i` declares by its shape, if
/// it does.
pub(super) fn shaped_declaration(
    masked: &[String],
    i: usize,
    syntax: FunctionSyntax,
) -> Option<String> {
    let (sig, end) = shaped_signature(masked, i);
    let next_opens = masked
        .get(end + 1)
        .is_some_and(|next| next.trim_start().starts_with('{'));
    let declared = match syntax {
        // In a script a leading `#` names a private member, not a directive.
        FunctionSyntax::Script => {
            is_c_family_function(sig.trim_start_matches('#'), next_opens)
                || (sig.ends_with('{') && script_binding(&sig).is_some())
        }
        _ => is_c_family_function(&sig, next_opens),
    };
    declared.then_some(sig)
}

/// For each line, whether it starts inside parentheses or brackets that
/// the lines before it left open: a continuation of a statement, never a
/// declaration. A line at the left margin starts afresh, so that a bracket
/// misread in a literal does not hide the rest of the file.
pub(super) fn continuation_lines(masked: &[String]) -> Vec<bool> {
    let mut depth: isize = 0;
    masked
        .iter()
        .map(|line| {
            let at_margin = !line.starts_with([' ', '\t']) && !line.starts_with([')', ']', '>']);
            if at_margin {
                depth = 0;
            }
            // A line opening with `>` closes type parameters of the lines above.
            let inside = depth > 0 || line.trim_start().starts_with('>');
            depth = (depth + paren_balance(line)).max(0);
            inside
        })
        .collect()
}

/// Opening minus closing parentheses and brackets of a line.
pub(super) fn paren_balance(line: &str) -> isize {
    line.bytes()
        .map(|b| match b {
            b'(' | b'[' => 1,
            b')' | b']' => -1,
            _ => 0,
        })
        .sum()
}
