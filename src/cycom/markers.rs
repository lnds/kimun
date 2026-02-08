pub struct ComplexityMarkers {
    pub keywords: &'static [&'static str],
    pub operators: &'static [&'static str],
    pub function_markers: &'static [&'static str],
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
mod tests {
    use super::*;

    #[test]
    fn rust_markers_exist() {
        let m = markers_for("Rust").unwrap();
        assert!(m.brace_scoped);
        assert!(m.keywords.contains(&"if"));
        assert!(m.function_markers.contains(&"fn "));
    }

    #[test]
    fn json_returns_none() {
        assert!(markers_for("JSON").is_none());
    }

    #[test]
    fn html_returns_none() {
        assert!(markers_for("HTML").is_none());
    }

    #[test]
    fn python_is_indent_scoped() {
        let m = markers_for("Python").unwrap();
        assert!(!m.brace_scoped);
        assert!(m.keywords.contains(&"elif"));
    }

    #[test]
    fn c_family_shared() {
        let java = markers_for("Java").unwrap();
        let c = markers_for("C").unwrap();
        assert!(std::ptr::eq(java, c));
    }

    #[test]
    fn shell_variants() {
        assert!(markers_for("Bourne Shell").is_some());
        assert!(markers_for("Bourne Again Shell").is_some());
        assert!(markers_for("Zsh").is_some());
    }

    #[test]
    fn unknown_language_returns_none() {
        assert!(markers_for("Unknown").is_none());
        assert!(markers_for("Markdown").is_none());
        assert!(markers_for("TOML").is_none());
    }
}
