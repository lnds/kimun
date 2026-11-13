//! How each language writes its literals and comments, beyond what
//! `LanguageSpec` declares for counting lines.

use crate::loc::language::LanguageSpec;

/// What a backtick opens.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Backticks {
    /// Nothing: code (a command in a shell).
    Code,
    /// A raw string that may span lines (Go).
    Raw,
    /// A template literal with `${…}` interpolations (JavaScript).
    Template,
}

/// How a heredoc is written, if the language has them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Heredocs {
    None,
    /// `<<EOF`, `<<-EOF`, `<<'EOF'`.
    Shell,
    /// `<<~EOS`, `<<-EOS`, `<<EOS`.
    Ruby,
    /// `<<<EOT`, `<<<"EOT"`, `<<<'EOT'`.
    Php,
    /// `<<"EOF"`, `<<'EOF'`, `<<EOF`, `<<~EOF`.
    Perl,
}

/// The literals and comments of a language.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Syntax {
    /// `'` opens a string; otherwise it opens a character literal, or is
    /// the quote of a lifetime or a label (Rust).
    pub single_quotes: bool,
    /// `"""` (and `'''` where `'` opens a string) span lines.
    pub triple_quotes: bool,
    /// A `"` string may span lines without a `\` at their end.
    pub quotes_span_lines: bool,
    pub backticks: Backticks,
    /// C#'s verbatim strings: `@"…"`, a quote doubled inside.
    pub verbatim: bool,
    /// C++'s raw strings: `R"delim(…)delim"`.
    pub cpp_raw: bool,
    /// Rust's raw strings: `r"…"`, `r#"…"#`, with a `b` or `c` prefix.
    pub rust_raw: bool,
    pub heredocs: Heredocs,
    /// Regular expression literals (JavaScript).
    pub regex: bool,
    /// `?"` and `?'` are the code of a character (Ruby, Elixir).
    pub char_codes: bool,
    /// A `#` is a comment only where a word starts (shells).
    pub hash_at_word_start: bool,
    /// `$#` is code (Perl).
    pub dollar_hash: bool,
}

impl Syntax {
    pub(crate) fn of(spec: &LanguageSpec) -> Self {
        let name = spec.name;
        let script = matches!(name, "JavaScript" | "JSX" | "TypeScript");
        let shell = matches!(name, "Bourne Shell" | "Bourne Again Shell" | "Zsh");
        Self {
            single_quotes: spec.single_quote_strings,
            triple_quotes: spec.triple_quote_strings
                || matches!(
                    name,
                    "Java" | "C#" | "Kotlin" | "Swift" | "Scala" | "Groovy" | "Gradle" | "Dart"
                ),
            quotes_span_lines: shell
                || matches!(
                    name,
                    "Rust" | "Ruby" | "PHP" | "Perl" | "Elixir" | "Elixir Script"
                ),
            backticks: match name {
                "Go" => Backticks::Raw,
                _ if script => Backticks::Template,
                _ => Backticks::Code,
            },
            verbatim: name == "C#",
            cpp_raw: name == "C++",
            rust_raw: spec.raw_strings,
            heredocs: match name {
                "Ruby" => Heredocs::Ruby,
                "PHP" => Heredocs::Php,
                "Perl" => Heredocs::Perl,
                _ if shell => Heredocs::Shell,
                _ => Heredocs::None,
            },
            regex: script,
            char_codes: matches!(name, "Ruby" | "Elixir" | "Elixir Script"),
            hash_at_word_start: shell,
            dollar_hash: name == "Perl",
        }
    }
}

impl Syntax {
    /// Whether the `#` at `at` is code: a shell's count (`$#`, `${#x}`) or
    /// a `#` inside a word, Perl's last index (`$#a`).
    pub(crate) fn hash_is_code(&self, bytes: &[u8], at: usize) -> bool {
        let before = at.checked_sub(1).map(|b| bytes[b]);
        let in_word = self.hash_at_word_start && before.is_some_and(|b| !b" \t;&|()".contains(&b));
        in_word || (self.dollar_hash && before == Some(b'$'))
    }

    /// The length of the code of a quote character at `at` (`?"`, `?\'`),
    /// if one is there.
    pub(crate) fn char_code_len(&self, bytes: &[u8], at: usize) -> Option<usize> {
        let rest = &bytes[at..];
        let before = at.checked_sub(1).map(|b| bytes[b]);
        let after_value = before.is_some_and(|b| super::is_word_byte(b) || b")]".contains(&b));
        let len = match rest {
            [b'?', b'"' | b'\'', ..] => 2,
            [b'?', b'\\', _, ..] => 3,
            _ => return None,
        };
        (self.char_codes && !after_value).then_some(len)
    }
}
