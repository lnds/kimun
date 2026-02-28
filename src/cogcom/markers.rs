/// Language-specific tokens used to compute cognitive complexity.
///
/// Cognitive complexity (SonarSource, 2017) measures the difficulty of
/// understanding code. Unlike cyclomatic complexity, it penalizes nesting
/// depth and rewards linear control flow. Keywords are classified into
/// three categories that determine how they contribute to the score.
pub struct CognitiveMarkers {
    /// Structural keywords: +1 + current nesting depth. These represent
    /// control flow that requires mental effort to follow. Each one also
    /// increases nesting depth for its body.
    /// (e.g. `if`, `for`, `while`, `match`, `catch`)
    pub structural_keywords: &'static [&'static str],
    /// Hybrid keywords: +1 only, no nesting increment. These continue
    /// a linear chain and don't add cognitive burden from nesting.
    /// (e.g. `else if`, `elif`, `elsif`)
    pub hybrid_keywords: &'static [&'static str],
    /// Fundamental keywords: +1 only, but DO increment nesting for their body.
    /// (e.g. `else`)
    pub fundamental_keywords: &'static [&'static str],
    /// Boolean operators for sequence detection. Each *change* between
    /// operator types adds +1 (e.g. `a && b && c` = +1, `a && b || c` = +2).
    pub boolean_operators: &'static [&'static str],
    /// Tokens that mark function definitions (reused from cycom).
    pub function_markers: &'static [&'static str],
    /// Whether functions are delimited by braces (true) or indentation (false).
    pub brace_scoped: bool,
    /// Line comment markers for this language.
    pub line_comments: &'static [&'static str],
}

static RUST: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "loop", "match"],
    hybrid_keywords: &["else if"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &["fn "],
    brace_scoped: true,
    line_comments: &["//"],
};

static PYTHON: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "except"],
    hybrid_keywords: &["elif"],
    fundamental_keywords: &["else"],
    boolean_operators: &["and", "or"],
    function_markers: &["async def ", "def "],
    brace_scoped: false,
    line_comments: &["#"],
};

static JAVASCRIPT: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "do", "switch", "catch"],
    hybrid_keywords: &["else if"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||", "??"],
    function_markers: &["function "],
    brace_scoped: true,
    line_comments: &["//"],
};

static C_FAMILY: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "do", "switch", "catch"],
    hybrid_keywords: &["else if"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &[],
    brace_scoped: true,
    line_comments: &["//"],
};

static GO: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "switch", "select"],
    hybrid_keywords: &["else if"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &["func "],
    brace_scoped: true,
    line_comments: &["//"],
};

static RUBY: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "unless", "for", "while", "until", "rescue"],
    hybrid_keywords: &["elsif"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &["def "],
    brace_scoped: false,
    line_comments: &["#"],
};

static KOTLIN: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "when", "catch"],
    hybrid_keywords: &["else if"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &["fun "],
    brace_scoped: true,
    line_comments: &["//"],
};

static SWIFT: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "switch", "catch", "guard"],
    hybrid_keywords: &["else if"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &["func "],
    brace_scoped: true,
    line_comments: &["//"],
};

static SCALA: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "match", "catch"],
    hybrid_keywords: &["else if"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &["def "],
    brace_scoped: true,
    line_comments: &["//"],
};

static SHELL: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "until", "case"],
    hybrid_keywords: &["elif"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &[],
    brace_scoped: false,
    line_comments: &["#"],
};

static HASKELL: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "case"],
    hybrid_keywords: &[],
    fundamental_keywords: &["else"],
    boolean_operators: &[],
    function_markers: &[],
    brace_scoped: false,
    line_comments: &["--"],
};

static ELIXIR: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "cond", "case", "for", "rescue"],
    hybrid_keywords: &[],
    fundamental_keywords: &["else"],
    boolean_operators: &[],
    function_markers: &["defp ", "def "],
    brace_scoped: false,
    line_comments: &["#"],
};

static LUA: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while"],
    hybrid_keywords: &["elseif"],
    fundamental_keywords: &["else"],
    boolean_operators: &[],
    function_markers: &["function "],
    brace_scoped: false,
    line_comments: &["--"],
};

static PERL: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "foreach", "while", "unless"],
    hybrid_keywords: &["elsif"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &["sub "],
    brace_scoped: true,
    line_comments: &["#"],
};

static ERLANG: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "case", "receive"],
    hybrid_keywords: &[],
    fundamental_keywords: &[],
    boolean_operators: &[],
    function_markers: &[],
    brace_scoped: false,
    line_comments: &["%"],
};

static OCAML: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "match"],
    hybrid_keywords: &[],
    fundamental_keywords: &["else"],
    boolean_operators: &[],
    function_markers: &["let "],
    brace_scoped: false,
    line_comments: &[],
};

static R: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "repeat"],
    hybrid_keywords: &["else if"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &[],
    brace_scoped: true,
    line_comments: &["#"],
};

static JULIA: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while"],
    hybrid_keywords: &["elseif"],
    fundamental_keywords: &["else"],
    boolean_operators: &["&&", "||"],
    function_markers: &["function "],
    brace_scoped: false,
    line_comments: &["#"],
};

static NIM: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "case", "except"],
    hybrid_keywords: &["elif"],
    fundamental_keywords: &["else"],
    boolean_operators: &[],
    function_markers: &["proc ", "func "],
    brace_scoped: false,
    line_comments: &["#"],
};

static ZIG: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "for", "while", "switch"],
    hybrid_keywords: &[],
    fundamental_keywords: &["else"],
    boolean_operators: &[],
    function_markers: &["fn "],
    brace_scoped: true,
    line_comments: &["//"],
};

static CLOJURE: CognitiveMarkers = CognitiveMarkers {
    structural_keywords: &["if", "cond", "case", "when"],
    hybrid_keywords: &[],
    fundamental_keywords: &[],
    boolean_operators: &[],
    function_markers: &["defn "],
    brace_scoped: false,
    line_comments: &[";"],
};

/// Look up the cognitive complexity markers for a given language name.
///
/// Returns `None` for languages without cognitive complexity support
/// (data/markup formats with no control flow).
pub fn cognitive_markers_for(language_name: &str) -> Option<&'static CognitiveMarkers> {
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
