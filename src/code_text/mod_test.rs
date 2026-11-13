use super::*;
use crate::loc::language::languages;

fn spec(name: &str) -> &'static LanguageSpec {
    languages().iter().find(|s| s.name == name).unwrap()
}

/// The code of each line of `src`, without trailing spaces.
fn code(language: &str, src: &str) -> Vec<String> {
    let lines: Vec<String> = src.lines().map(String::from).collect();
    read(&lines, spec(language))
        .into_iter()
        .map(|line| line.text.trim_end().to_string())
        .collect()
}

/// Whether code is left on each line of `src`.
fn has_code(language: &str, src: &str) -> Vec<bool> {
    let lines: Vec<String> = src.lines().map(String::from).collect();
    read(&lines, spec(language))
        .into_iter()
        .map(|line| line.code)
        .collect()
}

#[test]
fn a_rust_label_or_lifetime_keeps_the_rest_of_its_line() {
    assert_eq!(
        code(
            "Rust",
            "'rows: for row in grid { if a && b { break 'rows; } }"
        ),
        [" rows: for row in grid { if a && b { break  rows; } }"]
    );
    assert_eq!(
        code("Rust", "fn pick<'a>(x: &'a str) { if n > 3 && m {} }"),
        ["fn pick< a>(x: & a str) { if n > 3 && m {} }"]
    );
}

#[test]
fn a_rust_character_literal_is_blanked_within_its_quotes() {
    assert_eq!(
        code("Rust", "if c == '{' || c == '\\'' { x }"),
        ["if c == ' ' || c == '  ' { x }"]
    );
}

#[test]
fn rust_raw_strings_are_blanked_on_every_line_they_cover() {
    let src = "let a = r#\"x\"# + r\"y\";\nlet b = br#\"\n  }\n\"#;";
    assert_eq!(
        code("Rust", src),
        ["let a = \"   \"  + \"  \";", "let b = \"\"", "", "  ;"]
    );
    assert_eq!(has_code("Rust", src), [true, true, false, true]);
}

#[test]
fn a_rust_string_spans_lines() {
    let src = "let m = \"shown if out of range, \\\n         or while waiting\";\nif x {}";
    assert_eq!(
        code("Rust", src),
        ["let m = \"\"", "                          ;", "if x {}"]
    );
}

#[test]
fn nested_rust_comments_close_with_the_outer_one() {
    assert_eq!(
        code("Rust", "a /* b /* c */ if */ d"),
        ["a                    d"]
    );
}

#[test]
fn a_block_comment_after_code_is_blanked() {
    assert_eq!(
        code(
            "C",
            "int y = x; /* if (y) case */\nreturn y; /* else {\n } */ z;"
        ),
        ["int y = x;", "return y;", "      z;"]
    );
}

#[test]
fn a_comment_marker_inside_a_string_opens_no_comment() {
    assert_eq!(
        code("C", "s = \"/* if */\"; if (x) {}"),
        ["s = \"        \"; if (x) {}"]
    );
}

#[test]
fn a_javascript_template_literal_is_a_string() {
    assert_eq!(code("JavaScript", "const s = `{`;"), ["const s = \" \";"]);
    assert_eq!(
        code(
            "JavaScript",
            "fetch(`https://api.example.com/${id}`).then(x)"
        ),
        ["fetch(\"                        \" id\"\").then(x)"]
    );
}

#[test]
fn the_code_of_a_template_interpolation_is_kept() {
    assert_eq!(
        code("TypeScript", "x = `a ${b ? { c: 1 }.c : 2} d`;"),
        ["x = \"  \" b ? { c: 1 }.c : 2\"  \";"]
    );
}

#[test]
fn a_template_literal_spans_lines() {
    let src = "const q = `\n  select * /* if */\n  from t`;\nif (x) {}";
    assert_eq!(
        code("JavaScript", src),
        ["const q = \"", "", "         ;", "if (x) {}"]
    );
    assert_eq!(has_code("JavaScript", src), [true, false, true, true]);
}

