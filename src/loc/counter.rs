use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use super::language::LanguageSpec;

#[derive(Debug, Default, Clone)]
pub struct FileStats {
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Blank,
    Comment,
    Code,
}

#[derive(Debug, PartialEq)]
enum StringKind {
    Double,
    Single,
    TripleDouble,
    TripleSingle,
}

#[derive(Debug, PartialEq)]
enum State {
    Normal,
    InString(StringKind),
    InBlockComment(usize), // nesting depth
}

fn is_binary(data: &[u8]) -> bool {
    data.contains(&0)
}

fn bytes_start_with(haystack: &[u8], needle: &str) -> bool {
    haystack.starts_with(needle.as_bytes())
}

pub fn count_lines(path: &Path, spec: &LanguageSpec) -> io::Result<Option<FileStats>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Binary detection on first 512 bytes
    let mut header = [0u8; 512];
    let n = reader.read(&mut header)?;
    if is_binary(&header[..n]) {
        return Ok(None);
    }
    reader.seek(SeekFrom::Start(0))?;

    Ok(Some(count_reader(reader, spec)))
}

/// Classify each line of source code as Blank, Comment, or Code.
pub fn classify_reader<R: BufRead>(reader: R, spec: &LanguageSpec) -> Vec<LineKind> {
    process_lines(reader, spec)
}

pub fn count_reader<R: BufRead>(reader: R, spec: &LanguageSpec) -> FileStats {
    let mut stats = FileStats::default();
    for kind in process_lines(reader, spec) {
        match kind {
            LineKind::Blank => stats.blank += 1,
            LineKind::Comment => stats.comment += 1,
            LineKind::Code => stats.code += 1,
        }
    }
    stats
}

struct StepResult {
    advance: usize,
    new_state: Option<State>,
    has_code: bool,
    has_comment: bool,
    break_line: bool,
}

fn skip_pragma(bytes: &[u8], start: usize, pclose: &str) -> usize {
    let len = bytes.len();
    let mut i = start;
    while i < len {
        if bytes_start_with(&bytes[i..], pclose) {
            return i + pclose.len();
        }
        i += 1;
    }
    i
}

fn step_normal(rest: &[u8], spec: &LanguageSpec, full_bytes: &[u8], pos: usize) -> StepResult {
    // Triple-quote strings (before regular quotes)
    if spec.triple_quote_strings {
        if rest.len() >= 3 && &rest[..3] == b"\"\"\"" {
            return StepResult {
                advance: 3,
                new_state: Some(State::InString(StringKind::TripleDouble)),
                has_code: true,
                has_comment: false,
                break_line: false,
            };
        }
        if spec.single_quote_strings && rest.len() >= 3 && &rest[..3] == b"'''" {
            return StepResult {
                advance: 3,
                new_state: Some(State::InString(StringKind::TripleSingle)),
                has_code: true,
                has_comment: false,
                break_line: false,
            };
        }
    }

    // Pragma (e.g. Haskell {-# ... #-}) — must check before block comment
    if let Some((popen, pclose)) = spec.pragma
        && bytes_start_with(rest, popen)
    {
        let end = skip_pragma(full_bytes, pos + popen.len(), pclose);
        return StepResult {
            advance: end - pos,
            new_state: None,
            has_code: true,
            has_comment: false,
            break_line: false,
        };
    }

    // Block comment open
    if let Some((open, _)) = spec.block_comment
        && bytes_start_with(rest, open)
    {
        return StepResult {
            advance: open.len(),
            new_state: Some(State::InBlockComment(1)),
            has_code: false,
            has_comment: true,
            break_line: false,
        };
    }

    // Line comment
    let is_line_comment = spec.line_comments.iter().any(|lc| {
        if !bytes_start_with(rest, lc) {
            return false;
        }
        if !spec.line_comment_not_before.is_empty()
            && let Some(&next_byte) = rest.get(lc.len())
            && spec.line_comment_not_before.as_bytes().contains(&next_byte)
        {
            return false;
        }
        true
    });
    if is_line_comment {
        return StepResult {
            advance: 0,
            new_state: None,
            has_code: false,
            has_comment: true,
            break_line: true,
        };
    }

    let ch = rest[0];

    // Double-quote string
    if ch == b'"' {
        return StepResult {
            advance: 1,
            new_state: Some(State::InString(StringKind::Double)),
            has_code: true,
            has_comment: false,
            break_line: false,
        };
    }

    // Single-quote string
    if spec.single_quote_strings && ch == b'\'' {
        return StepResult {
            advance: 1,
            new_state: Some(State::InString(StringKind::Single)),
            has_code: true,
            has_comment: false,
            break_line: false,
        };
    }

    StepResult {
        advance: 1,
        new_state: None,
        has_code: !ch.is_ascii_whitespace(),
        has_comment: false,
        break_line: false,
    }
}

