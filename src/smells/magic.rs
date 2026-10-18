//! The magic_number smell: a bare numeric literal in logic, rather than the
//! value of something named.

use crate::loc::counter::LineKind;
use crate::util::mask_strings;

use super::analyzer::{SmellInstance, SmellKind};

/// Declaration keywords: a numeric literal on such a line is being given a
/// name, which is the fix the smell asks for. Any of them before the first
/// `=` makes the line a declaration, whatever modifiers come first
/// (`private static final`, `public const`). The shell ones (`readonly`,
/// `declare`, `export`, `local`) cover `readonly FOO=16` and the like.
const DECL_KEYWORDS: &[&str] = &[
    "const",
    "constexpr",
    "let",
    "static",
    "final",
    "val",
    "#define",
    "enum",
    "readonly",
    "declare",
    "export",
    "local",
    "type",
];

/// Trivial numeric values that are not considered magic numbers.
const TRIVIAL_NUMBERS: &[&str] = &["0", "1", "2", "-1"];

/// Detect bare numeric literals in code lines.
///
/// Left out: declarations and what they leave open (the members of an
/// `enum`, a `const (...)` block, the elements of a table, the alternatives
/// of a `type` union), trivial values, and literals that a key or a
/// parameter already names (`connect: 3`, `"MAX": 4093`, `code: int = 303`).
pub fn detect_magic_numbers(
    lines: &[String],
    kinds: &[LineKind],
    line_comments: &[&str],
) -> Vec<SmellInstance> {
    let mut declarations = Declarations::default();
    let mut smells = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if kinds.get(i) != Some(&LineKind::Code) {
            continue;
        }
        let masked = mask_strings(line, line_comments);
        let trimmed = masked.trim();
        if declarations.covers(trimmed) {
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

/// What is known of the declaration being read, line by line.
#[derive(Default)]
struct Declarations {
    /// Brackets a declaration of data left open: its lines are part of it.
    open: i32,
    /// A declaration whose data starts on the next line: the block of an
    /// `enum` (`public enum Token` above `{`), or the value after a trailing
    /// `=` (`type Code =` above `| 400`).
    pending: bool,
}

impl Declarations {
    /// Whether `trimmed` is a declaration or part of one, taking it in.
    fn covers(&mut self, trimmed: &str) -> bool {
        let balance = bracket_balance(trimmed);
        if self.open > 0 {
            self.open += balance;
            return true;
        }
        let continues = self.pending && starts_continuation(trimmed);
        let declares = continues || is_declaration_line(trimmed);
        if declares && (continues || opens_data(trimmed)) {
            self.open = balance.max(0);
        }
        self.pending = declares && balance <= 0 && awaits_data(trimmed, continues);
        declares
    }
}

/// Whether a declaration line opens data rather than a body: an
/// initializer (the bracket comes after a plain `=`), the members of an
/// `enum`, or a Go group (`const (`, `var (`).
fn opens_data(trimmed: &str) -> bool {
    let initializer =
        plain_assign(trimmed).is_some_and(|at| trimmed[at..].contains(['(', '[', '{']));
    let group = matches!(trimmed, "const (" | "var (");
    initializer || group || has_word(trimmed, "enum")
}

/// Where the first `=` of a line that assigns is: outside parentheses, so
/// that a parameter default (`f(int x = 1) {`) is none, and not part of
/// `==`, `<=`, `=>` and the like.
fn plain_assign(trimmed: &str) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    let mut depth = 0;
    (0..bytes.len()).find(|&i| {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        let alone = !matches!(bytes.get(i + 1), Some(b'=' | b'>'))
            && !(i > 0 && b"=!<>+-*/%&|^:".contains(&bytes[i - 1]));
        depth == 0 && bytes[i] == b'=' && alone
    })
}

/// Whether the data of a declaration that opened none on its line comes on
/// the next one.
fn awaits_data(trimmed: &str, continues: bool) -> bool {
    has_word(trimmed, "enum")
        || trimmed.ends_with('=')
        || (continues && !trimmed.ends_with([';', ',']))
}

fn has_word(text: &str, word: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|w| w == word)
}

/// Whether a line goes on with the declaration above it: it opens its
/// block, or continues its value with an operator.
fn starts_continuation(trimmed: &str) -> bool {
    trimmed.starts_with(['{', '(', '[', '|', '&'])
}

