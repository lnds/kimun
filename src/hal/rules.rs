/// Per-language tokenization rules for Halstead complexity metrics.
///
/// Each language defines operator keywords (control flow), symbolic
/// operators (longest-match ordered), and ignored keywords (declarations,
/// modifiers, type names).
use super::tokenizer::TokenRules;

// ── Language rules ──────────────────────────────────────────────────────

/// Rust tokenization rules: includes `as` as operator (type cast),
/// lifetime-related keywords in ignored set.
pub static RUST: TokenRules = TokenRules {
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

/// Python tokenization rules: `as` is ignored (aliasing, not type cast).
pub static PYTHON: TokenRules = TokenRules {
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

/// JavaScript/TypeScript tokenization rules.
pub static JAVASCRIPT: TokenRules = TokenRules {
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

/// Go tokenization rules.
///
/// Includes `go` (goroutine launch), `defer`, `select`, and `range`
/// as operators since they represent control flow decisions.
pub static GO: TokenRules = TokenRules {
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

/// C-family tokenization rules (C, C++, Java, C#, Objective-C, PHP, Dart).
///
/// Uses a shared operator/keyword set covering the common subset of these
/// languages. The `goto` keyword is included for C/C++ but harmless for
/// languages that don't support it.
pub static C_FAMILY: TokenRules = TokenRules {
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

/// Ruby tokenization rules.
///
/// Includes block delimiters (`do`, `end`, `begin`) as operators since they
/// affect control flow. The `yield` keyword is an operator (passes control
/// to a block).
pub static RUBY: TokenRules = TokenRules {
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

/// Kotlin tokenization rules: `as` is an operator (type cast).
///
/// Includes `when` (Kotlin's `switch` equivalent) and `is` (type check)
/// as operators. Distinguishes Kotlin-specific types in ignored set.
pub static KOTLIN: TokenRules = TokenRules {
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

/// Swift tokenization rules: `as` is an operator (type cast).
///
/// Includes `guard` (early exit pattern) and `repeat` (do-while equivalent)
/// as operators. Swift-specific types are in the ignored set.
pub static SWIFT: TokenRules = TokenRules {
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

/// Shell (Bash/Zsh/sh) tokenization rules.
///
/// Treats shell control structures (`if/then/fi`, `for/do/done`,
/// `case/esac`) as operators. The `$` symbol is included as an operator
/// since it controls variable expansion.
pub static SHELL: TokenRules = TokenRules {
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
