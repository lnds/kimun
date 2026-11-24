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
use crate::lang::language::is_fortran;
use crate::util::Apostrophe;

/// Integer types written by their size (Go, Nim).
const SIZED_INTEGERS: &[&str] = &[
    "int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16", "uint32", "uint64",
];

/// What OCaml and F# share: names that may end in primes (`x'`) and the
/// basic types.
const ML: TokenRules = TokenRules {
    name_bytes: b"'",
    apostrophe: Apostrophe::Prime,
    type_names: &[
        "int", "float", "string", "bool", "unit", "char", "list", "option", "array",
    ],
    ..TokenRules::PLAIN
};

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
    ..TokenRules::PLAIN
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
    ..TokenRules::PLAIN
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
    ..TokenRules::PLAIN
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
        "float32",
        "float64",
        "bool",
        "string",
        "byte",
        "rune",
        "error",
    ],
    type_names: SIZED_INTEGERS,
    ..TokenRules::PLAIN
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
    ..TokenRules::PLAIN
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
    ..TokenRules::PLAIN
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
    ..TokenRules::PLAIN
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
    ..TokenRules::PLAIN
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
    ..TokenRules::PLAIN
};

/// Kaikai tokenization rules.
///
/// Functional language with algebraic effects. Control flow keywords
/// (`if`, `match`, `case`, `when`, `handle`, `with`) and boolean keywords
/// (`and`, `or`, `not`) are operators. `as` is an operator (capability/import
/// alias binding). Pipeline operators (`|>`, `|`, `||`, `|?`, `|!`) drive
/// dataflow. `:=` is mutable cell write, `++` is concat, `??` the default of
/// an `Option`, `&` makes a reference, `$` calls an intrinsic, `=>` lambda
/// arrow, `->` type/pattern arrow. Declarations (`fn`, `let`, `var`, `type`,
/// `effect`, `protocol`, `impl`, `import`, `use`, visibility, `for` of an
/// `impl`, `lazy`) are ignored.
pub static KAIKAI: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "else", "match", "case", "when", "handle", "with", "and", "or", "not", "as",
    ],
    operator_symbols: &[
        "|>", "|?", "|!", "...", "++", "??", "==", "!=", "<=", ">=", ":=", "=>", "->", "||", "..",
        "+", "-", "*", "/", "%", "^", "|", "!", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[",
        "]", "{", "}", "@", "?", "&", "$",
    ],
    ignored_keywords: &[
        "fn", "let", "var", "const", "type", "effect", "protocol", "impl", "import", "use", "pub",
        "priv", "extern", "test", "bench", "axiom", "check", "where", "requires", "ensures",
        "assert", "theory", "for", "lazy", "unit", "true", "false", "Int", "Real", "Bool",
        "String", "Char", "Unit",
    ],
    ..TokenRules::PLAIN
};

/// Elixir tokenization rules.
///
/// `do` and `end` delimit blocks the way braces do elsewhere, and `fn`
/// opens an anonymous function closed by `end`, so all three are
/// operators, as `do`/`end` are in Ruby. `when` guards a clause and
/// `raise`, `reraise` and `throw` leave the normal flow. The pipe `|>`,
/// concatenation (`<>`, `++`), the match `=` and the pin `^` are
/// operators. Definitions (`def`, `defmodule`, ...), the directives that
/// bring modules in (`alias`, `import`, `require`, `use`) and the names
/// of documentation and typespec attributes (`@doc`, `@spec`, ...) are
/// ignored: they declare, they do not compute.
pub static ELIXIR: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "unless", "case", "cond", "with", "for", "receive", "try", "rescue", "catch",
        "after", "else", "do", "end", "fn", "when", "and", "or", "not", "in", "raise", "reraise",
        "throw",
    ],
    operator_symbols: &[
        "===", "!==", "<<<", ">>>", "<<~", "~>>", "<~>", "|||", "&&&", "^^^", "...", "|>", "<>",
        "++", "--", "==", "!=", "=~", "<=", ">=", "&&", "||", "->", "<-", "=>", "::", "..", "//",
        "**", "<<", ">>", "~>", "<~", "\\\\", "+", "-", "*", "/", "|", "&", "^", "!", "<", ">",
        "=", ";", ",", ".", ":", "(", ")", "[", "]", "{", "}", "%", "@", "~",
    ],
    ignored_keywords: &[
        "def",
        "defp",
        "defmodule",
        "defmacro",
        "defmacrop",
        "defstruct",
        "defprotocol",
        "defimpl",
        "defdelegate",
        "defguard",
        "defguardp",
        "defexception",
        "defoverridable",
        "alias",
        "import",
        "require",
        "use",
        "moduledoc",
        "doc",
        "spec",
        "type",
        "typep",
        "opaque",
        "callback",
        "macrocallback",
        "impl",
        "behaviour",
    ],
    ..TokenRules::PLAIN
};

