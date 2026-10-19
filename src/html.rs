//! Self-contained HTML pages: one file with its styles, script and data
//! inside, no external assets, light and dark themes. Each page brings its
//! own body, styles and script; the shell and the base styles are shared.

use serde::Serialize;

const BASE_STYLE: &str = include_str!("html/base.css");

/// What a page puts in the shell.
pub struct Page<'a> {
    pub title: &'a str,
    pub style: &'a str,
    pub body: &'a str,
    pub script: &'a str,
}

/// `text` safe inside HTML text and attribute values.
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `data` as JSON safe inside a `<script>` element: a `<` can only be in
/// a string, where `\u003c` stands for it, so nothing closes the element.
pub fn script_json(data: &impl Serialize) -> serde_json::Result<String> {
    Ok(serde_json::to_string(data)?.replace('<', "\\u003c"))
}

/// The whole document, with `data` readable by the script as the JSON of
/// the element `#data`.
pub fn render(page: &Page, data: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="generator" content="kimun {version}">
<title>{title}</title>
<style>
{BASE_STYLE}
{style}
</style>
</head>
<body>
{body}
<script type="application/json" id="data">{data}</script>
<script>
{script}
</script>
</body>
</html>
"#,
        version = env!("CARGO_PKG_VERSION"),
        title = escape(page.title),
        style = page.style,
        body = page.body,
        script = page.script,
    )
}

#[cfg(test)]
#[path = "html_test.rs"]
mod tests;