#[test]
fn a_javascript_regex_literal_is_blanked() {
    assert_eq!(
        code("JavaScript", "if (/if|for|while {/.test(s)) {}"),
        ["if (/              /.test(s)) {}"]
    );
    assert_eq!(code("JavaScript", "a = b / c / d;"), ["a = b / c / d;"]);
}

#[test]
fn a_go_raw_string_spans_lines() {
    let src = "s := `{\"a\": [\n  if x`\nif y {}";
    assert_eq!(code("Go", src), ["s := \"\"", "", "if y {}"]);
    assert_eq!(has_code("Go", src), [true, false, true]);
}

#[test]
fn a_go_rune_is_a_character_literal() {
    assert_eq!(code("Go", "if r == '\"' { x }"), ["if r == ' ' { x }"]);
}

#[test]
fn shell_comments_start_words() {
    assert_eq!(
        code(
            "Bourne Shell",
            "if [ $# -gt 0 ] && [ ${#x} -gt 1 ]; then # if"
        ),
        ["if [ $# -gt 0 ] && [ ${#x} -gt 1 ]; then"]
    );
}

#[test]
fn a_shell_heredoc_is_text_up_to_its_terminator() {
    let src = "cat <<-EOF > f\n  if while\n  EOF\nif x; then y; fi";
    assert_eq!(
        code("Bourne Shell", src),
        ["cat \"\"     > f", "", "", "if x; then y; fi"]
    );
}

#[test]
fn a_php_hash_comment_is_a_comment_and_an_attribute_is_not() {
    assert_eq!(code("PHP", "$a = 1; # if for"), ["$a = 1;"]);
    assert_eq!(code("PHP", "#[Route('/x')]"), ["#[Route('  ')]"]);
}

#[test]
fn a_php_heredoc_ends_at_its_terminator() {
    let src = "$s = <<<SQL\n  select if\n  SQL;\nif ($x) {}";
    assert_eq!(code("PHP", src), ["$s = \"\"", "", "     ;", "if ($x) {}"]);
}

#[test]
fn a_ruby_heredoc_ends_at_its_terminator() {
    let src = "q = <<~SQL\n  where a or b\nSQL\nclass << self";
    assert_eq!(code("Ruby", src), ["q = \"\"", "", "", "class << self"]);
}

#[test]
fn perl_keeps_the_last_index_of_an_array() {
    assert_eq!(code("Perl", "for (0..$#a) { } # if"), ["for (0..$#a) { }"]);
}

#[test]
fn a_character_code_holds_no_quote() {
    assert_eq!(code("Elixir", "c == ?\" or c == ?'"), ["c == ?  or c == ?"]);
    assert_eq!(code("Ruby", "x ? \"a\" : b"), ["x ? \" \" : b"]);
}

#[test]
fn java_text_blocks_csharp_verbatim_and_cpp_raw_strings_span_lines() {
    let java = "String q = \"\"\"\n  if (a) { }\n  \"\"\";\nif (b) {}";
    assert_eq!(
        code("Java", java),
        ["String q = \"\"", "", "     ;", "if (b) {}"]
    );
    let csharp = "var q = @\"\n  if \"\"x\"\" while\n\";\nif (b) {}";
    assert_eq!(code("C#", csharp), ["var q = \"\"", "", " ;", "if (b) {}"]);
    let cpp = "auto q = R\"sql(\n  if ( \"x\" )\n)sql\";\nif (b) {}";
    assert_eq!(
        code("C++", cpp),
        ["auto q = \"\"", "", "     ;", "if (b) {}"]
    );
}

#[test]
fn kotlin_and_swift_raw_strings_span_lines() {
    let src = "val q = \"\"\"\n  if (a) when\n\"\"\"\nif (b) {}";
    assert_eq!(code("Kotlin", src), ["val q = \"\"", "", "", "if (b) {}"]);
    assert_eq!(code("Swift", src), ["val q = \"\"", "", "", "if (b) {}"]);
}

#[test]
fn a_python_string_ends_with_its_line() {
    assert_eq!(code("Python", "s = \"open\nif x:"), ["s = \"\"", "if x:"]);
}