/// Fortran tokenization rules, on code read in lower case.
///
/// The dotted operators (`.and.`, `.eq.`) come before `.` would, and `//`
/// is concatenation. I/O statements (`print`, `read`, `write`) and `call`
/// are operators: they act. The `end` of a construct, joined to its
/// keyword when read (`endif`), is ignored, as declarations and attributes
/// are.
pub static FORTRAN: TokenRules = TokenRules {
    operator_keywords: &[
        "if",
        "then",
        "else",
        "elseif",
        "do",
        "while",
        "select",
        "case",
        "where",
        "elsewhere",
        "forall",
        "call",
        "return",
        "stop",
        "exit",
        "cycle",
        "goto",
        "continue",
        "allocate",
        "deallocate",
        "nullify",
        "print",
        "read",
        "write",
    ],
    operator_symbols: &[
        ".neqv.", ".eqv.", ".and.", ".not.", ".or.", ".eq.", ".ne.", ".lt.", ".le.", ".gt.",
        ".ge.", "**", "//", "==", "/=", "<=", ">=", "=>", "::", "(/", "/)", "+", "-", "*", "/",
        "<", ">", "=", "%", "(", ")", "[", "]", ",", ":", ";",
    ],
    ignored_keywords: &[
        "program",
        "module",
        "submodule",
        "subroutine",
        "function",
        "procedure",
        "end",
        "endif",
        "enddo",
        "endselect",
        "endwhere",
        "endforall",
        "endblock",
        "endassociate",
        "contains",
        "use",
        "only",
        "implicit",
        "none",
        "interface",
        "integer",
        "real",
        "double",
        "precision",
        "complex",
        "logical",
        "character",
        "type",
        "class",
        "intent",
        "in",
        "out",
        "inout",
        "parameter",
        "dimension",
        "allocatable",
        "pointer",
        "target",
        "optional",
        "save",
        "public",
        "private",
        "kind",
        "len",
        "result",
        "recursive",
        "pure",
        "impure",
        "elemental",
        "external",
        "intrinsic",
        "common",
        "data",
        "equivalence",
        "include",
    ],
    ..TokenRules::PLAIN
};

/// Pascal tokenization rules, on code read in lower case.
///
/// `begin` and `end` delimit blocks, as braces do in the C family, and
/// count as operators the way `{` and `}` do there. The word operators
/// (`and`, `div`, `shl`, `in`, `is`, `as`) compute.
pub static PASCAL: TokenRules = TokenRules {
    operator_keywords: &[
        "if",
        "then",
        "else",
        "case",
        "of",
        "for",
        "to",
        "downto",
        "do",
        "while",
        "repeat",
        "until",
        "with",
        "try",
        "except",
        "finally",
        "raise",
        "exit",
        "break",
        "continue",
        "goto",
        "begin",
        "end",
        "and",
        "or",
        "not",
        "xor",
        "div",
        "mod",
        "shl",
        "shr",
        "in",
        "is",
        "as",
        "inherited",
    ],
    operator_symbols: &[
        ":=", "<>", "<=", ">=", "..", "+=", "-=", "*=", "/=", "+", "-", "*", "/", "=", "<", ">",
        "^", "@", ".", ",", ";", ":", "(", ")", "[", "]",
    ],
    ignored_keywords: &[
        "program",
        "unit",
        "library",
        "interface",
        "implementation",
        "uses",
        "initialization",
        "finalization",
        "procedure",
        "function",
        "constructor",
        "destructor",
        "operator",
        "var",
        "const",
        "type",
        "class",
        "record",
        "object",
        "packed",
        "array",
        "set",
        "file",
        "label",
        "resourcestring",
        "threadvar",
        "out",
        "string",
        "integer",
        "cardinal",
        "boolean",
        "char",
        "real",
        "double",
        "single",
        "extended",
        "byte",
        "word",
        "longint",
        "int64",
        "pointer",
        "private",
        "protected",
        "public",
        "published",
        "strict",
        "property",
        "read",
        "write",
        "default",
        "virtual",
        "override",
        "overload",
        "abstract",
        "reintroduce",
        "static",
        "dynamic",
        "inline",
        "forward",
        "external",
        "cdecl",
        "stdcall",
    ],
    ..TokenRules::PLAIN
};

