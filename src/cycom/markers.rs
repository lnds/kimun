/// Language-specific tokens used to compute cyclomatic complexity.
///
/// Cyclomatic complexity counts independent paths through code. Each
/// keyword or operator that creates a branch adds 1 to the complexity.
/// The classification below determines what counts as a branch point
/// for each language.
pub struct ComplexityMarkers {
    /// Branch keywords that increase complexity (e.g. "if", "for", "while").
    /// Multi-word entries like "else if" must appear before their prefixes.
    pub keywords: &'static [&'static str],
    /// Boolean operators that add decision points (e.g. "&&", "||").
    pub operators: &'static [&'static str],
    /// Tokens that mark function definitions (e.g. "fn ", "def ").
    /// Each function starts with a base complexity of 1.
    pub function_markers: &'static [&'static str],
    /// Whether functions are delimited by braces (true) or indentation (false).
    pub brace_scoped: bool,
    /// Line comment markers for this language (used to strip comment text
    /// before masking strings, preventing unmatched quotes in comments from
    /// confusing the string masker).
    pub line_comments: &'static [&'static str],
}

/// Rust: `loop` is a keyword (unconditional loop = 1 path). `match` adds
/// 1 per arm via the `match` keyword itself (arms are counted separately
/// by the analyzer). `&&`/`||` are short-circuit operators that create
/// additional decision points within boolean expressions.
static RUST: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "while", "loop", "match"],
    operators: &["&&", "||"],
    function_markers: &["fn "],
    brace_scoped: true,
    line_comments: &["//"],
};

/// Python: `brace_scoped` is false because Python uses indentation for
/// scoping, not braces. Boolean operators `and`/`or` are keywords (not
/// symbols) so they appear in `keywords` rather than `operators`.
/// `except` is a branch (exception handler path). `async def` must
/// appear before `def` in function_markers to match first.
static PYTHON: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elif", "if", "for", "while", "except", "and", "or"],
    operators: &[],
    function_markers: &["async def ", "def "],
    brace_scoped: false,
    line_comments: &["#"],
};

/// JavaScript/TypeScript: `??` (nullish coalescing) is an operator that
/// creates a branch (null check path). `case` in switch statements each
/// adds a decision point. Arrow functions are detected separately by the
/// analyzer, so only `function` appears in function_markers.
static JAVASCRIPT: ComplexityMarkers = ComplexityMarkers {
    keywords: &[
        "else if", "if", "for", "while", "do", "switch", "case", "catch",
    ],
    operators: &["&&", "||", "??"],
    function_markers: &["function "],
    brace_scoped: true,
    line_comments: &["//"],
};

/// C-family (C, C++, Java, C#, Objective-C, PHP, Dart): no function_markers
/// because function definitions vary too widely across these languages
/// (return type before name in C/C++, annotations in Java, etc.).
/// Function detection falls back to brace-counting heuristics instead.
static C_FAMILY: ComplexityMarkers = ComplexityMarkers {
    keywords: &[
        "else if", "if", "for", "while", "do", "switch", "case", "catch",
    ],
    operators: &["&&", "||"],
    function_markers: &[],
    brace_scoped: true,
    line_comments: &["//"],
};

/// Go: `select` is included because it multiplexes channel operations,
/// creating one path per `case`. Go has no `while` or `do` — `for` covers
/// all loop forms. There is no ternary operator in Go.
static GO: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "switch", "case", "select"],
    operators: &["&&", "||"],
    function_markers: &["func "],
    brace_scoped: true,
    line_comments: &["//"],
};

/// Ruby: `brace_scoped` is false because Ruby uses `end` for scoping.
/// `unless` and `until` are negated conditionals/loops that each create
/// a branch. `when` is Ruby's case-arm keyword. `rescue` handles
/// exceptions (an alternative execution path).
static RUBY: ComplexityMarkers = ComplexityMarkers {
    keywords: &[
        "elsif", "if", "unless", "for", "while", "until", "when", "rescue",
    ],
    operators: &["&&", "||"],
    function_markers: &["def "],
    brace_scoped: false,
    line_comments: &["#"],
};

static KOTLIN: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "while", "when", "catch"],
    operators: &["&&", "||"],
    function_markers: &["fun "],
    brace_scoped: true,
    line_comments: &["//"],
};

static SWIFT: ComplexityMarkers = ComplexityMarkers {
    keywords: &[
        "else if", "if", "for", "while", "switch", "case", "catch", "guard",
    ],
    operators: &["&&", "||"],
    function_markers: &["func "],
    brace_scoped: true,
    line_comments: &["//"],
};

