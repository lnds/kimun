use super::*;

fn targets(text: &str) -> Vec<(usize, String, Form)> {
    scan(text)
        .references
        .into_iter()
        .map(|r| (r.line, r.target, r.form))
        .collect()
}

fn anchors(text: &str) -> Vec<String> {
    let mut anchors: Vec<String> = scan(text).anchors.into_iter().collect();
    anchors.sort();
    anchors
}

#[test]
fn links_images_and_code_spans() {
    let text = "See [the guide](docs/guide.md#setup \"Guide\") and ![logo](img/logo.png).\n\
                Run `src/main.rs` or ``a ` b``.\n";
    assert_eq!(
        targets(text),
        vec![
            (1, "docs/guide.md#setup".to_string(), Form::Link),
            (1, "img/logo.png".to_string(), Form::Image),
            (2, "src/main.rs".to_string(), Form::Code),
            (2, "a ` b".to_string(), Form::Code),
        ]
    );
}

#[test]
fn a_code_span_inside_link_text_is_read_too() {
    assert_eq!(
        targets("[`src/kind.rs`](src/kind.rs)"),
        vec![
            (1, "src/kind.rs".to_string(), Form::Code),
            (1, "src/kind.rs".to_string(), Form::Link),
        ]
    );
}

#[test]
fn destinations_in_angle_brackets_and_with_parentheses() {
    assert_eq!(
        targets("[a](<my file.md>) [b](notes/a_(b).md)"),
        vec![
            (1, "my file.md".to_string(), Form::Link),
            (1, "notes/a_(b).md".to_string(), Form::Link),
        ]
    );
}

#[test]
fn reference_definitions_but_not_footnotes() {
    assert_eq!(
        targets("[guide]: docs/guide.md \"Guide\"\n[^1]: a footnote\n"),
        vec![(1, "docs/guide.md".to_string(), Form::Link)]
    );
}

#[test]
fn html_links_images_and_anchors() {
    let text = "<p align=\"center\"><img src=\"logo.svg\" width=\"80\"></p>\n\
                <a href='docs/x.md'>x</a> <a name=\"top\"></a>\n";
    let found = scan(text);
    let refs: Vec<(String, Form)> = found
        .references
        .into_iter()
        .map(|r| (r.target, r.form))
        .collect();
    assert_eq!(
        refs,
        vec![
            ("logo.svg".to_string(), Form::Image),
            ("docs/x.md".to_string(), Form::Link),
        ]
    );
    assert!(found.anchors.contains("top"));
}

#[test]
fn autolinks_and_comparisons_are_no_tags() {
    assert!(targets("<https://example.com> and a < b > c").is_empty());
}

#[test]
fn fenced_code_front_matter_and_comments_refer_to_nothing() {
    let text = "---\ntitle: `x/y.md`\n---\n\
                ```rust\nlet p = `src/a.rs`;\n```\n\
                ~~~~\n[a](b.md)\n~~~\n~~~~\n\
                <!-- [old](old.md)\nstill `old/x.rs` -->[new](new.md)\n";
    assert_eq!(targets(text), vec![(12, "new.md".to_string(), Form::Link)]);
}

#[test]
fn code_spans_and_link_texts_go_on_across_lines() {
    let text = "with `State[T](init) as\nState[Int](0)` done and [a\nlink](docs/x.md)\n\n`open\n\nnever closed` [b](b.md)\n";
    assert_eq!(
        targets(text),
        vec![
            (1, "State[T](init) as State[Int](0)".to_string(), Form::Code),
            (2, "docs/x.md".to_string(), Form::Link),
            (7, "b.md".to_string(), Form::Link),
        ]
    );
}

#[test]
fn indented_code_refers_to_nothing_but_list_content_does() {
    let text = "Text\n\n    Carrier[String](\"P\")\n\n    [x](x.md)\nafter [a](a.md)\n\
                - item\n\n    [b](b.md)\n";
    assert_eq!(
        targets(text),
        vec![
            (6, "a.md".to_string(), Form::Link),
            (9, "b.md".to_string(), Form::Link),
        ]
    );
}

#[test]
fn a_paragraph_goes_on_in_an_indented_line() {
    assert_eq!(
        targets("Text\n    [x](x.md)\n"),
        vec![(2, "x.md".to_string(), Form::Link)]
    );
}

#[test]
fn escaped_brackets_and_backticks_open_nothing() {
    assert!(targets(r"\[a](b.md) \`src/x.rs\`").is_empty());
}

#[test]
fn headings_give_anchors_as_github_does() {
    let text = "# Kimün: code metrics\n\
                ## `km loc` — lines ##\n\
                ### Tipos de archivo\n\
                ## Install\n## Install\n\
                Setext title\n============\n\
                ## [Linked](https://x.y) heading\n";
    assert_eq!(
        anchors(text),
        vec![
            "install",
            "install-1",
            "kimün-code-metrics",
            "km-loc--lines",
            "linked-heading",
            "setext-title",
            "tipos-de-archivo",
        ]
    );
}

#[test]
fn a_rule_after_a_list_or_blank_line_is_no_heading() {
    assert!(anchors("- item\n---\n\n---\n").is_empty());
    assert!(anchors("#hashtag\n").is_empty());
}

#[test]
fn slug_keeps_letters_digits_hyphens_and_underscores() {
    assert_eq!(slug("What's new in v0.40?"), "whats-new-in-v040");
    assert_eq!(
        slug("snake_case and kebab-case"),
        "snake_case-and-kebab-case"
    );
}
