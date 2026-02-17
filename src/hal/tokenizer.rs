use std::collections::HashSet;

use crate::util::mask_strings;

/// Per-language rules for classifying tokens as operators or operands.
///
/// Classification follows Halstead's definitions:
/// - **Operators**: control-flow keywords (if, for, while, return, ...),
///   type-cast/query operators (as, sizeof, typeof, new, delete),
///   and symbolic operators (+, -, *, =, ==, &&, ||, ...).
/// - **Operands**: identifiers, numeric/string literals, function names.
/// - **Ignored**: declaration keywords (fn, let, struct, class, import),
///   visibility/modifier keywords (pub, mut, virtual, async),
///   and type names (i32, bool, String, ...).
///
/// **Design note:** Function names are classified as operands, with the call
/// syntax `()` counted as the operator. This differs from Halstead's original
/// formulation where function names were operators, but aligns with modern
/// interpretations that treat function names as first-class values. The `as`
/// keyword is context-dependent: in Rust/Kotlin/Swift it is primarily a
/// type-cast operator, so it is classified as an operator; in Python it is
/// used exclusively for aliasing (`import X as Y`, `except E as e`,
/// `with ... as`), not type conversion, so it is ignored.
pub struct TokenRules {
    /// Keywords that count as operators (control flow and operations only).
    pub operator_keywords: &'static [&'static str],
    /// Multi-char and single-char symbolic operators, longest first.
    pub operator_symbols: &'static [&'static str],
    /// Keywords to ignore (declarations, modifiers, type names).
    pub ignored_keywords: &'static [&'static str],
}

/// Raw token counts extracted from source code.
pub struct TokenCounts {
    pub distinct_operators: HashSet<String>,
    pub distinct_operands: HashSet<String>,
    pub total_operators: usize,
    pub total_operands: usize,
}

// ── Language rules ──────────────────────────────────────────────────────

static RUST: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "else", "match", "for", "while", "loop", "return", "break", "continue", "as", "in",
    ],
    operator_symbols: &[
        "..=", "...", "=>", "->", "&&", "||", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=", "%=",
        "&=", "|=", "^=", "<<=", ">>=", "<<", ">>", "::", "..", "+", "-", "*", "/", "%", "&", "|",
        "^", "!", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{", "}", "?", "#",
    ],
    ignored_keywords: &[
        "fn", "let", "mut", "const", "static", "extern", "unsafe", "async", "await", "move", "ref",
        "impl", "struct", "enum", "trait", "type", "use", "mod", "where", "pub", "crate", "super",
        "self", "Self", "dyn", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32",
        "u64", "u128", "usize", "f32", "f64", "bool", "char", "str", "String", "Vec", "Option",
        "Result", "Box",
    ],
};

static PYTHON: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "elif", "else", "for", "while", "return", "yield", "try", "except", "finally",
        "raise", "assert", "del", "break", "continue", "in", "not", "and", "or", "is", "with",
    ],
    operator_symbols: &[
        "**=", "//=", ">>=", "<<=", "**", "//", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=",
        "%=", "&=", "|=", "^=", "<<", ">>", "->", "+", "-", "*", "/", "%", "&", "|", "^", "~", "<",
        ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{", "}", "@",
    ],
    ignored_keywords: &[
        "def", "class", "import", "from", "as", "lambda", "global", "nonlocal", "async", "await",
        "pass", "int", "float", "str", "bool", "list", "dict", "tuple", "set", "type",
    ],
};

static JAVASCRIPT: TokenRules = TokenRules {
    operator_keywords: &[
        "if",
        "else",
        "for",
        "while",
        "do",
        "switch",
        "case",
        "default",
        "return",
        "break",
        "continue",
        "throw",
        "try",
        "catch",
        "finally",
        "new",
        "delete",
        "typeof",
        "instanceof",
        "in",
        "of",
        "yield",
    ],
    operator_symbols: &[
        "===", "!==", ">>>", "**=", ">>=", "<<=", "=>", "&&", "||", "??", "==", "!=", "<=", ">=",
        "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "**", "<<", ">>", "++", "--", "+", "-",
        "*", "/", "%", "&", "|", "^", "~", "!", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[",
        "]", "{", "}", "?",
    ],
    ignored_keywords: &[
        "function",
        "var",
        "let",
        "const",
        "class",
        "extends",
        "import",
        "export",
        "async",
        "await",
        "undefined",
        "void",
    ],
};

static GO: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "else", "for", "switch", "case", "default", "return", "break", "continue", "go",
        "defer", "select", "range",
    ],
    operator_symbols: &[
        ":=", "&&", "||", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
        "<<=", ">>=", "<<", ">>", "<-", "++", "--", "+", "-", "*", "/", "%", "&", "|", "^", "!",
        "<", ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{", "}",
    ],
    ignored_keywords: &[
        "func",
        "var",
        "const",
        "type",
        "struct",
        "interface",
        "import",
        "package",
        "map",
        "chan",
        "int",
        "int8",
        "int16",
        "int32",
        "int64",
        "uint",
        "uint8",
        "uint16",
        "uint32",
        "uint64",
        "float32",
        "float64",
        "bool",
        "string",
        "byte",
        "rune",
        "error",
    ],
};

