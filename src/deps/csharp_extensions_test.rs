use super::*;
use crate::deps::c_like::tokens;

/// The word before each `(` of `source` that `called_at` finds, with the
/// number of arguments it passes.
fn called(source: &str) -> Vec<(String, usize)> {
    let tokens = tokens(source);
    (0..tokens.len())
        .filter(|&i| tokens[i] == Token::Punct(b'('))
        .filter_map(|i| Some((called_at(&tokens, i)?, i)))
        .map(|(name, open)| match tokens[name] {
            Token::Word(w) => (w.to_string(), call_arguments(&tokens, open).unwrap_or(99)),
            Token::Punct(p) => ((p as char).to_string(), 0),
        })
        .collect()
}

#[test]
fn a_call_is_named_by_the_word_before_its_type_arguments() {
    let calls = called("x.Cast<List<int>>(); a.Run(f(1, 2), x => x > 0); (b)");
    let expected = [("Cast", 0), ("Run", 2), ("f", 2)];
    let expected: Vec<(String, usize)> =
        expected.iter().map(|(n, c)| (n.to_string(), *c)).collect();
    assert_eq!(calls, expected);
    assert_eq!(called("if (a > (b)) {}"), [("if".to_string(), 99)]);
    assert_eq!(
        called("int IList.IndexOf(T x) => 0;"),
        [("IndexOf".to_string(), 99)]
    );
}

fn arity(source: &str) -> Option<Range<usize>> {
    let tokens = tokens(source);
    let open = tokens
        .iter()
        .position(|t| *t == Token::Punct(b'('))
        .unwrap();
    extension_arity(&tokens, open, open - 1)
}

#[test]
fn an_extension_takes_this_before_a_type_and_is_not_private() {
    assert_eq!(
        arity("public static bool IsValueType(this Type t)"),
        Some(0..1)
    );
    assert_eq!(
        arity("[A] internal static int Size(this in Span s)"),
        Some(0..1)
    );
    assert_eq!(arity("private static T Next(this IEnumerator<T> e)"), None);
    assert_eq!(arity("void Attach(this)"), None);
    assert_eq!(arity("void Attach(Type t)"), None);
}

#[test]
fn optional_and_variadic_parameters_widen_the_arity() {
    assert_eq!(
        arity("static bool Has<K, V>(this IDictionary<K, V> d, K key, int n = 0)"),
        Some(1..3)
    );
    assert_eq!(
        arity("static string FormatWith(this string f, IFormatProvider p, params object[] args)"),
        Some(1..usize::MAX)
    );
}

#[test]
fn a_call_reaches_the_declarations_in_namespaces_it_sees_by_arity() {
    let declared = |namespace: &str, arity: Range<usize>| Extension {
        namespace: namespace.to_string(),
        method: "IsValueType".to_string(),
        arity,
    };
    let declarations = [
        (declared("Lib.Utilities", 0..1), 0),
        (declared("Lib.Utilities", 1..2), 0),
        (declared("Other", 0..1), 1),
        (declared("Lib.Utilities", 2..3), 2),
    ];
    let mut members: Vec<Members> = (0..3).map(|_| Members::default()).collect();
    for (extension, file) in declarations {
        members[file].declared.push(extension);
    }
    let extensions = Extensions::of(members.iter().map(Some));
    let seen = ["".to_string(), "Lib.Utilities".to_string()];
    let call = |count| ("IsValueType".to_string(), count);
    assert_eq!(extensions.candidates(&call(0), &seen), [0]);
    assert_eq!(extensions.candidates(&call(1), &seen), [0]);
    assert_eq!(extensions.candidates(&call(2), &seen), [2]);
    assert!(extensions.candidates(&call(3), &seen).is_empty());
}