/// Lua tokenization rules.
///
/// `do`, `then` and `end` delimit blocks, as `do`/`end` do in Ruby, so they
/// are operators; `function` and `local` declare and are ignored. `~=` is
/// inequality, `..` concatenation, `#` length and `//` floor division.
pub static LUA: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "then", "elseif", "else", "for", "while", "repeat", "until", "do", "end", "return",
        "break", "goto", "and", "or", "not", "in",
    ],
    operator_symbols: &[
        "...", "..", "==", "~=", "<=", ">=", "//", "<<", ">>", "::", "+", "-", "*", "/", "%", "^",
        "#", "&", "~", "|", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{", "}",
    ],
    ignored_keywords: &["function", "local"],
    ..TokenRules::PLAIN
};

/// Perl tokenization rules.
///
/// The sigils `$` and `@` are operators, as `$` is in shell: they access a
/// variable. `%` is both the hash sigil and modulo. The word operators
/// (`eq`, `lt`, `cmp`, `and`, ...) compute, and `die` and `eval` leave or
/// guard the normal flow. `x` is left out: it is as often a variable name.
/// Declarations (`sub`, `my`, `package`, `use`, ...) are ignored.
pub static PERL: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "elsif", "else", "unless", "while", "until", "for", "foreach", "do", "last", "next",
        "redo", "goto", "return", "die", "eval", "and", "or", "not", "xor", "eq", "ne", "lt", "gt",
        "le", "ge", "cmp",
    ],
    operator_symbols: &[
        "<=>", "**=", "||=", "&&=", "//=", "<<=", ">>=", "...", "->", "=>", "=~", "!~", "==", "!=",
        "<=", ">=", "&&", "||", "//", "**", "++", "--", "+=", "-=", "*=", "/=", ".=", "%=", "|=",
        "&=", "^=", "<<", ">>", "::", "..", "+", "-", "*", "/", "%", "&", "|", "^", "~", "!", "<",
        ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{", "}", "?", "$", "@", "\\",
    ],
    ignored_keywords: &[
        "sub", "my", "our", "local", "use", "no", "package", "require", "BEGIN", "END",
    ],
    ..TokenRules::PLAIN
};

/// Scala tokenization rules (Scala 2 and 3).
///
/// `match` and `case` are pattern-matching control flow, `yield` gives the
/// value of a `for`, and `new` builds an object, so all are operators.
/// `with` mixes in a trait, a declaration, and is ignored with the other
/// declarations and modifiers. `<-` binds in a `for`, `<:` and `>:` bound a
/// type.
pub static SCALA: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "then", "else", "match", "case", "for", "while", "do", "yield", "return", "throw",
        "try", "catch", "finally", "new",
    ],
    operator_symbols: &[
        ">>>", "<<=", ">>=", "=>", "<-", "->", "<:", ">:", "::", "++", "+=", "-=", "*=", "/=",
        "%=", "&=", "|=", "^=", "==", "!=", "<=", ">=", "&&", "||", "<<", ">>", "+", "-", "*", "/",
        "%", "&", "|", "^", "~", "!", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{",
        "}", "@", "#",
    ],
    ignored_keywords: &[
        "def",
        "val",
        "var",
        "lazy",
        "class",
        "object",
        "trait",
        "enum",
        "extends",
        "with",
        "type",
        "import",
        "package",
        "private",
        "protected",
        "override",
        "abstract",
        "final",
        "sealed",
        "implicit",
        "given",
        "using",
        "extension",
        "export",
        "Any",
        "AnyRef",
        "Boolean",
        "Byte",
        "Char",
        "Double",
        "Float",
        "Int",
        "Long",
        "Nothing",
        "Short",
        "String",
        "Unit",
    ],
    ..TokenRules::PLAIN
};