static SCALA: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "while", "match", "case", "catch"],
    operators: &["&&", "||"],
    function_markers: &["def "],
    brace_scoped: true,
    line_comments: &["//"],
};

static SHELL: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elif", "if", "for", "while", "until", "case"],
    operators: &["&&", "||"],
    function_markers: &[],
    brace_scoped: false,
    line_comments: &["#"],
};

/// Haskell: minimal markers because most branching uses pattern matching
/// (which doesn't use keywords captured here). Only `if` and `case`
/// expressions add complexity. No function markers — Haskell function
/// definitions are identified by the `=` sign at the top level, which
/// is too ambiguous for simple keyword matching.
static HASKELL: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "case"],
    operators: &[],
    function_markers: &[],
    brace_scoped: false,
    line_comments: &["--"],
};

/// Elixir: `cond` is a multi-way conditional (like chained if-else).
/// `defp` (private function) must appear before `def` in function_markers
/// to match first. No boolean operators — Elixir uses `and`/`or` which
/// are already covered as keywords would be, but these are less common
/// than pattern matching for branching.
static ELIXIR: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "cond", "case", "for", "rescue"],
    operators: &[],
    function_markers: &["defp ", "def "],
    brace_scoped: false,
    line_comments: &["#"],
};

static LUA: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elseif", "if", "for", "while"],
    operators: &[],
    function_markers: &["function "],
    brace_scoped: false,
    line_comments: &["--"],
};

static PERL: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elsif", "if", "for", "foreach", "while", "unless"],
    operators: &["&&", "||"],
    function_markers: &["sub "],
    brace_scoped: true,
    line_comments: &["#"],
};

static ERLANG: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "case", "receive"],
    operators: &[],
    function_markers: &[],
    brace_scoped: false,
    line_comments: &["%"],
};

static OCAML: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "match", "with"],
    operators: &[],
    function_markers: &["let "],
    brace_scoped: false,
    line_comments: &[],
};

static R: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "while", "repeat"],
    operators: &["&&", "||"],
    function_markers: &[],
    brace_scoped: true,
    line_comments: &["#"],
};

static JULIA: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elseif", "if", "for", "while"],
    operators: &["&&", "||"],
    function_markers: &["function "],
    brace_scoped: false,
    line_comments: &["#"],
};

static NIM: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elif", "if", "for", "while", "case", "except"],
    operators: &[],
    function_markers: &["proc ", "func "],
    brace_scoped: false,
    line_comments: &["#"],
};

static ZIG: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else", "if", "for", "while", "switch"],
    operators: &[],
    function_markers: &["fn "],
    brace_scoped: true,
    line_comments: &["//"],
};

static CLOJURE: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "cond", "case", "when"],
    operators: &[],
    function_markers: &["defn "],
    brace_scoped: false,
    line_comments: &[";"],
};

/// Look up the complexity markers for a given language name.
///
/// Returns `None` for languages without cyclomatic complexity support
/// (e.g. JSON, HTML, Markdown — these are data/markup formats with no
/// control flow). Languages that share similar syntax are grouped:
/// C/C++/Java/C#/ObjC/PHP/Dart all use `C_FAMILY`, and OCaml/F# share
/// the `OCAML` markers.
pub fn markers_for(language_name: &str) -> Option<&'static ComplexityMarkers> {
    match language_name {
        "Rust" => Some(&RUST),
        "Python" => Some(&PYTHON),
        "JavaScript" | "TypeScript" => Some(&JAVASCRIPT),
        "Java" | "C#" | "C" | "C++" | "Objective-C" | "PHP" | "Dart" => Some(&C_FAMILY),
        "Go" => Some(&GO),
        "Ruby" => Some(&RUBY),
        "Kotlin" => Some(&KOTLIN),
        "Swift" => Some(&SWIFT),
        "Scala" => Some(&SCALA),
        "Bourne Shell" | "Bourne Again Shell" | "Zsh" => Some(&SHELL),
        "Haskell" => Some(&HASKELL),
        "Elixir" | "Elixir Script" => Some(&ELIXIR),
        "Lua" => Some(&LUA),
        "Perl" => Some(&PERL),
        "Erlang" => Some(&ERLANG),
        "OCaml" | "F#" => Some(&OCAML),
        "R" => Some(&R),
        "Julia" => Some(&JULIA),
        "Nim" => Some(&NIM),
        "Zig" => Some(&ZIG),
        "Clojure" => Some(&CLOJURE),
        _ => None,
    }
}

#[cfg(test)]
#[path = "markers_test.rs"]
mod tests;
