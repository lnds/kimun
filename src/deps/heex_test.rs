use super::*;

#[test]
fn keeps_what_is_between_braces() {
    let code = code(r#"<div class="box">{Card.title(@item)}</div>"#);
    assert_eq!(code.trim(), "Card.title(@item)");
}

#[test]
fn keeps_nested_braces_whole() {
    let code = code("<.table rows={%{a: Row.of(%{b: 1})}} />");
    assert_eq!(code.trim(), "%{a: Row.of(%{b: 1})}");
}

#[test]
fn a_brace_in_a_string_does_not_close() {
    let code = code(r#"<p>{Label.of("}")}</p>"#);
    assert_eq!(code.trim(), r#"Label.of("}")"#);
}

#[test]
fn keeps_what_is_between_eex_tags() {
    let code = code("<%= Money.format(@total) %>\n<% end %>");
    let pieces: Vec<&str> = code.lines().map(str::trim).collect();
    assert_eq!(pieces, ["Money.format(@total)", "end"]);
}

#[test]
fn keeps_the_component_called_as_a_tag() {
    let code = code("<Card.header title=\"Total\">\n</Card.header>");
    assert_eq!(code.trim(), "Card.header");
}

#[test]
fn prose_and_markup_are_left_out() {
    let code = code("<h1>Save Changes</h1>\n<Button>Cancel Order</Button>");
    assert!(!code.contains("Save"));
    assert!(!code.contains("Cancel"));
}

#[test]
fn comments_are_left_out() {
    let code = code("<!-- {Old.call()} -->\n<%# Older.call() %>\n{New.call()}");
    assert_eq!(code.trim(), "New.call()");
}

#[test]
fn unclosed_pieces_do_not_panic() {
    code("{Card.title(");
    code("<%= Card.title(");
    code("<Card");
    code("{\"");
}

#[test]
fn a_template_is_told_by_its_extension() {
    assert!(is_template(Path::new("lib/page/index.html.heex")));
    assert!(!is_template(Path::new("lib/page/index.ex")));
}

#[test]
fn the_owner_is_beside_the_template_or_beside_its_directory() {
    assert_eq!(
        owners(Path::new("lib/page_html/home.html.heex")),
        [
            PathBuf::from("lib/page_html/home.ex"),
            PathBuf::from("lib/page_html.ex"),
        ]
    );
}
