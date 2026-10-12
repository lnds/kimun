use super::*;
use crate::loc::language::detect;

#[test]
fn mask_python() {
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let lines: Vec<String> = vec![
        "x = 1",         // 0: not in string
        "y = \"\"\"",    // 1: opens triple (has code before delimiter)
        "def foo():",    // 2: interior — entirely inside triple string
        "    return 42", // 3: interior — entirely inside triple string
        "\"\"\"",        // 4: closes triple (not interior)
        "z = 2",         // 5: not in string
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let mask = multi_line_string_mask(&lines, spec);
    assert!(!mask[0]);
    assert!(!mask[1]);
    assert!(mask[2]);
    assert!(mask[3]);
    assert!(!mask[4]);
    assert!(!mask[5]);
}

#[test]
fn mask_closing_with_code() {
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let lines: Vec<String> = vec![
        "x = \"\"\"", // 0: opens triple
        "docstring",  // 1: interior
        "\"\"\" + y", // 2: closes triple, has code after
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let mask = multi_line_string_mask(&lines, spec);
    assert!(!mask[0]);
    assert!(mask[1]);
    assert!(!mask[2]);
}

#[test]
fn mask_escaped_triple_quotes() {
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let lines: Vec<String> = vec![r#"x = """has fake \"\"\" inside""""#, "y = 1 + 2"]
        .into_iter()
        .map(String::from)
        .collect();
    let mask = multi_line_string_mask(&lines, spec);
    assert!(!mask[0]);
    assert!(!mask[1]);
}

#[test]
fn mask_non_triple_language() {
    let spec = detect(std::path::Path::new("test.rs")).unwrap();
    let lines: Vec<String> = vec!["let x = 1;", "let y = 2;"]
        .into_iter()
        .map(String::from)
        .collect();
    let mask = multi_line_string_mask(&lines, spec);
    assert!(!mask[0]);
    assert!(!mask[1]);
}

#[test]
fn mask_single_quote_triple_python() {
    // Python also supports '''...''' triple-quoted strings
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let lines: Vec<String> = vec![
        "x = '''",  // 0: opens single-quote triple
        "interior", // 1: interior
        "'''",      // 2: closes
        "y = 2",    // 3: outside
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let mask = multi_line_string_mask(&lines, spec);
    assert!(!mask[0]); // opening line is not purely interior
    assert!(mask[1]); // interior line is masked
    assert!(!mask[2]); // closing line is not purely interior
    assert!(!mask[3]); // outside string
}

#[test]
fn mask_single_line_single_quote_string() {
    // A single-quoted string that opens and closes on the same line should not mask anything
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let lines: Vec<String> = vec!["x = 'hello'", "y = 2"]
        .into_iter()
        .map(String::from)
        .collect();
    let mask = multi_line_string_mask(&lines, spec);
    assert!(!mask[0]);
    assert!(!mask[1]);
}

#[test]
fn mask_advance_inside_triple_with_escape() {
    // Escaped quote inside triple string should not close the triple
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let lines: Vec<String> = vec![
        r#"x = """\"""  not end yet"""  "#, // opens, has escaped quote, closes same line
        "y = 2",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let mask = multi_line_string_mask(&lines, spec);
    assert!(!mask[0]);
    assert!(!mask[1]);
}

#[test]
fn mask_triple_opens_and_closes_same_line() {
    // A triple-quoted string that opens and closes on the same line
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let lines: Vec<String> = vec![r#"x = """hello""""#, "y = 2"]
        .into_iter()
        .map(String::from)
        .collect();
    let mask = multi_line_string_mask(&lines, spec);
    assert!(!mask[0]);
    assert!(!mask[1]);
}

#[test]
fn demote_turns_string_interior_into_blank() {
    let spec = detect(std::path::Path::new("test.kai")).unwrap();
    let lines: Vec<String> = vec![
        "fn banner() : String =",
        "  \"\"\"",
        "  if you match this",
        "  \"\"\"",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let kinds = vec![LineKind::Code; 4];
    let demoted = demote_multi_line_strings(&lines, &kinds, spec);
    assert_eq!(
        demoted,
        vec![
            LineKind::Code,
            LineKind::Code,
            LineKind::Blank,
            LineKind::Code
        ]
    );
}

#[test]
fn demote_leaves_non_triple_languages_untouched() {
    let spec = detect(std::path::Path::new("test.rs")).unwrap();
    let lines: Vec<String> = vec!["let x = 1;", "let y = 2;"]
        .into_iter()
        .map(String::from)
        .collect();
    let kinds = vec![LineKind::Code, LineKind::Comment];
    let demoted = demote_multi_line_strings(&lines, &kinds, spec);
    assert_eq!(demoted, kinds);
}

#[test]
fn mask_empty_lines() {
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let mask = multi_line_string_mask(&[], spec);
    assert!(mask.is_empty());
}

#[test]
fn starts_in_string_marks_the_closing_line() {
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let lines: Vec<String> = ["x = \"\"\"", "inside", "closing\"\"\" + y", "z = 2"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(
        starts_in_string_mask(&lines, spec),
        [false, true, true, false]
    );
}

#[test]
fn starts_in_string_without_triple_quotes() {
    let spec = detect(std::path::Path::new("test.rs")).unwrap();
    let lines = vec!["x = \"\"\"".to_string(), "inside".to_string()];
    assert_eq!(starts_in_string_mask(&lines, spec), [false, false]);
}

fn view(file: &str, src: &str) -> (Vec<String>, Vec<LineKind>) {
    let spec = detect(std::path::Path::new(file)).unwrap();
    let lines: Vec<String> = src.lines().map(String::from).collect();
    let kinds = crate::loc::counter::classify_reader(src.as_bytes(), spec);
    let (lines, kinds) = code_view(&lines, &kinds, spec);
    (lines.into_owned(), kinds)
}

#[test]
fn code_view_leaves_a_raw_string_empty() {
    let (lines, kinds) = view(
        "a.rs",
        "let a = r#\"{ \"if\": \"while {\" }\"#;\nlet b = br\"x\" + 1;\n",
    );
    assert_eq!(
        lines,
        [
            "let a = \"\"                      ;",
            "let b = b\"\"   + 1;"
        ]
    );
    assert_eq!(kinds, [LineKind::Code, LineKind::Code]);
}

#[test]
fn code_view_blanks_the_lines_inside_a_raw_string() {
    let (lines, kinds) = view("a.rs", "let a = r#\"{\n  if x {\n}\"#;\nlet b = 1;\n");
    assert_eq!(lines, ["let a = \"\"", "", "   ;", "let b = 1;"]);
    assert_eq!(
        kinds,
        [
            LineKind::Code,
            LineKind::Blank,
            LineKind::Code,
            LineKind::Code
        ]
    );
}

#[test]
fn code_view_borrows_what_holds_no_raw_string() {
    let spec = detect(std::path::Path::new("a.rs")).unwrap();
    let lines = vec!["let a = \"x\";".to_string()];
    let (same, _) = code_view(&lines, &[LineKind::Code], spec);
    assert!(matches!(same, Cow::Borrowed(_)));

    // Another language keeps its text, whatever it looks like.
    let (lines, _) = view("a.c", "x = r#\"a\"#;\n");
    assert_eq!(lines, ["x = r#\"a\"#;"]);
}

#[test]
fn code_view_still_demotes_the_interior_of_triple_quoted_strings() {
    let (_, kinds) = view("a.py", "x = \"\"\"\nif a:\n\"\"\"\n");
    assert_eq!(kinds[1], LineKind::Blank);
}
