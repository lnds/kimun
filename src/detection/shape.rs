//! Declarations recognised by their shape: signatures joined over the
//! lines they span, the C-family heuristic, and the lines that continue a
//! statement rather than start one.

use super::*;

/// How many lines a signature may span before it is given up on.
pub(super) const MAX_SIGNATURE_LINES: usize = 30;

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
    // Preprocessor directives (`#define`, `#if`, `#include`, ...) are not
    // functions. A function-like `#define NAME(args) body` otherwise looks
    // like a C function (`identifier (params) body`) and would be misdetected,
    // absorbing the cognitive complexity of the code that follows it.
    if sig.starts_with('#') {
        return false;
    }
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

/// The line at `i` joined with those after it until its parentheses
/// close, and the index of its last line. Type parameters that a line
/// leaves open at its end (`<`) are followed to the line that closes them
/// (`>`).
pub(super) fn signature(masked: &[String], i: usize) -> (String, usize) {
    let last = (i + MAX_SIGNATURE_LINES).min(masked.len()) - 1;
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut generics = false;
    for (j, line) in masked.iter().enumerate().take(last + 1).skip(i) {
        let line = line.trim();
        parts.push(line);
        depth += paren_balance(line);
        generics = (generics && !line.starts_with('>')) || line.ends_with('<');
        if depth <= 0 && !generics {
            return (parts.join(" "), j);
        }
    }
    (parts.join(" "), last)
}
