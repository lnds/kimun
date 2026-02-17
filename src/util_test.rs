use super::*;

#[test]
fn mask_strings_basic() {
    assert_eq!(
        mask_strings(r#"let s = "if x > 0";"#, &[]),
        r#"let s = "        ";"#
    );
    assert_eq!(
        mask_strings(r#"let c = '{'; if x {"#, &[]),
        r#"let c = ' '; if x {"#
    );
    assert_eq!(
        mask_strings(r#"let s = "he said \"hi\"";"#, &[]),
        r#"let s = "              ";"#
    );
}

#[test]
fn mask_strings_empty() {
    assert_eq!(mask_strings("", &[]), "");
}

#[test]
fn mask_strings_no_strings() {
    assert_eq!(mask_strings("let x = 42;", &[]), "let x = 42;");
}

#[test]
fn mask_strings_raw_string() {
    // Python raw string: r prefix is just an identifier char, "..." is masked normally
    let result = mask_strings(r#"x = r"if|for|while""#, &[]);
    assert!(!result.contains("if|for|while"));
    assert!(result.contains("r")); // the r prefix is preserved
}

#[test]
fn mask_strings_unclosed_string() {
    // Unclosed string: mask everything after the quote
    assert_eq!(mask_strings(r#"let s = "hello"#, &[]), r#"let s = "     "#);
}

#[test]
fn mask_strings_with_line_comment() {
    // Unmatched quote in comment should NOT confuse string masking
    assert_eq!(
        mask_strings("x = 5; // don't do this", &["//"]),
        "x = 5;                 "
    );
}

#[test]
fn mask_strings_comment_after_string() {
    // String is masked, then comment is masked
    assert_eq!(
        mask_strings(r#"x = "hello"; // it's ok"#, &["//"]),
        r#"x = "     ";           "#
    );
}

#[test]
fn mask_strings_comment_marker_inside_string() {
    // Comment marker inside a string should NOT trigger comment masking
    assert_eq!(
        mask_strings(r#"x = "http://foo"; if y"#, &["//"]),
        r#"x = "          "; if y"#
    );
}
