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
fn mask_empty_lines() {
    let spec = detect(std::path::Path::new("test.py")).unwrap();
    let mask = multi_line_string_mask(&[], spec);
    assert!(mask.is_empty());
}
