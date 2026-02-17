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
        pragma: None,
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
        pragma: None,
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
        pragma: None,
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
        pragma: None,
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
        pragma: Some(("{-#", "#-}")),
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
        pragma: None,
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
        pragma: None,
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
