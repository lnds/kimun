/// Language specification registry and detection.
///
/// Defines 40+ programming languages via the `lang!` macro, each with
/// file extension/filename mappings, comment syntax (line, block, nested),
/// string delimiter rules, pragma support, and shebang patterns.
use std::path::Path;

use super::lang_macro::lang_spec;

/// Specification of a programming language's syntax for line classification.
///
/// Each field controls how the FSM in `counter.rs` identifies comments,
/// strings, and pragmas. Languages are detected by extension, filename,
/// or shebang line.
#[derive(Debug)]
pub struct LanguageSpec {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub filenames: &'static [&'static str],
    pub line_comments: &'static [&'static str],
    /// Characters that, if immediately following a line comment marker,
    /// prevent it from being treated as a comment. Used for Haskell where
    /// `-->` is an operator but `-- comment` is a comment.
    pub line_comment_not_before: &'static str,
    pub block_comment: Option<(&'static str, &'static str)>,
    pub nested_block_comments: bool,
    pub single_quote_strings: bool,
    pub triple_quote_strings: bool,
    pub pragma: Option<(&'static str, &'static str)>,
    pub shebangs: &'static [&'static str],
}

/// Return the static registry of all supported language specifications.
pub fn languages() -> &'static [LanguageSpec] {
    static LANGUAGES: &[LanguageSpec] = &[
        lang_spec!("Rust", ext: ["rs"],
              line: "//", block: "/*", "*/", nested: true),
        lang_spec!("Python", ext: ["py", "pyi"],
              line: "#", sq: true, tq: true,
              shebangs: ["python", "python3"]),
        lang_spec!("JavaScript", ext: ["js", "mjs", "cjs"],
              line: "//", block: "/*", "*/", sq: true,
              shebangs: ["node"]),
        lang_spec!("TypeScript", ext: ["ts", "mts", "cts"],
              line: "//", block: "/*", "*/", sq: true),
        lang_spec!("Java", ext: ["java"],
              line: "//", block: "/*", "*/"),
        lang_spec!("C", ext: ["c", "h"],
              line: "//", block: "/*", "*/"),
        lang_spec!("C++", ext: ["cpp", "cxx", "cc", "hpp", "hxx"],
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
            filenames: &[],
            line_comments: &["--"],
            line_comment_not_before: "!#$%&*+./<=>?@\\^|~",
            block_comment: Some(("{-", "-}")),
            nested_block_comments: true,
            single_quote_strings: false,
            triple_quote_strings: false,
            pragma: Some(("{-#", "#-}")),
            shebangs: &[],
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
        lang_spec!("Elixir", ext: ["ex"],
              line: "#", sq: true,
              shebangs: ["elixir"]),
        lang_spec!("Elixir Script", ext: ["exs"],
              line: "#", sq: true),
        lang_spec!("Clojure", ext: ["clj", "cljs", "cljc", "edn"],
              line: ";"),
        lang_spec!("Zig", ext: ["zig"],
              line: "//"),
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
        lang_spec!("DOS Batch", ext: ["bat", "cmd"],
              lines: ["::", "rem ", "REM ", "Rem "]),
        lang_spec!("Properties", ext: ["properties"],
              line: "#"),
        lang_spec!("Text", ext: ["txt"],
              none),
    ];
    LANGUAGES
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
