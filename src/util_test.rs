use super::*;

#[test]
fn mask_strings_basic() {
    assert_eq!(
        mask_strings(r#"let s = "if x > 0";"#),
        r#"let s = "        ";"#
    );
    assert_eq!(
        mask_strings(r#"let c = '{'; if x {"#),
        r#"let c = ' '; if x {"#
    );
    assert_eq!(
        mask_strings(r#"let s = "he said \"hi\"";"#),
        r#"let s = "              ";"#
    );
}

#[test]
fn mask_strings_empty() {
    assert_eq!(mask_strings(""), "");
}

#[test]
fn mask_strings_no_strings() {
    assert_eq!(mask_strings("let x = 42;"), "let x = 42;");
}

#[test]
fn mask_strings_raw_string() {
    // Python raw string: r prefix is just an identifier char, "..." is masked normally
    let result = mask_strings(r#"x = r"if|for|while""#);
    assert!(!result.contains("if|for|while"));
    assert!(result.contains("r")); // the r prefix is preserved
}

#[test]
fn mask_strings_unclosed_string() {
    // Unclosed string: mask everything after the quote
    assert_eq!(mask_strings(r#"let s = "hello"#), r#"let s = "     "#);
}