/// Opening minus closing brackets of a line.
fn bracket_balance(trimmed: &str) -> i32 {
    trimmed
        .bytes()
        .map(|b| match b {
            b'(' | b'[' | b'{' => 1,
            b')' | b']' | b'}' => -1,
            _ => 0,
        })
        .sum()
}

/// Whether a line declares a name: a declaration keyword before its first
/// `=`, or an assignment to an upper-case name (a constant by convention).
fn is_declaration_line(trimmed: &str) -> bool {
    let head = trimmed.split('=').next().unwrap_or("");
    let lower = head.to_lowercase();
    let keyword = lower
        .split(|c: char| c.is_whitespace() || c == '(')
        .any(|word| DECL_KEYWORDS.contains(&word));
    keyword || assigns_constant(trimmed, head)
}

/// Whether the line assigns to an upper-case name: `MAX_RETRIES = 5`.
fn assigns_constant(trimmed: &str, head: &str) -> bool {
    let name = head.trim();
    trimmed.len() > head.len()
        && name.chars().any(|c| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Check whether a byte is part of a numeric literal (digit, hex letter, dot, prefix char).
/// Includes `_` to handle Rust/Python/Swift digit separators (e.g. `1_000_000`, `0xFF_FF`).
fn is_numeric_char(b: u8) -> bool {
    b.is_ascii_hexdigit() || matches!(b, b'.' | b'_' | b'x' | b'X' | b'o' | b'O')
}

/// Check whether a `-` at `pos` is a unary negative sign (not a binary minus).
/// Returns true if followed by a digit and not preceded by alphanumeric.
fn is_negative_prefix(bytes: &[u8], pos: usize) -> bool {
    bytes[pos] == b'-'
        && pos + 1 < bytes.len()
        && bytes[pos + 1].is_ascii_digit()
        && (pos == 0 || !bytes[pos - 1].is_ascii_alphanumeric())
}

/// Whether the byte at `pos` continues an identifier.
fn is_ident_byte(bytes: &[u8], pos: usize) -> bool {
    bytes
        .get(pos)
        .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

/// Consume a numeric literal starting at `start` in `bytes`, returning the
/// position after the literal (including any exponent suffix like e+10).
fn scan_numeric_literal(bytes: &[u8], start: usize) -> usize {
    let mut j = start;
    while j < bytes.len() && is_numeric_char(bytes[j]) {
        let exponent_sign =
            matches!(bytes[j], b'e' | b'E') && matches!(bytes.get(j + 1), Some(b'+' | b'-'));
        j += if exponent_sign { 2 } else { 1 };
    }
    j
}

/// Whether the literal that starts at `start` is the value of something
/// named: a key (`name: 3`, `"KEY": 4093`, `key => 3`) or a parameter
/// default (`code: int = 303` inside parentheses).
fn is_named_value(bytes: &[u8], start: usize) -> bool {
    let before = String::from_utf8_lossy(&bytes[..start]);
    let before = before.trim_end();
    if before.ends_with("=>") || (before.ends_with(':') && !before.ends_with("::")) {
        return true;
    }
    before.ends_with('=') && !before.ends_with("==") && inside_parens(before)
}

/// Whether the end of `text` lies inside parentheses it opened.
fn inside_parens(text: &str) -> bool {
    let depth: i32 = text
        .bytes()
        .map(|b| match b {
            b'(' => 1,
            b')' => -1,
            _ => 0,
        })
        .sum();
    depth > 0
}

/// Scan a masked code line for a bare numeric literal that isn't trivial.
fn has_magic_number(trimmed: &str) -> bool {
    let bytes = trimmed.as_bytes();
    let mut j = 0;
    while j < bytes.len() {
        let start = j;
        if is_negative_prefix(bytes, j) {
            j += 1;
        }
        if !bytes[j].is_ascii_digit() {
            j += 1;
            continue;
        }
        j = scan_numeric_literal(bytes, j);
        let in_identifier =
            (start > 0 && is_ident_byte(bytes, start - 1)) || is_ident_byte(bytes, j);
        if in_identifier || is_named_value(bytes, start) {
            continue;
        }
        let number = trimmed[start..j].trim_end_matches('.');
        if !TRIVIAL_NUMBERS.contains(&number) {
            return true;
        }
    }
    false
}

#[cfg(test)]
#[path = "magic_test.rs"]
mod tests;