static C_FAMILY: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "else", "for", "while", "do", "switch", "case", "default", "return", "break",
        "continue", "goto", "sizeof", "new", "delete", "throw", "try", "catch",
    ],
    operator_symbols: &[
        "->", "::", "&&", "||", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=", "%=", "&=", "|=",
        "^=", "<<=", ">>=", "<<", ">>", "++", "--", "+", "-", "*", "/", "%", "&", "|", "^", "~",
        "!", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{", "}", "?", "#",
    ],
    ignored_keywords: &[
        "typedef",
        "struct",
        "union",
        "enum",
        "class",
        "namespace",
        "using",
        "template",
        "virtual",
        "override",
        "int",
        "long",
        "short",
        "char",
        "float",
        "double",
        "void",
        "unsigned",
        "signed",
        "bool",
        "auto",
        "const",
        "static",
        "extern",
        "register",
        "volatile",
        "inline",
        "public",
        "private",
        "protected",
    ],
};

static RUBY: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "elsif", "else", "unless", "case", "when", "while", "until", "for", "do", "begin",
        "rescue", "ensure", "raise", "return", "break", "next", "yield", "end", "then", "in",
        "and", "or", "not",
    ],
    operator_symbols: &[
        "<=>", "===", "**=", "&&=", "||=", "=>", "&&", "||", "==", "!=", "<=", ">=", "+=", "-=",
        "*=", "/=", "%=", "**", "<<", ">>", "..", "+", "-", "*", "/", "%", "&", "|", "^", "~", "!",
        "<", ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{", "}", "?", "@",
    ],
    ignored_keywords: &[
        "def",
        "class",
        "module",
        "require",
        "include",
        "extend",
        "attr_reader",
        "attr_writer",
        "attr_accessor",
    ],
};

static KOTLIN: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "else", "when", "for", "while", "do", "return", "break", "continue", "throw", "try",
        "catch", "finally", "is", "as", "in",
    ],
    operator_symbols: &[
        "?:", "&&", "||", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=", "%=", "->", "::", "++",
        "--", "..", "+", "-", "*", "/", "%", "!", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[",
        "]", "{", "}", "?", "@",
    ],
    ignored_keywords: &[
        "fun",
        "val",
        "var",
        "class",
        "object",
        "interface",
        "import",
        "package",
        "private",
        "public",
        "protected",
        "internal",
        "open",
        "override",
        "abstract",
        "data",
        "sealed",
        "companion",
        "Int",
        "Long",
        "Short",
        "Byte",
        "Float",
        "Double",
        "Boolean",
        "Char",
        "String",
        "Unit",
        "Any",
        "Nothing",
    ],
};

static SWIFT: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "else", "guard", "switch", "case", "default", "for", "while", "repeat", "return",
        "break", "continue", "throw", "try", "catch", "as", "is", "in",
    ],
    operator_symbols: &[
        "&&", "||", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=", "%=", "->", "..<", "...", "??",
        "+", "-", "*", "/", "%", "&", "|", "^", "~", "!", "<", ">", "=", ";", ",", ".", ":", "(",
        ")", "[", "]", "{", "}", "?", "@",
    ],
    ignored_keywords: &[
        "func",
        "let",
        "var",
        "class",
        "struct",
        "enum",
        "protocol",
        "extension",
        "import",
        "where",
        "private",
        "public",
        "internal",
        "open",
        "fileprivate",
        "override",
        "mutating",
        "static",
        "Int",
        "Float",
        "Double",
        "Bool",
        "String",
        "Character",
        "Void",
        "Any",
        "Self",
    ],
};

