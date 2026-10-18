//! Language specification registry and detection.
//!
//! Defines 40+ programming languages via the `lang!` macro, each with
//! file extension/filename mappings, comment syntax (line, block, nested),
//! string delimiter rules, pragma support, and shebang patterns.
//! Language detection uses a two-phase lookup: first by filename (for
//! files like `Makefile`, `CMakeLists.txt`), then by extension. Shebang
//! detection is the final fallback for extensionless scripts.
use std::path::Path;

use super::lang_macro::lang_spec;

/// Specification of a programming language's syntax for line classification.
///
/// Each field controls how the FSM in `counter.rs` identifies comments,
/// strings, and pragmas. Languages are detected by extension, filename,
/// or shebang line.
#[derive(Debug)]
pub struct LanguageSpec {
    /// Display name (e.g. "Rust", "Python", "JavaScript").
    pub name: &'static str,
    /// File extensions that identify this language (without leading dot).
    pub extensions: &'static [&'static str],
    /// Exact filenames that identify this language (e.g. "Makefile").
    pub filenames: &'static [&'static str],
    /// Line comment markers (e.g. `["//"]` or `["#"]`).
    pub line_comments: &'static [&'static str],
    /// Characters that, if immediately following a line comment marker,
    /// prevent it from being treated as a comment. Used for Haskell where
    /// `-->` is an operator but `-- comment` is a comment.
    pub line_comment_not_before: &'static str,
    /// Block comment delimiters: `Some(("/*", "*/"))` or `None`.
    pub block_comment: Option<(&'static str, &'static str)>,
    /// Whether block comments can nest (e.g. Rust `/* /* */ */`).
    pub nested_block_comments: bool,
    /// Whether single quotes delimit strings (false for Rust lifetimes).
    pub single_quote_strings: bool,
    /// Whether triple quotes delimit multi-line strings (Python).
    pub triple_quote_strings: bool,
    /// Whether `r"…"` and `r#"…"#` are raw strings: no escapes, and one
    /// opened with `#` ends only at a quote followed by as many (Rust).
    pub raw_strings: bool,
    /// Pragma delimiters that override block comment detection (Haskell).
    pub pragma: Option<(&'static str, &'static str)>,
    /// Documentation attribute delimiters, opening and closing, e.g.
    /// Kaikai's `#[doc(` … `)]` or Elixir's `@doc """` … `"""`. Their body
    /// is documentation, so it is classified as comment even though it is
    /// written as a string literal.
    pub doc_attributes: &'static [(&'static str, &'static str)],
    /// A second kind of block comment, which the first does not close:
    /// Pascal's `(*` … `*)` besides `{` … `}`.
    pub other_block_comment: Option<(&'static str, &'static str)>,
    /// Characters that make a comment of the whole line when they stand in
    /// its first column (Fortran's fixed form: `C`, `c`, `*`), unless a
    /// directive follows.
    pub column_one_comments: &'static str,
    /// Whether keywords and names are the same whatever their case
    /// (Fortran, Pascal). Analyzers read such code in lower case.
    pub case_insensitive: bool,
    /// Whether a backslash escapes the next character in a quoted string.
    /// Not in Fortran nor Pascal, where `'\'` is a string of one character
    /// and a quote is escaped by doubling it.
    pub backslash_escapes: bool,
    /// What makes a directive of a comment when it follows the comment
    /// marker, in any case: Fortran's `!$omp`, `!$acc`, and `!$ ` for
    /// conditional compilation. A directive is code.
    pub directives: &'static [&'static str],
    /// Shebang interpreter names for extensionless script detection.
    pub shebangs: &'static [&'static str],
}

impl LanguageSpec {
    /// A language without comments, whose strings are double-quoted and
    /// escape with a backslash: the base the specs written by hand start
    /// from.
    pub const PLAIN: LanguageSpec = LanguageSpec {
        name: "",
        extensions: &[],
        filenames: &[],
        line_comments: &[],
        line_comment_not_before: "",
        block_comment: None,
        nested_block_comments: false,
        single_quote_strings: false,
        triple_quote_strings: false,
        raw_strings: false,
        pragma: None,
        doc_attributes: &[],
        other_block_comment: None,
        column_one_comments: "",
        case_insensitive: false,
        backslash_escapes: true,
        directives: &[],
        shebangs: &[],
    };
}

