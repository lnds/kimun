use super::*;

fn template() -> StringKind {
    StringKind::Template(vec![Frame::Text])
}

/// Bytes of `text` read inside `kind` until it closes, if it does.
fn closes_after(text: &str, mut kind: StringKind) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let (advance, closed) = scan(&bytes[i..], &mut kind);
        i += advance;
        if closed {
            return Some(i);
        }
    }
    None
}

#[test]
fn a_template_closes_at_its_backtick_past_its_interpolations() {
    assert_eq!(closes_after("a ${b} c` d", template()), Some(9));
    assert_eq!(closes_after("a ${ {x: `y`}.x } c` d", template()), Some(20));
    assert_eq!(closes_after("${ \"`\" }` d", template()), Some(9));
}

#[test]
fn an_escaped_backtick_does_not_close_a_template() {
    assert_eq!(closes_after("a \\` b", template()), None);
}

#[test]
fn a_doubled_quote_does_not_close_a_verbatim_string() {
    let verbatim = StringKind::Delimited(Delimited {
        close: b"\"".to_vec(),
        escapes: false,
        doubled: true,
        spans: true,
    });
    assert_eq!(closes_after("a \"\"b\"\" c\" d", verbatim), Some(10));
}
