use super::*;

#[test]
fn escape_leaves_no_markup() {
    assert_eq!(
        escape(r#"<a href="x">&</a>"#),
        "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
    );
}

#[test]
fn script_json_cannot_close_its_element() {
    let json = script_json(&vec!["</script><!--"]).unwrap();
    assert!(!json.contains('<'));
    let back: Vec<String> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, vec!["</script><!--"]);
}

#[test]
fn render_puts_everything_in_one_document() {
    let page = Page {
        title: "A & B",
        style: ".x{}",
        body: "<main></main>",
        script: "run();",
    };
    let html = render(&page, "{}");
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<title>A &amp; B</title>"));
    assert!(html.contains(r#"<script type="application/json" id="data">{}</script>"#));
    assert!(html.contains(".x{}") && html.contains("run();") && html.contains("--bg"));
    assert!(!html.contains("http"));
}