static SHELL: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "then", "elif", "else", "fi", "for", "while", "until", "do", "done", "case", "esac",
        "in", "return", "exit", "break", "continue",
    ],
    operator_symbols: &[
        "&&", "||", "==", "!=", "<=", ">=", ">>", "<<", ";;", "|", "&", ";", "<", ">", "=", "(",
        ")", "[", "]", "{", "}", "!", "$",
    ],
    ignored_keywords: &[
        "function", "local", "export", "readonly", "declare", "eval", "exec", "source",
    ],
};

/// Look up the tokenization rules for a language by name.
/// Returns `None` for unsupported languages (no Halstead analysis available).
pub fn rules_for(language: &str) -> Option<&'static TokenRules> {
    match language {
        "Rust" => Some(&RUST),
        "Python" => Some(&PYTHON),
        "JavaScript" | "TypeScript" => Some(&JAVASCRIPT),
        "Go" => Some(&GO),
        "C" | "C++" | "C#" | "Java" | "Objective-C" | "PHP" | "Dart" => Some(&C_FAMILY),
        "Ruby" => Some(&RUBY),
        "Kotlin" => Some(&KOTLIN),
        "Swift" => Some(&SWIFT),
        "Bourne Shell" | "Bourne Again Shell" | "Zsh" => Some(&SHELL),
        _ => None,
    }
}

/// Extract and classify tokens from code lines, counting operators and operands.
///
/// Each line is first passed through `mask_strings` to replace string literal
/// contents with spaces, preventing keywords inside strings from being counted.
/// Non-ASCII bytes are skipped (may appear in trailing comments on code lines).
/// Tokens are classified into three categories:
/// - **Operator keywords** (if, for, return, ...) → counted as operators
/// - **Ignored keywords** (fn, let, struct, ...) → skipped entirely
/// - **Identifiers and numeric literals** → counted as operands
///
/// Symbolic operators use longest-match semantics via `try_match_symbol`.
pub fn count_tokens(code_lines: &[&str], rules: &TokenRules) -> TokenCounts {
    let mut counts = TokenCounts {
        distinct_operators: HashSet::new(),
        distinct_operands: HashSet::new(),
        total_operators: 0,
        total_operands: 0,
    };

    for line in code_lines {
        let masked = mask_strings(line);
        let bytes = masked.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let ch = bytes[i];

            // Skip whitespace
            if ch.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            // Skip non-ASCII bytes (comments on code lines may contain UTF-8)
            if !ch.is_ascii() {
                i += 1;
                continue;
            }

            // Try multi-char symbols (longest match first — rules are pre-sorted)
            if let Some(sym) = try_match_symbol(&bytes[i..], rules.operator_symbols) {
                counts.distinct_operators.insert(sym.to_string());
                counts.total_operators += 1;
                i += sym.len();
                continue;
            }

            // Alphanumeric token (identifier, keyword, or number)
            if ch.is_ascii_alphanumeric() || ch == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                // Safe: start..i spans only ASCII bytes
                let token = &masked[start..i];

                // Check if this is an operator keyword
                if rules.operator_keywords.contains(&token) {
                    counts.distinct_operators.insert(token.to_string());
                    counts.total_operators += 1;
                } else if rules.ignored_keywords.contains(&token) {
                    // Skip ignored tokens
                } else if is_numeric(token) {
                    // Numeric literal → operand
                    counts.distinct_operands.insert(token.to_string());
                    counts.total_operands += 1;
                } else {
                    // Identifier (variable name, function name, etc.) → operand.
                    // Function names are operands; the call mechanism ()
                    // is already counted as an operator via operator_symbols.
                    counts.distinct_operands.insert(token.to_string());
                    counts.total_operands += 1;
                }
                continue;
            }

            // Unrecognized char — skip
            i += 1;
        }
    }

    counts
}

/// Find the first (longest) symbol that matches at the start of `rest`.
/// Relies on `operator_symbols` being sorted longest-first so that `>>=`
/// is matched before `>>` or `>`.
fn try_match_symbol<'a>(rest: &[u8], symbols: &[&'a str]) -> Option<&'a str> {
    symbols
        .iter()
        .find(|sym| rest.starts_with(sym.as_bytes()))
        .copied()
}

/// Check if a token is a numeric literal (starts with an ASCII digit).
/// Covers decimal, hex (0x), binary (0b), and octal (0o) prefixes.
fn is_numeric(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // Starts with digit, or 0x/0b/0o prefix
    bytes[0].is_ascii_digit()
}

#[cfg(test)]
#[path = "tokenizer_test.rs"]
mod tests;
