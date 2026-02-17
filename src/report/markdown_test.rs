use super::*;

#[test]
fn escape_md_no_special_chars() {
    assert_eq!(escape_md("src/main.rs"), "src/main.rs");
}

#[test]
fn escape_md_pipe() {
    assert_eq!(escape_md("foo|bar.rs"), "foo\\|bar.rs");
}

#[test]
fn escape_md_backslash_and_pipe() {
    assert_eq!(escape_md("path\\|file.rs"), "path\\\\\\|file.rs");
}

#[test]
fn top_of_truncated() {
    assert_eq!(top_of(5, 20), "top 5 of 20");
}

#[test]
fn top_of_not_truncated() {
    assert_eq!(top_of(3, 3), "3");
}

#[test]
fn top_of_zero() {
    assert_eq!(top_of(0, 0), "0");
}
