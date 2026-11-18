use super::*;

/// The literal `%w(` opens, with the brackets nesting in it.
fn word_list() -> Delimited {
    match percent(b"%w(", b"!|/^").unwrap() {
        (Literal::Delimited(d), 3) => d,
        other => panic!("not a percent literal: {other:?}"),
    }
}

/// Where in `text` the literal closes, walking it a byte at a time.
fn closes_at(mut d: Delimited, text: &[u8]) -> Option<usize> {
    (0..text.len()).find(|&i| d.step_closes(&text[i..]))
}

#[test]
fn a_nested_bracket_does_not_close_a_literal_that_nests() {
    assert_eq!(closes_at(word_list(), b"a (b) c) d"), Some(7));
    assert_eq!(closes_at(word_list(), b"a ((b)) c)"), Some(9));
    assert_eq!(closes_at(word_list(), b"a (b c"), None);
}

#[test]
fn a_literal_that_does_not_nest_closes_at_its_close() {
    let (Literal::Delimited(mut d), 3) = percent(b"%r!", b"!|/^").unwrap() else {
        panic!("not a percent literal");
    };
    assert_eq!(d.nests, None);
    assert!(!d.step_closes(b"("));
    assert!(d.step_closes(b"!"));
    let mut d = word_list();
    d.nests = None;
    assert_eq!(closes_at(d, b"a (b) c)"), Some(4));
}

#[test]
fn a_quote_after_a_dollar_or_an_odd_backslash_opens_nothing() {
    let (ruby, shell) = (Marks::of("Ruby", false), Marks::of("Bourne Shell", true));
    assert!(quote_opens_nothing(b"after = $'", 9, ruby));
    assert!(quote_opens_nothing(b"x=it\\'s", 5, shell));
    assert!(quote_opens_nothing(b"x=a\\\\\\'b", 6, shell));
}

#[test]
fn a_quote_after_an_even_backslash_or_none_opens_a_string() {
    let (ruby, shell) = (Marks::of("Ruby", false), Marks::of("Bourne Shell", true));
    assert!(!quote_opens_nothing(b"x=a\\\\'b'", 5, shell));
    assert!(!quote_opens_nothing(b"x = 'a'", 4, shell));
    assert!(!quote_opens_nothing(b"x = 'a'", 4, ruby));
    // Only a shell escapes outside a string; only Ruby and Perl name `$'`.
    assert!(!quote_opens_nothing(b"x\\'", 2, ruby));
    assert!(!quote_opens_nothing(b"x=$'a'", 4, shell));
}
