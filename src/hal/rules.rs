/// Per-language tokenization rules for Halstead complexity metrics.
///
/// Each language defines three token categories:
/// - **operator_keywords**: control flow keywords that represent decisions
///   or actions (e.g. `if`, `return`, `for`). These count as operators in
///   Halstead's model because they direct program execution.
/// - **operator_symbols**: symbolic operators ordered longest-first so that
///   multi-character operators (`>>=`, `&&`) are matched before their
///   single-character prefixes (`>`, `&`). Longest-match ordering is
///   critical for correct tokenization.
/// - **ignored_keywords**: declarations, modifiers, and type names that
///   are structural rather than computational. These are excluded from
///   both operator and operand counts to focus the metric on logic
///   complexity rather than syntactic boilerplate.
///
/// The classification of a keyword as operator vs. ignored depends on
/// the language semantics. For example, `as` is an operator in Rust
/// (type cast — a computation) but ignored in Python (aliasing — a
/// declaration). Each language's doc comment explains such distinctions.
use super::tokenizer::TokenRules;

// ── Language rules ──────────────────────────────────────────────────────

/// Rust tokenization rules.
///
/// `as` is classified as an operator because it performs a type cast (a
/// runtime computation), unlike Python where `as` merely creates an alias.
/// `loop` is an operator (unconditional looping control flow). Lifetime
/// and ownership keywords (`mut`, `ref`, `move`) are ignored because they
/// are declarative annotations rather than executable operations.
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

/// Python tokenization rules.
///
/// `as` is ignored here (unlike Rust) because Python's `as` is used for
/// aliasing in imports (`import x as y`) and context managers (`with x as y`),
/// which are declarative rather than computational. Boolean operators
/// (`and`, `or`, `not`, `is`) are keywords rather than symbols in Python.
/// `with` is an operator because it controls resource management flow.
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
///
/// Shared between JS and TS since their operator sets are identical.
/// `typeof` and `instanceof` are operators (runtime type checks).
/// `yield` is an operator (generator control flow). The nullish
/// coalescing operator `??` is included alongside `&&` and `||`.
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
/// `go` (goroutine launch), `defer` (deferred execution), and `select`
/// (channel multiplexing) are operators because they control execution
/// flow. `range` is an operator (iteration control). Go has no
/// ternary operator, and `:=` (short variable declaration) is included
/// as an operator symbol because it combines declaration with assignment.
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
/// Uses a shared operator/keyword set covering the common subset. The
/// `goto` keyword is included for C/C++ but is harmless for languages
/// that don't support it — an unrecognized keyword simply won't match
/// any source tokens. `sizeof` is an operator (compile-time computation).
/// The `->` (member access) and `::` (scope resolution) symbols are
/// included because they represent operand access operations.
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
/// Block delimiters (`do`, `end`, `begin`) are operators because they
/// demarcate executable blocks, unlike braces in C-family languages which
/// are purely syntactic. `yield` passes control to a block (operator).
/// `unless` and `until` are operators (negated conditionals/loops).
/// The spaceship operator `<=>` and case equality `===` are included.
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

/// Kotlin tokenization rules.
///
/// `as` is an operator (type cast, like Rust). `when` replaces `switch`
/// and is an operator (pattern-matching control flow). `is` performs
/// runtime type checks (operator). Kotlin visibility modifiers (`internal`,
/// `open`, `sealed`, `companion`) are ignored as declarative annotations.
/// The Elvis operator `?:` is included as a null-handling operator.
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

/// Swift tokenization rules.
///
/// `as` is an operator (type cast). `guard` is an operator (early exit
/// pattern — enforces conditions at function boundaries). `repeat` is
/// Swift's do-while equivalent. The half-open range `..<` and closed
/// range `...` operators are included. `fileprivate` is ignored as a
/// visibility modifier with no computational effect.
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
/// Shell control structures use keyword pairs (`if/then/fi`,
/// `for/do/done`, `case/esac`) — all keywords in each pair are operators
/// because they jointly control execution flow. The `$` symbol is an
/// operator because it triggers variable expansion (a runtime operation).
/// `;;` (case terminator) is a shell-specific operator symbol.
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
