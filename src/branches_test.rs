use super::*;

#[test]
fn a_spaced_question_mark_is_a_ternary_but_a_wildcard_is_not() {
    assert_eq!(count_ternaries("x = a > 0 ? 1 : b ? 2 : 3;"), 2);
    assert_eq!(
        count_ternaries("List<? extends T> a, Map<K, ? super V> b"),
        0
    );
    assert_eq!(count_ternaries("a?.b ?? c"), 0);
}

#[test]
fn a_method_a_symbol_or_a_key_names_something() {
    let named = |line: &str, word: &str, keys| {
        let at = line.find(word).unwrap();
        names_something(line.as_bytes(), at, word.len(), keys)
    };
    assert!(named("p.catch(f)", "catch", false));
    assert!(!named("a..if", "if", false));
    assert!(named("opts[:if]", "if", true));
    assert!(named("f(if: true)", "if", true));
    assert!(!named("f(if: true)", "if", false));
    assert!(!named("Foo::if", "if", true));
}
