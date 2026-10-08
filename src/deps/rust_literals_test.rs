use super::*;

fn code(source: &str) -> String {
    code_and_strings(source).0
}

#[test]
fn comments_are_blanked_and_lines_kept() {
    let source = "a // crate::x\nb /* use y; /* nested */ still */ c\n/// doc\nd";
    let out = code(source);
    assert_eq!(out.lines().count(), source.lines().count());
    assert_eq!(
        out.split_whitespace().collect::<Vec<_>>(),
        ["a", "b", "c", "d"]
    );
}

#[test]
fn a_string_keeps_its_quotes_and_hands_its_content_apart() {
    let source = r#"#[path = "x_test.rs"] let s = "a \" b"; t"#;
    let (out, strings) = code_and_strings(source);
    assert_eq!(strings, ["x_test.rs", r#"a \" b"#]);
    assert_eq!(
        out.split_whitespace().collect::<Vec<_>>(),
        [
            "#[path", "=", "\"", "\"]", "let", "s", "=", "\"", "\";", "t"
        ]
    );
    assert_eq!(out.chars().count(), source.chars().count());
}

#[test]
fn raw_and_byte_strings_are_strings() {
    let source = "a = r#\"use crate::x; \"quoted\" \"#; b = br##\"y\"##; c = b\"z\"; d = r\"w\"; e";
    let (out, strings) = code_and_strings(source);
    assert_eq!(strings, ["use crate::x; \"quoted\" ", "y", "z", "w"]);
    assert!(!out.contains("crate"));
    assert!(out.trim_end().ends_with('e'));
    // A name that ends in `r` does not open a raw string.
    assert_eq!(code_and_strings("for_r\"x\"").1, ["x"]);
    assert!(code("var\"x\"").starts_with("var\""));
}

#[test]
fn characters_are_blanked_and_lifetimes_are_code() {
    let out = code(
        "let q = '\"'; let e = '\\''; let u = '\\u{1F600}'; fn f<'a>(x: &'a str) {} 'outer: loop {}",
    );
    assert!(!out.contains('"'));
    assert!(out.contains("fn f<'a>(x: &'a str)"));
    assert!(out.contains("'outer: loop"));
}

#[test]
fn unterminated_literals_do_not_panic() {
    for source in [
        "\"open",
        "r#\"open",
        "/* open",
        "'",
        "'\\",
        "r",
        "x = '\\u{12",
    ] {
        let _ = code_and_strings(source);
    }
}
