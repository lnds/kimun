use super::*;

fn words(source: &str) -> Vec<&str> {
    tokens(source)
        .into_iter()
        .filter_map(|t| match t {
            Token::Word(w) => Some(w),
            Token::Punct(_) => None,
        })
        .collect()
}

#[test]
fn comments_and_literals_are_left_out() {
    let src = "a // b c\n/* d\n e */ f \"g \\\" h\" 'i' '\\'' j\n";
    assert_eq!(words(src), ["a", "f", "j"]);
}

#[test]
fn text_blocks_and_raw_strings_close_with_as_many_quotes() {
    let src = "a \"\"\"\nb \" c\n\"\"\" d \"\"\"\"x \"\"\" y\"\"\"\" e\n";
    assert_eq!(words(src), ["a", "d", "e"]);
}

#[test]
fn verbatim_and_interpolated_strings_are_literals() {
    let src = "a @\"b \"\" c\" d $\"e {f}\" g $@\"h\" i\n";
    assert_eq!(words(src), ["a", "d", "g", "i"]);
}

#[test]
fn punctuation_is_kept() {
    assert_eq!(
        tokens("a.b;"),
        [
            Token::Word("a"),
            Token::Punct(b'.'),
            Token::Word("b"),
            Token::Punct(b';')
        ]
    );
}
