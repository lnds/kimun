use super::*;
use std::io::Cursor;

fn spec_c_like() -> LanguageSpec {
    LanguageSpec {
        name: "C",
        extensions: &["c"],
        filenames: &[],
        line_comments: &["//"],
        line_comment_not_before: "",
        block_comment: Some(("/*", "*/")),
        nested_block_comments: false,
        single_quote_strings: false,
        triple_quote_strings: false,
        triple_quote_comments: false,
        raw_strings: false,
        pragma: None,
        doc_attributes: &[],
        shebangs: &[],
    }
}

fn spec_rust() -> LanguageSpec {
    LanguageSpec {
        name: "Rust",
        extensions: &["rs"],
        filenames: &[],
        line_comments: &["//"],
        line_comment_not_before: "",
        block_comment: Some(("/*", "*/")),
        nested_block_comments: true,
        single_quote_strings: false,
        triple_quote_strings: false,
        triple_quote_comments: false,
        raw_strings: true,
        pragma: None,
        doc_attributes: &[],
        shebangs: &[],
    }
}

fn spec_python() -> LanguageSpec {
    LanguageSpec {
        name: "Python",
        extensions: &["py"],
        filenames: &[],
        line_comments: &["#"],
        line_comment_not_before: "",
        block_comment: None,
        nested_block_comments: false,
        single_quote_strings: true,
        triple_quote_strings: true,
        triple_quote_comments: false,
        raw_strings: false,
        pragma: None,
        doc_attributes: &[],
        shebangs: &["python", "python3"],
    }
}

fn spec_js() -> LanguageSpec {
    LanguageSpec {
        name: "JavaScript",
        extensions: &["js"],
        filenames: &[],
        line_comments: &["//"],
        line_comment_not_before: "",
        block_comment: Some(("/*", "*/")),
        nested_block_comments: false,
        single_quote_strings: true,
        triple_quote_strings: false,
        triple_quote_comments: false,
        raw_strings: false,
        pragma: None,
        doc_attributes: &[],
        shebangs: &["node"],
    }
}

fn spec_haskell() -> LanguageSpec {
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
        triple_quote_comments: false,
        raw_strings: false,
        pragma: Some(("{-#", "#-}")),
        doc_attributes: &[],
        shebangs: &[],
    }
}

fn spec_kaikai() -> LanguageSpec {
    LanguageSpec {
        name: "Kaikai",
        extensions: &["kai"],
        filenames: &[],
        line_comments: &["#"],
        line_comment_not_before: "[",
        block_comment: None,
        nested_block_comments: false,
        single_quote_strings: false,
        triple_quote_strings: true,
        triple_quote_comments: false,
        raw_strings: false,
        pragma: None,
        doc_attributes: &[("#[doc(", ")]")],
        shebangs: &[],
    }
}

fn count(spec: &LanguageSpec, text: &str) -> FileStats {
    count_reader(Cursor::new(text.as_bytes()), spec)
}

// --- Basic classification ---

#[test]
fn blank_lines() {
    let stats = count(&spec_c_like(), "  \n\n  \n");
    assert_eq!(stats.blank, 3);
    assert_eq!(stats.code, 0);
    assert_eq!(stats.comment, 0);
}

#[test]
fn code_only() {
    let stats = count(&spec_c_like(), "int x = 1;\nreturn x;\n");
    assert_eq!(stats.code, 2);
    assert_eq!(stats.comment, 0);
    assert_eq!(stats.blank, 0);
}

#[test]
fn line_comment_only() {
    let stats = count(&spec_c_like(), "// this is a comment\n");
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 0);
}