/// Zig tokenization rules.
///
/// `defer` and `errdefer` schedule code, `try`, `catch` and `orelse`
/// handle errors and nulls, and `unreachable` ends a path: all control
/// flow. The wrapping (`+%`) and saturating (`+|`) arithmetic, `++`
/// concatenation, `**` repetition, `.*` dereference and `.?` unwrapping
/// are operators. `@` calls a builtin. Declarations, `comptime`, `async`
/// and type names are ignored.
pub static ZIG: TokenRules = TokenRules {
    operator_keywords: &[
        "if",
        "else",
        "switch",
        "while",
        "for",
        "return",
        "break",
        "continue",
        "defer",
        "errdefer",
        "try",
        "catch",
        "orelse",
        "and",
        "or",
        "unreachable",
        "suspend",
        "resume",
    ],
    operator_symbols: &[
        "<<=", ">>=", "+%=", "-%=", "*%=", "+|=", "-|=", "*|=", "...", "**", "++", "+%", "-%",
        "*%", "+|", "-|", "*|", "=>", "==", "!=", "<=", ">=", "<<", ">>", "+=", "-=", "*=", "/=",
        "%=", "&=", "|=", "^=", "..", ".*", ".?", "||", "+", "-", "*", "/", "%", "&", "|", "^",
        "~", "!", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{", "}", "?", "@",
    ],
    ignored_keywords: &[
        "fn",
        "const",
        "var",
        "pub",
        "extern",
        "export",
        "inline",
        "comptime",
        "struct",
        "enum",
        "union",
        "error",
        "packed",
        "threadlocal",
        "usingnamespace",
        "test",
        "async",
        "await",
        "anytype",
        "type",
        "void",
        "bool",
        "noreturn",
        "anyerror",
        "i8",
        "i16",
        "i32",
        "i64",
        "i128",
        "isize",
        "u8",
        "u16",
        "u32",
        "u64",
        "u128",
        "usize",
        "f16",
        "f32",
        "f64",
        "f128",
    ],
    ..TokenRules::PLAIN
};

/// Clojure tokenization rules.
///
/// A Lisp: every expression has parentheses, so they are no operator. The
/// operators are the special forms and core macros in call position, the
/// arithmetic and comparison functions, and the reader's `'`, `@`, `#`, `^`
/// and `` ` ``. A name goes on through `-`, `?`, `!` and the like, so
/// `if-not` and `some->` are one word. Definitions and the namespace form
/// declare and are ignored.
pub static CLOJURE: TokenRules = TokenRules {
    operator_keywords: &[
        "if",
        "if-not",
        "if-let",
        "if-some",
        "when",
        "when-not",
        "when-let",
        "when-some",
        "when-first",
        "cond",
        "condp",
        "cond->",
        "cond->>",
        "case",
        "do",
        "let",
        "letfn",
        "loop",
        "recur",
        "fn",
        "try",
        "catch",
        "finally",
        "throw",
        "and",
        "or",
        "not",
        "not=",
        "doseq",
        "dotimes",
        "for",
        "while",
        "some->",
        "some->>",
        "as->",
        "new",
        "set!",
    ],
    operator_symbols: &[
        "->>", "->", "~@", "<=", ">=", "+", "-", "*", "/", "<", ">", "=", "'", "@", "#", "^", "~",
        "`",
    ],
    ignored_keywords: &[
        "def",
        "defn",
        "defn-",
        "defmacro",
        "defonce",
        "defprotocol",
        "defrecord",
        "deftype",
        "defmulti",
        "defmethod",
        "definterface",
        "declare",
        "ns",
        "require",
        "import",
        "use",
        "refer",
        "as",
    ],
    name_bytes: b"-?!*+<>=/.'#",
    apostrophe: Apostrophe::Plain,
    ..TokenRules::PLAIN
};

