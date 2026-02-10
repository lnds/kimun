use std::path::Path;

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

macro_rules! lang {
    ($name:expr, ext: [$($ext:expr),*], files: [$($f:expr),*], $($rest:tt)*) => {
        lang!(@build $name, &[$($ext),*], &[$($f),*], $($rest)*)
    };
    ($name:expr, ext: [$($ext:expr),*], $($rest:tt)*) => {
        lang!(@build $name, &[$($ext),*], &[], $($rest)*)
    };
    // line + block + pragma
    (@build $name:expr, $ext:expr, $files:expr,
     line: $lc:expr, block: $bo:expr, $bc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     , pragma: $po:expr, $pc:expr
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[$lc],
            line_comment_not_before: "",
            block_comment: Some(($bo, $bc)),
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: Some(($po, $pc)),
            shebangs: &[$($($sh),*)?],
        }
    };
    // line + block (no pragma)
    (@build $name:expr, $ext:expr, $files:expr,
     line: $lc:expr, block: $bo:expr, $bc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[$lc],
            line_comment_not_before: "",
            block_comment: Some(($bo, $bc)),
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
    // line comment only
    (@build $name:expr, $ext:expr, $files:expr,
     line: $lc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[$lc],
            line_comment_not_before: "",
            block_comment: None,
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
    // block + pragma (no line comment)
    (@build $name:expr, $ext:expr, $files:expr,
     block: $bo:expr, $bc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     , pragma: $po:expr, $pc:expr
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[],
            line_comment_not_before: "",
            block_comment: Some(($bo, $bc)),
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: Some(($po, $pc)),
            shebangs: &[$($($sh),*)?],
        }
    };
    // block only (no pragma, no line comment)
    (@build $name:expr, $ext:expr, $files:expr,
     block: $bo:expr, $bc:expr
     $(, nested: $nested:expr)?
     $(, sq: $sq:expr)?
     $(, tq: $tq:expr)?
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[],
            line_comment_not_before: "",
            block_comment: Some(($bo, $bc)),
            nested_block_comments: false $(|| $nested)?,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false $(|| $tq)?,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
    // multiple line comment markers (e.g. DOS Batch: :: and rem)
    (@build $name:expr, $ext:expr, $files:expr,
     lines: [$($lc:expr),+]
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[$($lc),+],
            line_comment_not_before: "",
            block_comment: None,
            nested_block_comments: false,
            single_quote_strings: false,
            triple_quote_strings: false,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
    // no comments
    (@build $name:expr, $ext:expr, $files:expr,
     none
     $(, sq: $sq:expr)?
     $(, shebangs: [$($sh:expr),*])?
    ) => {
        LanguageSpec {
            name: $name,
            extensions: $ext,
            filenames: $files,
            line_comments: &[],
            line_comment_not_before: "",
            block_comment: None,
            nested_block_comments: false,
            single_quote_strings: false $(|| $sq)?,
            triple_quote_strings: false,
            pragma: None,
            shebangs: &[$($($sh),*)?],
        }
    };
}

pub fn languages() -> &'static [LanguageSpec] {
    static LANGUAGES: &[LanguageSpec] = &[
        lang!("Rust", ext: ["rs"],
              line: "//", block: "/*", "*/", nested: true),
        lang!("Python", ext: ["py", "pyi"],
              line: "#", sq: true, tq: true,
              shebangs: ["python", "python3"]),
        lang!("JavaScript", ext: ["js", "mjs", "cjs"],
              line: "//", block: "/*", "*/", sq: true,
              shebangs: ["node"]),
        lang!("TypeScript", ext: ["ts", "mts", "cts"],
              line: "//", block: "/*", "*/", sq: true),
        lang!("Java", ext: ["java"],
              line: "//", block: "/*", "*/"),
        lang!("C", ext: ["c", "h"],
              line: "//", block: "/*", "*/"),
        lang!("C++", ext: ["cpp", "cxx", "cc", "hpp", "hxx"],
              line: "//", block: "/*", "*/"),
        lang!("C#", ext: ["cs"],
              line: "//", block: "/*", "*/"),
        lang!("Go", ext: ["go"],
              line: "//", block: "/*", "*/"),
        lang!("Ruby", ext: ["rb"], files: ["Rakefile", "Gemfile"],
              line: "#", sq: true,
              shebangs: ["ruby"]),
        lang!("Bourne Shell", ext: ["sh"],
              line: "#", sq: true,
              shebangs: ["sh"]),
        lang!("Bourne Again Shell", ext: ["bash"],
              line: "#", sq: true,
              shebangs: ["bash"]),
        lang!("Zsh", ext: ["zsh"],
              line: "#", sq: true,
              shebangs: ["zsh"]),
        lang!("HTML", ext: ["html", "htm"],
              block: "<!--", "-->", sq: true),
        lang!("CSS", ext: ["css"],
              block: "/*", "*/", sq: true),
        lang!("SQL", ext: ["sql"],
              line: "--", block: "/*", "*/", sq: true),
        lang!("TOML", ext: ["toml"],
              line: "#"),
        lang!("YAML", ext: ["yaml", "yml"],
              line: "#"),
        lang!("JSON", ext: ["json"],
              none),
        lang!("Markdown", ext: ["md", "markdown"],
              none),
        lang!("Kotlin", ext: ["kt", "kts"],
              line: "//", block: "/*", "*/", nested: true),
        lang!("Swift", ext: ["swift"],
              line: "//", block: "/*", "*/", nested: true),
        lang!("PHP", ext: ["php"],
              line: "//", block: "/*", "*/", sq: true),
        lang!("Dart", ext: ["dart"],
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
        lang!("Lua", ext: ["lua"],
              line: "--", block: "--[[", "]]", sq: true,
              shebangs: ["lua"]),
        lang!("Perl", ext: ["pl", "pm"],
              line: "#", sq: true,
              shebangs: ["perl"]),
        lang!("R", ext: ["r", "R"],
              line: "#", sq: true,
              shebangs: ["Rscript"]),
        lang!("Scala", ext: ["scala", "sc", "sbt"],
              line: "//", block: "/*", "*/", nested: true),
        lang!("XML", ext: ["xml", "xsl", "xslt", "svg", "fsproj", "csproj", "vbproj", "vcxproj", "sln", "plist", "xaml"],
              block: "<!--", "-->", sq: true),
        lang!("Dockerfile", ext: [], files: ["Dockerfile"],
              line: "#"),
        lang!("Makefile", ext: ["mk"], files: ["Makefile", "makefile", "GNUmakefile"],
              line: "#"),
        lang!("Elixir", ext: ["ex"],
              line: "#", sq: true,
              shebangs: ["elixir"]),
        lang!("Elixir Script", ext: ["exs"],
              line: "#", sq: true),
        lang!("Clojure", ext: ["clj", "cljs", "cljc", "edn"],
              line: ";"),
        lang!("Zig", ext: ["zig"],
              line: "//"),
        lang!("Objective-C", ext: ["m", "mm"],
              line: "//", block: "/*", "*/"),
        lang!("OCaml", ext: ["ml", "mli"],
              block: "(*", "*)", nested: true),
        lang!("F#", ext: ["fs", "fsi", "fsx"],
              line: "//", block: "(*", "*)", nested: true),
        lang!("Nim", ext: ["nim"],
              line: "#", block: "#[", "]#", nested: true),
        lang!("Julia", ext: ["jl"],
              line: "#", block: "#=", "=#", nested: true,
              shebangs: ["julia"]),
        lang!("Terraform", ext: ["tf"],
              line: "#", block: "/*", "*/"),
        lang!("Groovy", ext: ["groovy"],
              line: "//", block: "/*", "*/", sq: true),
        lang!("Gradle", ext: ["gradle"],
              line: "//", block: "/*", "*/", sq: true),
        lang!("Erlang", ext: ["erl", "hrl"],
              line: "%"),
        lang!("DOS Batch", ext: ["bat", "cmd"],
              lines: ["::", "rem ", "REM ", "Rem "]),
        lang!("Properties", ext: ["properties"],
              line: "#"),
        lang!("Text", ext: ["txt"],
              none),
    ];
    LANGUAGES
}

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
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detect_by_extension_rs() {
        let spec = detect(Path::new("main.rs")).unwrap();
        assert_eq!(spec.name, "Rust");
    }

    #[test]
    fn detect_by_extension_py() {
        let spec = detect(Path::new("script.py")).unwrap();
        assert_eq!(spec.name, "Python");
    }

    #[test]
    fn detect_by_filename_makefile() {
        let spec = detect(Path::new("Makefile")).unwrap();
        assert_eq!(spec.name, "Makefile");
    }

    #[test]
    fn detect_by_filename_dockerfile() {
        let spec = detect(Path::new("Dockerfile")).unwrap();
        assert_eq!(spec.name, "Dockerfile");
    }

    #[test]
    fn detect_unknown_extension() {
        assert!(detect(Path::new("file.xyz123")).is_none());
    }

    #[test]
    fn detect_no_extension() {
        // A file with no extension and no matching filename
        assert!(detect(Path::new("randomfile")).is_none());
    }

    #[test]
    fn shebang_python() {
        let spec = detect_by_shebang("#!/usr/bin/python3\n").unwrap();
        assert_eq!(spec.name, "Python");
    }

    #[test]
    fn shebang_env_python() {
        let spec = detect_by_shebang("#!/usr/bin/env python3\n").unwrap();
        assert_eq!(spec.name, "Python");
    }

    #[test]
    fn shebang_env_with_flags() {
        let spec = detect_by_shebang("#!/usr/bin/env -S python3 -u\n").unwrap();
        assert_eq!(spec.name, "Python");
    }

    #[test]
    fn shebang_bash() {
        let spec = detect_by_shebang("#!/bin/bash\n").unwrap();
        assert_eq!(spec.name, "Bourne Again Shell");
    }

    #[test]
    fn shebang_node() {
        let spec = detect_by_shebang("#!/usr/bin/env node\n").unwrap();
        assert_eq!(spec.name, "JavaScript");
    }

    #[test]
    fn shebang_not_a_shebang() {
        assert!(detect_by_shebang("print('hello')\n").is_none());
    }

    #[test]
    fn shebang_unknown_interpreter() {
        assert!(detect_by_shebang("#!/usr/bin/unknownlang\n").is_none());
    }

    #[test]
    fn languages_not_empty() {
        assert!(!languages().is_empty());
    }

    #[test]
    fn all_languages_have_names() {
        for spec in languages() {
            assert!(!spec.name.is_empty());
        }
    }
}
