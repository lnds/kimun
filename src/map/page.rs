//! The map as a self-contained HTML page: the circles come laid out, the
//! page draws them as SVG, zooms into a directory on a click and tells a
//! file's path, lines and value on hover.

use serde::Serialize;

use super::callout::{Callout, Share};
use super::layout::Placed;
use super::metric::Key;
use crate::html;

const STYLE: &str = include_str!("map.css");
const SCRIPT: &str = concat!(
    include_str!("map_callouts.js"),
    include_str!("map_patterns.js"),
    include_str!("map.js")
);

/// What every map builds on.
const CREDITS: &str = "The enclosure diagram follows Adam Tornhill, <i>Your Code as a Crime \
    Scene</i> (Pragmatic Bookshelf, 2015; second edition, 2024). The circles are packed as \
    d3.pack by Mike Bostock packs them: the front chain of Wang et al. (2006) and the smallest \
    enclosing circle of Welzl (1991).";

/// What the readiness for coding agents builds on.
const LLM_CREDITS: &str = " The thresholds of readiness for coding agents follow CodeScene's \
    research on Code Health and AI-assisted coding, mapped to the scale of km score. kimun is \
    not affiliated with Adam Tornhill or CodeScene.";

#[derive(Serialize)]
pub struct Data<'a> {
    /// Whether the map shows the readiness for coding agents.
    #[serde(skip)]
    pub llm: bool,
    /// What colours the files, as the legend titles it.
    pub metric: &'a str,
    pub legend: &'a [Key],
    /// The share of the lines on each side of the scale.
    pub summary: &'a [Share],
    pub callouts: &'a [Callout],
    pub nodes: &'a [Placed],
}

/// The page of the map of the directory `name`.
pub fn render(name: &str, data: &Data<'_>) -> serde_json::Result<String> {
    let title = format!("Code map · {name}");
    let body = format!(
        r#"<header class="page">
<h1>{title}</h1>
<p>Each circle is a directory holding its files, a file's area grows with its lines of code. Coloured by {metric}. Click a directory to zoom in, outside it to zoom out.</p>
<p id="summary"></p>
</header>
<div class="toolbar"><button id="up">Up</button><span id="crumb" class="mono"></span><span id="legend" class="legend"></span><label><input type="checkbox" id="patterns"> Patterns</label></div>
<main><svg id="map" viewBox="0 0 1000 1000" role="img" aria-label="{title}"></svg></main>
<div id="tip" hidden></div>"#,
        title = html::escape(&title),
        metric = html::escape(data.metric),
    );
    let credits = match data.llm {
        true => format!("{CREDITS}{LLM_CREDITS}"),
        false => format!("{CREDITS} kimun is not affiliated with Adam Tornhill."),
    };
    html::page(&title, &body, (STYLE, SCRIPT), &credits, data)
}