/// Erlang tokenization rules.
///
/// `case`, `if`, `receive` and `try` branch, closed by `end`; `fun` makes a
/// closure and `!` sends a message. The word operators (`andalso`, `div`,
/// `band`, ...) compute. The names of attributes (`-module`, `-export`,
/// `-spec`) declare and are ignored.
pub static ERLANG: TokenRules = TokenRules {
    operator_keywords: &[
        "case", "of", "if", "receive", "after", "try", "catch", "end", "begin", "fun", "when",
        "andalso", "orelse", "and", "or", "not", "xor", "div", "rem", "band", "bor", "bxor",
        "bnot", "bsl", "bsr",
    ],
    operator_symbols: &[
        "=:=", "=/=", "...", "->", "<-", "<=", "=<", ">=", "==", "/=", "||", "++", "--", "::",
        ":=", "=>", "<<", ">>", "!", "+", "-", "*", "/", "<", ">", "=", ";", ",", ".", ":", "(",
        ")", "[", "]", "{", "}", "|", "#", "?",
    ],
    ignored_keywords: &[
        "module",
        "export",
        "export_type",
        "import",
        "spec",
        "type",
        "opaque",
        "record",
        "define",
        "include",
        "include_lib",
        "behaviour",
        "behavior",
        "callback",
        "compile",
        "ifdef",
        "ifndef",
        "endif",
    ],
    ..TokenRules::PLAIN
};

/// F# tokenization rules.
///
/// `match`, `function` and `with` match patterns, `|>` and `>>` pipe and
/// compose, `:?` tests a type and `:>` casts to one. `new` builds an
/// object. `let`, `fun`, `type`, `module`, `open`, `member` and the rest
/// declare and are ignored, as `and` is: it joins definitions. A name may
/// end in primes (`x'`).
pub static FSHARP: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "then", "elif", "else", "match", "with", "function", "for", "while", "do", "done",
        "to", "downto", "in", "yield", "return", "try", "finally", "raise", "when", "not", "new",
        "upcast", "downcast",
    ],
    operator_symbols: &[
        "<<<", ">>>", "|||", "&&&", "^^^", "||>", "<||", ":?>", "...", "|>", "<|", ">>", "<<",
        "::", ":?", ":>", "<-", "->", ":=", "<>", "<=", ">=", "&&", "||", "**", "..", "@", "+",
        "-", "*", "/", "%", "^", "!", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[", "]", "{",
        "}", "|", "?",
    ],
    ignored_keywords: &[
        "let",
        "rec",
        "mutable",
        "fun",
        "use",
        "and",
        "of",
        "type",
        "module",
        "namespace",
        "open",
        "member",
        "static",
        "abstract",
        "override",
        "default",
        "val",
        "inherit",
        "interface",
        "class",
        "struct",
        "end",
        "private",
        "internal",
        "public",
        "inline",
        "byte",
        "int64",
        "decimal",
        "seq",
    ],
    ..ML
};

/// Haskell tokenization rules.
///
/// `case`/`of`, `if`/`then`/`else` and `do` direct the flow; the operators
/// of monads and functors (`>>=`, `<$>`, `<*>`), composition (`.`) and
/// application (`$`) compute, as does a function between backticks
/// (`` `div` ``). `let`, `where` and `in` bind, and `data`, `newtype`,
/// `class`, `instance` and `import` declare: all ignored. Type signatures
/// are left out of the lines read. A name may end in primes (`x'`).
pub static HASKELL: TokenRules = TokenRules {
    operator_keywords: &["case", "of", "if", "then", "else", "do"],
    operator_symbols: &[
        ">>=", "=<<", "<$>", "<*>", "<|>", "<&>", ">=>", "<=<", "<>", ">>", "<-", "->", "=>", "::",
        "++", "!!", "==", "/=", "<=", ">=", "&&", "||", "<$", "$!", "..", "**", "^^", ".", "$",
        "+", "-", "*", "/", "^", "<", ">", "=", ",", ";", "(", ")", "[", "]", "{", "}", "|", "\\",
        "@", "!", "~", "&",
    ],
    ignored_keywords: &[
        "data",
        "newtype",
        "type",
        "class",
        "instance",
        "deriving",
        "import",
        "qualified",
        "as",
        "hiding",
        "module",
        "where",
        "let",
        "in",
        "infixl",
        "infixr",
        "infix",
        "forall",
        "Int",
        "Integer",
        "Bool",
        "String",
        "Char",
        "Double",
        "Float",
        "Maybe",
        "Either",
        "IO",
    ],
    name_bytes: b"'",
    infix: Some(b'`'),
    apostrophe: Apostrophe::Prime,
    ..TokenRules::PLAIN
};