#[test]
fn code_with_trailing_comment() {
    let stats = count(&spec_c_like(), "int x = 1; // init x\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

// --- Strings ---

#[test]
fn comment_marker_inside_double_string() {
    let stats = count(&spec_c_like(), "char *s = \"// not a comment\";\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn comment_marker_inside_single_string() {
    let stats = count(&spec_js(), "var s = '// not a comment';\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn escaped_quote_in_string() {
    let stats = count(&spec_c_like(), "char *s = \"he said \\\"hello\\\"\";\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn trailing_backslash_in_string() {
    // Line ends with a backslash inside a string — should not panic
    let stats = count(&spec_c_like(), "char *s = \"test\\\n");
    assert_eq!(stats.code, 1);
}

#[test]
fn block_comment_inside_string() {
    let stats = count(&spec_c_like(), "char *s = \"/* not a comment */\";\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

// --- Block comments ---

#[test]
fn single_line_block_comment() {
    let stats = count(&spec_c_like(), "/* comment */\n");
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 0);
}

#[test]
fn multi_line_block_comment() {
    let stats = count(&spec_c_like(), "/*\n * line 1\n * line 2\n */\n");
    assert_eq!(stats.comment, 4);
    assert_eq!(stats.code, 0);
}

#[test]
fn code_before_block_comment() {
    let stats = count(&spec_c_like(), "int x = 1; /* comment */\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn blank_line_inside_block_comment() {
    let stats = count(&spec_c_like(), "/*\n\n */\n");
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.blank, 0);
}

// --- Nested block comments ---

#[test]
fn nested_block_comments_rust() {
    let stats = count(&spec_rust(), "/* outer /* inner */ still comment */\n");
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 0);
}

#[test]
fn nested_block_comments_multiline() {
    let stats = count(
        &spec_rust(),
        "/* outer\n/* inner */\nstill comment\n*/\ncode();\n",
    );
    assert_eq!(stats.comment, 4);
    assert_eq!(stats.code, 1);
}

#[test]
fn non_nested_block_comments_c() {
    // In C, block comments don't nest — first */ closes the comment
    let stats = count(&spec_c_like(), "/* outer /* inner */ code_here;\n");
    assert_eq!(stats.code, 1); // "code_here;" is code after comment closes
}

// --- Python triple-quote strings ---

#[test]
fn python_triple_double_quote() {
    let stats = count(&spec_python(), "s = \"\"\"hello\nworld\"\"\"\n");
    assert_eq!(stats.code, 2);
}

#[test]
fn python_triple_single_quote() {
    let stats = count(&spec_python(), "s = '''hello\nworld'''\n");
    assert_eq!(stats.code, 2);
}

#[test]
fn python_comment_inside_triple_string() {
    let stats = count(&spec_python(), "s = \"\"\"# not a comment\"\"\"\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

// --- Kaikai ---

#[test]
fn kaikai_hash_line_comment() {
    let stats = count(
        &spec_kaikai(),
        "# a comment\nfn add(a: Int, b: Int) : Int = a + b\n",
    );
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 1);
}

#[test]
fn kaikai_interpolation_hash_is_code() {
    // `#{...}` interpolation lives inside a string, not a comment
    let stats = count(&spec_kaikai(), "let s = \"hi #{name}\"\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn kaikai_char_literal_is_code() {
    // single quotes denote Char, not strings — must not open a string
    let stats = count(&spec_kaikai(), "let c = 'A'\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn kaikai_triple_quote_string() {
    let stats = count(&spec_kaikai(), "let m = \"\"\"\nfirst\nsecond\n\"\"\"\n");
    assert_eq!(stats.code, 4);
    assert_eq!(stats.comment, 0);
}

#[test]
fn kaikai_hash_inside_triple_quote_not_comment() {
    let stats = count(&spec_kaikai(), "let m = \"\"\"# not a comment\"\"\"\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn kaikai_attribute_is_code() {
    // `#[` opens an attribute, so it must not be read as a `#` comment
    let stats = count(&spec_kaikai(), "#[constructor]\nfn make() : Int = 1\n");
    assert_eq!(stats.code, 2);
    assert_eq!(stats.comment, 0);
}

#[test]
fn kaikai_single_line_doc_attribute_is_comment() {
    let stats = count(
        &spec_kaikai(),
        "#[doc(\"Adds two integers.\")]\nfn add(a: Int, b: Int) : Int = a + b\n",
    );
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 1);
}

#[test]
fn kaikai_multi_line_doc_attribute_is_comment() {
    let stats = count(
        &spec_kaikai(),
        "#[doc(\"\"\"\nSummary line.\nfn not_really_code() : Int = 42\n\"\"\")]\nfn f() : Int = 1\n",
    );
    assert_eq!(stats.comment, 4);
    assert_eq!(stats.code, 1);
}

#[test]
fn kaikai_doc_attribute_body_can_contain_closing_delimiter() {
    // `)]` inside the doc text belongs to the string, it does not close the attribute
    let stats = count(
        &spec_kaikai(),
        "#[doc(\"\"\"\nsee list(xs)] for details\n\"\"\")]\nfn f() : Int = 1\n",
    );
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.code, 1);
}

#[test]
fn kaikai_doc_attribute_body_hash_is_comment() {
    let stats = count(&spec_kaikai(), "#[doc(\"count #{n} items\")]\n");
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 0);
}

// --- Elixir ---

fn spec_elixir() -> &'static LanguageSpec {
    super::super::language::detect(std::path::Path::new("lib/a.ex")).unwrap()
}

#[test]
fn elixir_doc_heredocs_are_comment() {
    let text = "defmodule A do\n  @moduledoc \"\"\"\n  If it fails, it retries.\n\n  ## Examples\n  \"\"\"\n\n  @doc ~S\"\"\"\n  Runs it.\n  \"\"\"\n  def run, do: :ok\nend\n";
    let stats = count(spec_elixir(), text);
    assert_eq!(stats.comment, 8);
    assert_eq!(stats.code, 3);
}

#[test]
fn elixir_single_line_doc_is_code() {
    // cloc counts it as code too: only heredocs are documentation
    let stats = count(spec_elixir(), "@doc \"Runs it.\"\ndef run, do: :ok\n");
    assert_eq!(stats.code, 2);
    assert_eq!(stats.comment, 0);
}

#[test]
fn elixir_other_heredocs_are_code() {
    let stats = count(
        spec_elixir(),
        "@query \"\"\"\n# not a comment\nSELECT 1\n\"\"\"\n",
    );
    assert_eq!(stats.code, 4);
    assert_eq!(stats.comment, 0);
}

#[test]
fn elixir_script_doc_heredocs_are_comment() {
    let spec = super::super::language::detect(std::path::Path::new("test/a_test.exs")).unwrap();
    let stats = count(spec, "@doc \"\"\"\nHelper.\n\"\"\"\ndef h, do: 1\n");
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.code, 1);
}

// --- Haskell pragmas ---

#[test]
fn haskell_pragma_is_code() {
    let stats = count(&spec_haskell(), "{-# LANGUAGE OverloadedStrings #-}\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn haskell_arrow_not_comment() {
    // --> is an operator in Haskell, not a comment
    let stats = count(&spec_haskell(), "x = y --> z\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn haskell_dash_dash_space_is_comment() {
    let stats = count(&spec_haskell(), "-- this is a comment\n");
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 0);
}

#[test]
fn haskell_triple_dash_is_comment() {
    // --- is still a comment (- is not in the exception list because
    // consecutive dashes are comments by convention)
    let stats = count(&spec_haskell(), "--- section\n");
    assert_eq!(stats.comment, 1);
}

#[test]
fn haskell_block_comment_still_works() {
    let stats = count(&spec_haskell(), "{- this is a comment -}\n");
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 0);
}

#[test]
fn haskell_pragma_with_code() {
    let stats = count(
        &spec_haskell(),
        "{-# LANGUAGE OverloadedStrings #-}\nmodule Main where\n",
    );
    assert_eq!(stats.code, 2);
    assert_eq!(stats.comment, 0);
}

// --- Rust lifetimes should NOT trigger string mode ---

#[test]
fn rust_lifetime_not_string() {
    let stats = count(&spec_rust(), "fn foo<'a>(x: &'a str) -> &'a str {\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn rust_static_lifetime() {
    let stats = count(
        &spec_rust(),
        "static S: &'static str = \"hello\"; // comment\n",
    );
    assert_eq!(stats.code, 1);
}

// --- Edge cases ---

#[test]
fn empty_file() {
    let stats = count(&spec_c_like(), "");
    assert_eq!(stats.blank, 0);
    assert_eq!(stats.comment, 0);
    assert_eq!(stats.code, 0);
}

#[test]
fn only_whitespace_file() {
    let stats = count(&spec_c_like(), "   \n  \n\n");
    assert_eq!(stats.blank, 3);
    assert_eq!(stats.code, 0);
}

#[test]
fn comment_only_file() {
    let stats = count(&spec_c_like(), "// line 1\n// line 2\n// line 3\n");
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.code, 0);
}

#[test]
fn line_comment_at_start() {
    let stats = count(&spec_c_like(), "  // indented comment\n");
    assert_eq!(stats.comment, 1);
}

#[test]
fn mixed_blank_code_comment() {
    let text = "\nint x = 1;\n// comment\n\nint y = 2; // trailing\n";
    let stats = count(&spec_c_like(), text);
    assert_eq!(stats.blank, 2);
    assert_eq!(stats.code, 2);
    assert_eq!(stats.comment, 1);
}

#[test]
fn python_single_quote_with_hash() {
    let stats = count(&spec_python(), "s = '# not a comment'\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn shebang_is_code() {
    let stats = count(
        &spec_python(),
        "#!/usr/bin/env python3\n# comment\nprint('hi')\n",
    );
    assert_eq!(stats.code, 2);
    assert_eq!(stats.comment, 1);
}

#[test]
fn string_at_end_of_line_resets() {
    // Unterminated string on one line should not affect the next
    let stats = count(&spec_c_like(), "char *s = \"unterminated\nint x = 1;\n");
    assert_eq!(stats.code, 2);
}

// --- Additional coverage tests ---

#[test]
fn python_triple_double_close_mid_line() {
    // Close triple-double-quote mid-line, then code continues
    let stats = count(&spec_python(), "s = \"\"\"text\"\"\"; x = 1\n");
    assert_eq!(stats.code, 1);
}

#[test]
fn python_triple_single_close_mid_line() {
    let stats = count(&spec_python(), "s = '''text'''; x = 1\n");
    assert_eq!(stats.code, 1);
}

#[test]
fn single_quote_escape() {
    let stats = count(&spec_js(), "var s = 'it\\'s';\n");
    assert_eq!(stats.code, 1);
}

#[test]
fn single_quote_close() {
    let stats = count(&spec_js(), "var s = 'hello';\n");
    assert_eq!(stats.code, 1);
}

#[test]
fn binary_detection() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(b"hello\x00world").unwrap();
    tmp.flush().unwrap();

    let result = count_lines(tmp.path(), &spec_c_like()).unwrap();
    assert!(result.is_none(), "binary files should return None");
}

#[test]
fn count_lines_regular_file() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(b"int x = 1;\n// comment\n\n").unwrap();
    tmp.flush().unwrap();

    let stats = count_lines(tmp.path(), &spec_c_like()).unwrap().unwrap();
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.blank, 1);
}

fn spec_no_comments() -> LanguageSpec {
    LanguageSpec {
        name: "JSON",
        extensions: &["json"],
        filenames: &[],
        line_comments: &[],
        line_comment_not_before: "",
        block_comment: None,
        nested_block_comments: false,
        single_quote_strings: false,
        triple_quote_strings: false,
        triple_quote_comments: false,
        raw_strings: false,
        pragma: None,
        doc_attributes: &[],
        shebangs: &[],
    }
}

#[test]
fn no_comment_language() {
    let stats = count(&spec_no_comments(), "{\"key\": \"value\"}\n");
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

fn spec_batch() -> LanguageSpec {
    LanguageSpec {
        name: "DOS Batch",
        extensions: &["bat"],
        filenames: &[],
        line_comments: &["::", "rem ", "REM ", "Rem "],
        line_comment_not_before: "",
        block_comment: None,
        nested_block_comments: false,
        single_quote_strings: false,
        triple_quote_strings: false,
        triple_quote_comments: false,
        raw_strings: false,
        pragma: None,
        doc_attributes: &[],
        shebangs: &[],
    }
}

#[test]
fn batch_multiple_comment_markers() {
    let stats = count(
        &spec_batch(),
        ":: comment\nrem comment\nREM comment\necho hello\n",
    );
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.code, 1);
}

fn spec_heex() -> LanguageSpec {
    LanguageSpec {
        name: "HTML EEx",
        extensions: &["heex"],
        filenames: &[],
        line_comments: &[],
        line_comment_not_before: "",
        block_comment: Some(("<!--", "-->")),
        nested_block_comments: false,
        single_quote_strings: true,
        triple_quote_strings: false,
        triple_quote_comments: false,
        raw_strings: false,
        pragma: None,
        doc_attributes: &[],
        shebangs: &[],
    }
}

fn spec_po() -> LanguageSpec {
    LanguageSpec {
        name: "PO File",
        extensions: &["po", "pot"],
        filenames: &[],
        line_comments: &["#"],
        line_comment_not_before: "",
        block_comment: None,
        nested_block_comments: false,
        single_quote_strings: false,
        triple_quote_strings: false,
        triple_quote_comments: false,
        raw_strings: false,
        pragma: None,
        doc_attributes: &[],
        shebangs: &[],
    }
}

#[test]
fn heex_html_comment() {
    let stats = count(&spec_heex(), "<!-- comment -->\n<div>hello</div>\n");
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 1);
}

#[test]
fn heex_multiline_comment() {
    let stats = count(&spec_heex(), "<!--\n  multi\n  line\n-->\n<p>code</p>\n");
    assert_eq!(stats.comment, 4);
    assert_eq!(stats.code, 1);
}

#[test]
fn heex_code_only() {
    let stats = count(
        &spec_heex(),
        "<.live_component module={MyApp.Modal} id=\"modal\" />\n",
    );
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn po_comment_lines() {
    let stats = count(
        &spec_po(),
        "# translator comment\n#. extracted\n#: src/foo.ex:42\n#, fuzzy\nmsgid \"hello\"\nmsgstr \"hola\"\n",
    );
    assert_eq!(stats.comment, 4);
    assert_eq!(stats.code, 2);
}

#[test]
fn po_blank_and_code() {
    let stats = count(
        &spec_po(),
        "msgid \"\"\nmsgstr \"\"\n\nmsgid \"Save\"\nmsgstr \"Guardar\"\n",
    );
    assert_eq!(stats.blank, 1);
    assert_eq!(stats.code, 4);
    assert_eq!(stats.comment, 0);
}

// --- Raw strings ---

/// Code and comment lines of the Rust source made of `lines`.
fn rust_lines(lines: &[&str]) -> (usize, usize) {
    lines_of(&spec_rust(), lines)
}

fn lines_of(spec: &LanguageSpec, lines: &[&str]) -> (usize, usize) {
    let stats = count(spec, &lines.join("\n"));
    (stats.code, stats.comment)
}

#[test]
fn a_quote_inside_a_raw_string_does_not_end_it() {
    let counted = rust_lines(&[
        "fn main() {",
        "    let manifest = r#\"{ \"workspaces\": [\"packages/*\"] }\"#;",
        "    println!(\"{manifest}\");",
        "    let n = 1;",
        "}",
    ]);
    assert_eq!(counted, (5, 0));
}

#[test]
fn a_raw_string_ends_at_a_quote_with_as_many_hashes_as_opened_it() {
    let counted = rust_lines(&[
        "let a = r##\"one \"# /* still text\"##; // closed",
        "let b = r\"C:\\\"; /* comment */",
        "let c = 1;",
    ]);
    assert_eq!(counted, (3, 0));
    // The comment after the literal is one.
    let counted = rust_lines(&["let a = r#\"x\"#; /*", "still a comment", "*/"]);
    assert_eq!(counted, (1, 2));
}

#[test]
fn a_raw_string_spans_lines() {
    let counted = rust_lines(&[
        "let text = r#\"",
        "// not a comment",
        "/* nor this",
        "\"#;",
        "let n = 1; // one",
    ]);
    assert_eq!(counted, (5, 0));
}

#[test]
fn byte_and_c_raw_strings_are_raw_strings() {
    for prefix in ["br", "cr"] {
        let first = format!("let a = {prefix}#\"a \"/*\" b\"#;");
        assert_eq!(rust_lines(&[&first, "let n = 1;"]), (2, 0), "{prefix}");
    }
}

#[test]
fn an_r_that_starts_no_raw_string() {
    // A raw identifier, and names that end in `r`.
    let counted = rust_lines(&[
        "let r#type = 1; /* one */ let s = \"a\"; /*",
        "comment */",
        "let x = car\"/*\"; /*",
        "comment */",
        "let y = abr#\"/*\"; /*",
        "comment */",
    ]);
    assert_eq!(counted, (3, 3));
}

#[test]
fn raw_strings_belong_to_the_languages_that_have_them() {
    let lines = ["x = r#\"a \"/*\" b\"#;", "y = 1;"];
    assert_eq!(rust_lines(&lines), (2, 0));
    assert_eq!(lines_of(&spec_c_like(), &lines), (1, 1));
}

#[test]
fn raw_string_spans_tell_where_each_one_is() {
    let lines: Vec<String> = ["let a = r#\"x\"# + r\"y\";", "let b = br#\"", "  }", "\"#;"]
        .iter()
        .map(|line| line.to_string())
        .collect();
    let span = |start, end, opens| RawSpan { start, end, opens };
    assert_eq!(
        raw_string_spans(&lines, &spec_rust()),
        [
            vec![span(8, 14, true), span(17, 21, true)],
            vec![span(9, 12, true)],
            vec![span(0, 3, false)],
            vec![span(0, 2, false)],
        ]
    );
}

// --- Triple-quoted strings as comment (Python, as cloc counts them) ---

/// Blank, comment and code lines of `text`, written in `language`.
fn counted(language: &str, text: &str) -> (usize, usize, usize) {
    let spec = crate::loc::language::languages()
        .iter()
        .find(|s| s.name == language)
        .unwrap();
    let stats = count(spec, text);
    (stats.blank, stats.comment, stats.code)
}

#[test]
fn python_docstrings_are_comment() {
    let src = "\
\"\"\"Module docstring.

Explains the module.
\"\"\"


def add(a, b):
    \"\"\"Add two numbers.

    :param a: first
    \"\"\"
    return a + b


class Box:
    \'\'\'A box.\'\'\'
";
    assert_eq!(counted("Python", src), (6, 7, 3));
}

#[test]
fn python_triple_quoted_string_after_code() {
    // The line that opens the string holds code; the rest is comment.
    assert_eq!(
        counted("Python", "SQL = \"\"\"\nselect *\nfrom t\n\"\"\"\n"),
        (0, 3, 1)
    );
    // Code after the closing quotes makes the line code.
    assert_eq!(
        counted("Python", "\"\"\"a\"\"\" + \"\"\"b\"\"\"\n"),
        (0, 0, 1)
    );
}

#[test]
fn python_apostrophe_inside_a_docstring() {
    let src = "def f():\n    \"\"\"Don't\n    stop.\"\"\"\n    return 1\n";
    assert_eq!(counted("Python", src), (0, 2, 2));
}

#[test]
fn yacc_lex_cmake_jq_and_eex_comments() {
    assert_eq!(
        counted("yacc", "%token A\n/* c */\n// c\nrule: A ;\n"),
        (0, 2, 2)
    );
    assert_eq!(
        counted("lex", "%%\n/* c */\n[0-9]+ { return 1; }\n"),
        (0, 1, 2)
    );
    assert_eq!(counted("CMake", "# c\nproject(x)\n"), (0, 1, 1));
    assert_eq!(counted("jq", "# c\ndef f: .;\n"), (0, 1, 1));
    assert_eq!(counted("EEx", "<%# c %>\n<%= @x %>\n"), (0, 1, 1));
    assert_eq!(
        counted("HTML EEx", "<%!-- c\n--%>\n<% # c %>\n<div>{@x}</div>\n"),
        (0, 3, 1)
    );
    assert_eq!(
        counted("reStructuredText", ".. note:: x\nText.\n"),
        (0, 1, 1)
    );
}
