/// Language-specific tokens used to compute cyclomatic complexity.
/// Each field defines the syntactic elements that increase branching.
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
}

static RUST: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "while", "loop", "match"],
    operators: &["&&", "||"],
    function_markers: &["fn "],
    brace_scoped: true,
};

static PYTHON: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elif", "if", "for", "while", "except", "and", "or"],
    operators: &[],
    function_markers: &["async def ", "def "],
    brace_scoped: false,
};

static JAVASCRIPT: ComplexityMarkers = ComplexityMarkers {
    keywords: &[
        "else if", "if", "for", "while", "do", "switch", "case", "catch",
    ],
    operators: &["&&", "||", "??"],
    function_markers: &["function "],
    brace_scoped: true,
};

static C_FAMILY: ComplexityMarkers = ComplexityMarkers {
    keywords: &[
        "else if", "if", "for", "while", "do", "switch", "case", "catch",
    ],
    operators: &["&&", "||"],
    function_markers: &[],
    brace_scoped: true,
};

static GO: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "switch", "case", "select"],
    operators: &["&&", "||"],
    function_markers: &["func "],
    brace_scoped: true,
};

static RUBY: ComplexityMarkers = ComplexityMarkers {
    keywords: &[
        "elsif", "if", "unless", "for", "while", "until", "when", "rescue",
    ],
    operators: &["&&", "||"],
    function_markers: &["def "],
    brace_scoped: false,
};

static KOTLIN: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "while", "when", "catch"],
    operators: &["&&", "||"],
    function_markers: &["fun "],
    brace_scoped: true,
};

static SWIFT: ComplexityMarkers = ComplexityMarkers {
    keywords: &[
        "else if", "if", "for", "while", "switch", "case", "catch", "guard",
    ],
    operators: &["&&", "||"],
    function_markers: &["func "],
    brace_scoped: true,
};

static SCALA: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "while", "match", "case", "catch"],
    operators: &["&&", "||"],
    function_markers: &["def "],
    brace_scoped: true,
};

static SHELL: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elif", "if", "for", "while", "until", "case"],
    operators: &["&&", "||"],
    function_markers: &[],
    brace_scoped: false,
};

static HASKELL: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "case"],
    operators: &[],
    function_markers: &[],
    brace_scoped: false,
};

static ELIXIR: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "cond", "case", "for", "rescue"],
    operators: &[],
    function_markers: &["defp ", "def "],
    brace_scoped: false,
};

static LUA: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elseif", "if", "for", "while"],
    operators: &[],
    function_markers: &["function "],
    brace_scoped: false,
};

static PERL: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elsif", "if", "for", "foreach", "while", "unless"],
    operators: &["&&", "||"],
    function_markers: &["sub "],
    brace_scoped: true,
};

static ERLANG: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "case", "receive"],
    operators: &[],
    function_markers: &[],
    brace_scoped: false,
};

static OCAML: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "match", "with"],
    operators: &[],
    function_markers: &["let "],
    brace_scoped: false,
};

static R: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else if", "if", "for", "while", "repeat"],
    operators: &["&&", "||"],
    function_markers: &[],
    brace_scoped: true,
};

static JULIA: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elseif", "if", "for", "while"],
    operators: &["&&", "||"],
    function_markers: &["function "],
    brace_scoped: false,
};

static NIM: ComplexityMarkers = ComplexityMarkers {
    keywords: &["elif", "if", "for", "while", "case", "except"],
    operators: &[],
    function_markers: &["proc ", "func "],
    brace_scoped: false,
};

static ZIG: ComplexityMarkers = ComplexityMarkers {
    keywords: &["else", "if", "for", "while", "switch"],
    operators: &[],
    function_markers: &["fn "],
    brace_scoped: true,
};

static CLOJURE: ComplexityMarkers = ComplexityMarkers {
    keywords: &["if", "cond", "case", "when"],
    operators: &[],
    function_markers: &["defn "],
    brace_scoped: false,
};

/// Look up the complexity markers for a given language name.
/// Returns `None` for languages without cyclomatic complexity support
/// (e.g. JSON, HTML, Markdown).
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