/// Whether a language is one of the Fortrans, which cloc names by standard.
pub fn is_fortran(language: &str) -> bool {
    language.starts_with("Fortran")
}

/// The heredocs of `@moduledoc` and `@doc`, plain or as a sigil.
const ELIXIR_DOCS: &[(&str, &str)] = &[
    ("@moduledoc \"\"\"", "\"\"\""),
    ("@moduledoc ~S\"\"\"", "\"\"\""),
    ("@moduledoc ~s\"\"\"", "\"\"\""),
    ("@moduledoc '''", "'''"),
    ("@doc \"\"\"", "\"\"\""),
    ("@doc ~S\"\"\"", "\"\"\""),
    ("@doc ~s\"\"\"", "\"\"\""),
    ("@doc '''", "'''"),
];

const ELIXIR_SCRIPT: LanguageSpec = LanguageSpec {
    name: "Elixir Script",
    extensions: &["exs"],
    line_comments: &["#"],
    single_quote_strings: true,
    triple_quote_strings: true,
    doc_attributes: ELIXIR_DOCS,
    ..LanguageSpec::PLAIN
};

/// Fortran in free form, the base of every Fortran. OpenMP and OpenACC
/// directives (`!$omp`, `!$acc`) and conditional compilation (`!$ `) are
/// code.
const FORTRAN_90: LanguageSpec = LanguageSpec {
    name: "Fortran 90",
    extensions: &["f90", "F90"],
    line_comments: &["!"],
    single_quote_strings: true,
    case_insensitive: true,
    backslash_escapes: false,
    directives: &["$omp", "$acc", "$ "],
    ..LanguageSpec::PLAIN
};

