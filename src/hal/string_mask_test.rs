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