/// Julia tokenization rules.
///
/// `end` closes blocks, as in Ruby and Lua, and `do` and `begin` open
/// them. The broadcast operators (`.+`, `.*`, ...), `|>`, `=>`, `::` as a
/// type assertion and the operators written in Unicode (`÷`, `≤`, `∈`) are
/// operators, and so is `'`, the adjoint, after an operand. A name may end
/// in `!` (`push!`). Definitions and type names are ignored.
pub static JULIA: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "elseif", "else", "for", "while", "return", "break", "continue", "try", "catch",
        "finally", "do", "begin", "end", "in", "isa",
    ],
    operator_symbols: &[
        ".+=", ".-=", ".*=", "./=", ".==", ">>>", "===", "!==", "...", "|>", "<|", "=>", "->",
        "::", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "^=", ".+", ".-", ".*",
        "./", ".^", ".<", ".>", "<<", ">>", "<:", ">:", "÷", "≤", "≥", "≠", "∈", "∉", "∘", "+",
        "-", "*", "/", "\\", "%", "^", "&", "|", "~", "!", "<", ">", "=", ";", ",", ".", ":", "(",
        ")", "[", "]", "{", "}", "?", "$", "@", "'",
    ],
    ignored_keywords: &[
        "function",
        "struct",
        "mutable",
        "module",
        "baremodule",
        "using",
        "import",
        "export",
        "const",
        "global",
        "local",
        "let",
        "abstract",
        "primitive",
        "type",
        "macro",
        "quote",
        "where",
        "Int",
        "Int32",
        "Int64",
        "Float32",
        "Float64",
        "String",
        "Bool",
        "Char",
        "Any",
        "Nothing",
        "Vector",
        "Array",
        "Dict",
    ],
    name_bytes: b"!",
    apostrophe: Apostrophe::Prime,
    ..TokenRules::PLAIN
};

/// Nim tokenization rules.
///
/// Blocks go by indentation, as in Python. `case`/`of`, `when` and
/// `try`/`except` branch; `discard` and `defer` act. The word operators
/// (`div`, `mod`, `shl`, `and`, `in`, `is`) compute. Routines, variables,
/// types and imports declare and are ignored.
pub static NIM: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "elif", "else", "case", "of", "when", "for", "while", "return", "break", "continue",
        "try", "except", "finally", "raise", "yield", "discard", "defer", "block", "and", "or",
        "not", "xor", "div", "mod", "shl", "shr", "in", "notin", "is", "isnot", "cast", "addr",
    ],
    operator_symbols: &[
        "..<", "...", "..", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=", "&=", "->", "=>", "@",
        "$", "&", "+", "-", "*", "/", "%", "<", ">", "=", ";", ",", ".", ":", "(", ")", "[", "]",
        "{", "}", "^", "~", "!", "|", "?",
    ],
    ignored_keywords: &[
        "proc",
        "func",
        "method",
        "iterator",
        "converter",
        "macro",
        "template",
        "let",
        "var",
        "const",
        "type",
        "object",
        "enum",
        "tuple",
        "ref",
        "ptr",
        "distinct",
        "import",
        "include",
        "from",
        "export",
        "using",
        "static",
        "concept",
        "float",
        "float32",
        "float64",
        "string",
        "bool",
        "char",
        "seq",
        "void",
    ],
    type_names: SIZED_INTEGERS,
    ..TokenRules::PLAIN
};