/// Return the static registry of all supported language specifications.
pub fn languages() -> &'static [LanguageSpec] {
    static LANGUAGES: &[LanguageSpec] = &[
        // Rust is defined manually because of its raw strings, which the
        // macro does not carry.
        LanguageSpec {
            name: "Rust",
            extensions: &["rs"],
            line_comments: &["//"],
            block_comment: Some(("/*", "*/")),
            nested_block_comments: true,
            raw_strings: true,
            ..LanguageSpec::PLAIN
        },
        lang_spec!("Python", ext: ["py", "pyi"],
              line: "#", sq: true, tq: true,
              shebangs: ["python", "python3"]),
        lang_spec!("JavaScript", ext: ["js", "mjs", "cjs"],
              line: "//", block: "/*", "*/", sq: true,
              shebangs: ["node"]),
        lang_spec!("JSX", ext: ["jsx"],
              line: "//", block: "/*", "*/", sq: true),
        lang_spec!("TypeScript", ext: ["ts", "tsx", "mts", "cts"],
              line: "//", block: "/*", "*/", sq: true),
        lang_spec!("Java", ext: ["java"],
              line: "//", block: "/*", "*/"),
        lang_spec!("C", ext: ["c", "h"],
              line: "//", block: "/*", "*/"),
        lang_spec!("C++", ext: ["cpp", "cxx", "cc", "c++", "hpp", "hxx", "hh", "h++", "inl", "ipp", "tpp"],
              line: "//", block: "/*", "*/"),
        lang_spec!("C#", ext: ["cs"],
              line: "//", block: "/*", "*/"),
        lang_spec!("Go", ext: ["go"],
              line: "//", block: "/*", "*/"),
        lang_spec!("Ruby", ext: ["rb"], files: ["Rakefile", "Gemfile"],
              line: "#", sq: true,
              shebangs: ["ruby"]),
        lang_spec!("Bourne Shell", ext: ["sh"],
              line: "#", sq: true,
              shebangs: ["sh"]),
        lang_spec!("Bourne Again Shell", ext: ["bash"],
              line: "#", sq: true,
              shebangs: ["bash"]),
        lang_spec!("Zsh", ext: ["zsh"],
              line: "#", sq: true,
              shebangs: ["zsh"]),
        lang_spec!("HTML", ext: ["html", "htm"],
              block: "<!--", "-->", sq: true),
        lang_spec!("CSS", ext: ["css"],
              block: "/*", "*/", sq: true),
        lang_spec!("SQL", ext: ["sql"],
              line: "--", block: "/*", "*/", sq: true),
        lang_spec!("TOML", ext: ["toml"],
              line: "#"),
        lang_spec!("YAML", ext: ["yaml", "yml"],
              line: "#"),
        lang_spec!("JSON", ext: ["json"],
              none),
        lang_spec!("Markdown", ext: ["md", "markdown"],
              none),
        lang_spec!("Kotlin", ext: ["kt", "kts"],
              line: "//", block: "/*", "*/", nested: true),
        lang_spec!("Swift", ext: ["swift"],
              line: "//", block: "/*", "*/", nested: true),
        lang_spec!("PHP", ext: ["php"],
              line: "//", block: "/*", "*/", sq: true),
        lang_spec!("Dart", ext: ["dart"],
              line: "//", block: "/*", "*/", sq: true),
        // Haskell is defined manually (not via lang! macro) because it needs
        // `line_comment_not_before` to prevent `-->` from being treated as a
        // `--` comment. The macro does not support this field.
        LanguageSpec {
            name: "Haskell",
            extensions: &["hs"],
            line_comments: &["--"],
            line_comment_not_before: "!#$%&*+./<=>?@\\^|~",
            block_comment: Some(("{-", "-}")),
            nested_block_comments: true,
            pragma: Some(("{-#", "#-}")),
            ..LanguageSpec::PLAIN
        },
        lang_spec!("Lua", ext: ["lua"],
              line: "--", block: "--[[", "]]", sq: true,
              shebangs: ["lua"]),
        lang_spec!("Perl", ext: ["pl", "pm"],
              line: "#", sq: true,
              shebangs: ["perl"]),
        lang_spec!("R", ext: ["r", "R"],
              line: "#", sq: true,
              shebangs: ["Rscript"]),
        lang_spec!("Scala", ext: ["scala", "sc", "sbt"],
              line: "//", block: "/*", "*/", nested: true),
        lang_spec!("XML", ext: ["xml", "xsl", "xslt", "svg", "fsproj", "csproj", "vbproj", "vcxproj", "sln", "plist", "xaml"],
              block: "<!--", "-->", sq: true),
        lang_spec!("Dockerfile", ext: [], files: ["Dockerfile"],
              line: "#"),
        lang_spec!("Makefile", ext: ["mk"], files: ["Makefile", "makefile", "GNUmakefile"],
              line: "#"),
        // Elixir is defined manually because its `@moduledoc` and `@doc`
        // heredocs are documentation, as cloc counts them.
        LanguageSpec {
            name: "Elixir",
            extensions: &["ex"],
            shebangs: &["elixir"],
            ..ELIXIR_SCRIPT
        },
        ELIXIR_SCRIPT,
        lang_spec!("Clojure", ext: ["clj", "cljs", "cljc", "edn"],
              line: ";"),
        lang_spec!("Zig", ext: ["zig"],
              line: "//"),
        // Kaikai is defined manually because it needs two fields the macro
        // does not carry: `#[` opens an attribute (not a `#` comment), and
        // `#[doc( ... )]` wraps documentation that must not count as code.
        LanguageSpec {
            name: "Kaikai",
            extensions: &["kai"],
            line_comments: &["#"],
            line_comment_not_before: "[",
            triple_quote_strings: true,
            doc_attributes: &[("#[doc(", ")]")],
            ..LanguageSpec::PLAIN
        },
        lang_spec!("Objective-C", ext: ["m", "mm"],
              line: "//", block: "/*", "*/"),
        lang_spec!("OCaml", ext: ["ml", "mli"],
              block: "(*", "*)", nested: true),
        lang_spec!("F#", ext: ["fs", "fsi", "fsx"],
              line: "//", block: "(*", "*)", nested: true),
        lang_spec!("Nim", ext: ["nim"],
              line: "#", block: "#[", "]#", nested: true),
        lang_spec!("Julia", ext: ["jl"],
              line: "#", block: "#=", "=#", nested: true,
              shebangs: ["julia"]),
        lang_spec!("Terraform", ext: ["tf"],
              line: "#", block: "/*", "*/"),
        lang_spec!("Groovy", ext: ["groovy"],
              line: "//", block: "/*", "*/", sq: true),
        lang_spec!("Gradle", ext: ["gradle"],
              line: "//", block: "/*", "*/", sq: true),
        lang_spec!("Erlang", ext: ["erl", "hrl"],
              line: "%"),
        lang_spec!("HTML EEx", ext: ["heex"],
              block: "<!--", "-->", sq: true),
        lang_spec!("PO File", ext: ["po", "pot"],
              line: "#"),
        lang_spec!("DOS Batch", ext: ["bat", "cmd"],
              lines: ["::", "rem ", "REM ", "Rem "]),
        lang_spec!("Properties", ext: ["properties"],
              line: "#"),
        // Fortran is defined manually: it ignores case, and its fixed form
        // makes a comment of a line by the first column alone.
        LanguageSpec {
            name: "Fortran 77",
            extensions: &["f", "F", "for", "FOR", "ftn", "FTN", "f77", "F77", "pfo"],
            column_one_comments: "Cc*",
            ..FORTRAN_90
        },
        FORTRAN_90,
        LanguageSpec {
            name: "Fortran 95",
            extensions: &["f95", "F95"],
            ..FORTRAN_90
        },
        LanguageSpec {
            name: "Fortran 2003",
            extensions: &["f03", "F03"],
            ..FORTRAN_90
        },
        LanguageSpec {
            name: "Fortran 2008",
            extensions: &["f08", "F08"],
            ..FORTRAN_90
        },
        // Pascal (Delphi, Free Pascal) is defined manually: it has two kinds
        // of block comment, `{$…}` is a compiler directive, and it ignores
        // case.
        LanguageSpec {
            name: "Pascal",
            extensions: &["pas", "dpr", "dpk", "lpr", "pp", "pascal"],
            line_comments: &["//"],
            block_comment: Some(("{", "}")),
            other_block_comment: Some(("(*", "*)")),
            single_quote_strings: true,
            pragma: Some(("{$", "}")),
            case_insensitive: true,
            backslash_escapes: false,
            ..LanguageSpec::PLAIN
        },
        lang_spec!("Text", ext: ["txt"],
              none),
    ];
    LANGUAGES
}