fn step_in_string(rest: &[u8], ch: u8, kind: &StringKind, len: usize, pos: usize) -> StepResult {
    match kind {
        StringKind::TripleDouble => {
            if rest.len() >= 3 && &rest[..3] == b"\"\"\"" {
                return StepResult {
                    advance: 3,
                    new_state: Some(State::Normal),
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
        }
        StringKind::TripleSingle => {
            if rest.len() >= 3 && &rest[..3] == b"'''" {
                return StepResult {
                    advance: 3,
                    new_state: Some(State::Normal),
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
        }
        StringKind::Double => {
            if ch == b'\\' {
                return StepResult {
                    advance: (pos + 2).min(len) - pos,
                    new_state: None,
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
            if ch == b'"' {
                return StepResult {
                    advance: 1,
                    new_state: Some(State::Normal),
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
        }
        StringKind::Single => {
            if ch == b'\\' {
                return StepResult {
                    advance: (pos + 2).min(len) - pos,
                    new_state: None,
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
            if ch == b'\'' {
                return StepResult {
                    advance: 1,
                    new_state: Some(State::Normal),
                    has_code: true,
                    has_comment: false,
                    break_line: false,
                };
            }
        }
    }
    StepResult {
        advance: 1,
        new_state: None,
        has_code: true,
        has_comment: false,
        break_line: false,
    }
}

fn step_in_block_comment(rest: &[u8], spec: &LanguageSpec, depth: usize) -> StepResult {
    // Check for nested open (before checking close)
    if spec.nested_block_comments
        && let Some((open, _)) = spec.block_comment
        && bytes_start_with(rest, open)
    {
        return StepResult {
            advance: open.len(),
            new_state: Some(State::InBlockComment(depth + 1)),
            has_code: false,
            has_comment: true,
            break_line: false,
        };
    }

    if let Some((_, close)) = spec.block_comment
        && bytes_start_with(rest, close)
    {
        let new_state = if depth <= 1 {
            State::Normal
        } else {
            State::InBlockComment(depth - 1)
        };
        return StepResult {
            advance: close.len(),
            new_state: Some(new_state),
            has_code: false,
            has_comment: true,
            break_line: false,
        };
    }

    StepResult {
        advance: 1,
        new_state: None,
        has_code: false,
        has_comment: true,
        break_line: false,
    }
}

fn process_lines<R: BufRead>(reader: R, spec: &LanguageSpec) -> Vec<LineKind> {
    let mut kinds = Vec::new();
    let mut state = State::Normal;
    let mut is_first_line = true;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Shebang line is code, not a comment
        if is_first_line {
            is_first_line = false;
            if line.starts_with("#!") {
                kinds.push(LineKind::Code);
                continue;
            }
        }

        if line.trim().is_empty() && !matches!(state, State::InBlockComment(_)) {
            kinds.push(LineKind::Blank);
            continue;
        }

        let mut has_code = false;
        let mut has_comment = false;
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        if matches!(state, State::InBlockComment(_)) {
            has_comment = true;
        }
        if matches!(state, State::InString(_)) {
            has_code = true;
        }

        while i < len {
            let result = match &state {
                State::Normal => step_normal(&bytes[i..], spec, bytes, i),
                State::InString(kind) => step_in_string(&bytes[i..], bytes[i], kind, len, i),
                State::InBlockComment(depth) => step_in_block_comment(&bytes[i..], spec, *depth),
            };
            if result.has_code {
                has_code = true;
            }
            if result.has_comment {
                has_comment = true;
            }
            if let Some(new_state) = result.new_state {
                state = new_state;
            }
            i += result.advance;
            if result.break_line {
                break;
            }
        }

        // Reset InString at end of line for non-triple-quote strings
        // (most languages don't allow multi-line strings with plain quotes)
        if matches!(
            state,
            State::InString(StringKind::Double | StringKind::Single)
        ) {
            state = State::Normal;
        }

        if has_code {
            kinds.push(LineKind::Code);
        } else if has_comment {
            kinds.push(LineKind::Comment);
        } else {
            kinds.push(LineKind::Blank);
        }
    }

    kinds
}

#[cfg(test)]
mod tests {
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
}