/// OCaml tokenization rules.
///
/// `match`, `function` and `with` match patterns; `|>` and `@@` apply,
/// `::` and `@` build lists, `^` joins strings, `:=` and `!` write and read
/// a reference, and the float operators (`+.`, `*.`) and word operators
/// (`mod`, `land`, `lsl`) compute. `let`, `in`, `type`, `module`, `open`
/// and the rest declare and are ignored, with `begin` and `end`. A name may
/// end in primes (`x'`).
pub static OCAML: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "then", "else", "match", "with", "function", "for", "while", "do", "done", "to",
        "downto", "try", "raise", "when", "assert", "lazy", "not", "new", "mod", "land", "lor",
        "lxor", "lsl", "lsr", "asr", "or",
    ],
    operator_symbols: &[
        "|>", "@@", "::", ":=", "+.", "-.", "*.", "/.", "**", "->", "<-", "<>", "==", "!=", "<=",
        ">=", "&&", "||", ";;", ":>", "@", "^", "!", "+", "-", "*", "/", "<", ">", "=", ";", ",",
        ".", ":", "(", ")", "[", "]", "{", "}", "|", "~", "?", "#", "&",
    ],
    ignored_keywords: &[
        "let",
        "rec",
        "in",
        "and",
        "fun",
        "type",
        "module",
        "open",
        "include",
        "struct",
        "sig",
        "begin",
        "end",
        "val",
        "external",
        "functor",
        "of",
        "mutable",
        "private",
        "virtual",
        "method",
        "object",
        "class",
        "inherit",
        "initializer",
        "constraint",
        "exception",
    ],
    ..ML
};

/// R tokenization rules.
///
/// `<-`, `<<-`, `->` and `->>` assign, `|>` pipes, `$` and `@` take a
/// member, `::` a name of a package, and every `%op%` (`%>%`, `%in%`,
/// `%%`) is one operator. A name goes on through `.` (`is.na`).
/// `function`, `library` and `require` declare and are ignored.
pub static R: TokenRules = TokenRules {
    operator_keywords: &[
        "if", "else", "for", "while", "repeat", "break", "next", "return", "in", "switch", "stop",
    ],
    operator_symbols: &[
        "<<-", "->>", ":::", "|>", "<-", "->", "==", "!=", "<=", ">=", "&&", "||", "::", "+", "-",
        "*", "/", "^", "!", "&", "|", "<", ">", "=", "~", "$", "@", ":", ";", ",", "(", ")", "[",
        "]", "{", "}", "?", "\\",
    ],
    ignored_keywords: &["function", "library", "require"],
    name_bytes: b".",
    infix: Some(b'%'),
    ..TokenRules::PLAIN
};

/// Look up the tokenization rules for a language by name.
/// Returns `None` for unsupported languages (no Halstead analysis available).
pub fn rules_for(language: &str) -> Option<&'static TokenRules> {
    match language {
        "Rust" => Some(&RUST),
        "Python" => Some(&PYTHON),
        "JavaScript" | "JSX" | "TypeScript" => Some(&JAVASCRIPT),
        "Go" => Some(&GO),
        "C" | "C++" | "C#" | "Java" | "Objective-C" | "PHP" | "Dart" => Some(&C_FAMILY),
        "Ruby" => Some(&RUBY),
        "Kotlin" => Some(&KOTLIN),
        "Swift" => Some(&SWIFT),
        "Bourne Shell" | "Bourne Again Shell" | "Zsh" => Some(&SHELL),
        "Kaikai" => Some(&KAIKAI),
        "Elixir" | "Elixir Script" => Some(&ELIXIR),
        name if is_fortran(name) => Some(&FORTRAN),
        "Pascal" => Some(&PASCAL),
        "Lua" => Some(&LUA),
        "Perl" => Some(&PERL),
        "Scala" => Some(&SCALA),
        "Zig" => Some(&ZIG),
        "Clojure" => Some(&CLOJURE),
        "Erlang" => Some(&ERLANG),
        "F#" => Some(&FSHARP),
        "Haskell" => Some(&HASKELL),
        "Julia" => Some(&JULIA),
        "Nim" => Some(&NIM),
        "OCaml" => Some(&OCAML),
        "R" => Some(&R),
        _ => None,
    }
}