/// Languages that hold data or prose rather than code. The metrics of
/// code (complexity, indentation, change-proneness, duplication) leave
/// them out; line counts keep them.
const DATA_AND_PROSE: &[&str] = &[
    "JSON",
    "YAML",
    "TOML",
    "XML",
    "Markdown",
    "Text",
    "Properties",
    "PO File",
];

impl LanguageSpec {
    /// Whether files of this language are code, not data or prose.
    pub fn is_code(&self) -> bool {
        !DATA_AND_PROSE.contains(&self.name)
    }
}

/// Detect the language of a file by matching its filename or extension
/// against the language registry. Returns `None` for unrecognized files.
pub fn detect(path: &Path) -> Option<&'static LanguageSpec> {
    let file_name = path.file_name()?.to_str()?;

    for spec in languages() {
        if spec.filenames.contains(&file_name) {
            return Some(spec);
        }
    }

    let ext = path.extension()?.to_str()?;
    languages()
        .iter()
        .find(|spec| spec.extensions.contains(&ext))
}

/// Detect the language from a shebang line (e.g. `#!/usr/bin/env python3`).
/// Handles both direct paths and `env` wrappers with flags.
pub fn detect_by_shebang(first_line: &str) -> Option<&'static LanguageSpec> {
    let line = first_line.trim();
    if !line.starts_with("#!") {
        return None;
    }

    // Extract the interpreter name from patterns like:
    //   #!/usr/bin/env python3
    //   #!/usr/bin/python
    //   #!/bin/bash
    let interpreter = line
        .rsplit('/')
        .next()
        .unwrap_or("")
        .split_whitespace()
        .next()
        .unwrap_or("");

    // If "env", the real interpreter is the first non-flag argument
    // Handles: #!/usr/bin/env python3, #!/usr/bin/env -S python3 -u
    let prog = if interpreter == "env" {
        line.split_whitespace()
            .skip_while(|s| !s.ends_with("env"))
            .skip(1) // skip "env" itself
            .find(|s| !s.starts_with('-'))
            .unwrap_or("")
    } else {
        interpreter
    };

    for spec in languages() {
        for shebang in spec.shebangs {
            if prog == *shebang || prog.starts_with(*shebang) {
                return Some(spec);
            }
        }
    }

    None
}

#[cfg(test)]
#[path = "language_test.rs"]
mod tests;
