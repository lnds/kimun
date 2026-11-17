use super::*;

#[test]
fn a_character_literal_is_as_long_as_its_closing_quote_says() {
    assert_eq!(literal_len(b"a'"), 2);
    assert_eq!(literal_len(b"\\''"), 3);
    assert_eq!(literal_len(b"\\n' rest"), 3);
    // The longest ones: four bytes of a character, eleven of an escape.
    assert_eq!(literal_len("\u{1F600}'".as_bytes()), 5);
    assert_eq!(literal_len(b"\\u{10FFFFF}'"), 12);
}

#[test]
fn a_quote_that_closes_too_far_is_no_character_literal() {
    assert_eq!(literal_len(b"abcdef"), 0);
    assert_eq!(literal_len(b"abcde'"), 0);
    assert_eq!(literal_len(b"\\u{10FFFFFF}'"), 0);
    assert_eq!(literal_len(b""), 0);
}

#[test]
fn a_lifetime_or_a_label_has_no_character_literal() {
    assert_eq!(character_len(b"a> x"), 0);
    assert_eq!(character_len(b"outer: loop"), 0);
    assert_eq!(character_len(b"a' x"), 2);
    assert_eq!(character_len(b"{' x"), 2);
}
