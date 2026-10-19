//! The dependency matrix as a self-contained HTML page. The tree comes
//! ordered and the edges by file, so the page can expand and collapse
//! directories and count the cells again; it opens as the text would show.

use serde::Serialize;

use super::matrix::{Line, Matrix};
use crate::html;

const STYLE: &str = include_str!("matrix_html.css");
const SCRIPT: &str = include_str!("matrix_html.js");
const CREDITS: &str = "The dependency structure matrix comes from Donald V. Steward, \
    \"The Design Structure System\" (1981); its layout here follows the one of IntelliJ IDEA. \
    kimun is not affiliated with JetBrains.";

#[derive(Serialize)]
struct Node<'a> {
    name: &'a str,
    path: String,
    children: &'a [usize],
    file: Option<usize>,
}

#[derive(Serialize)]
struct Data<'a> {
    nodes: Vec<Node<'a>>,
    files: Vec<String>,
    /// User, used, whether it loads or compiles, whether it is in a cycle.
    edges: Vec<(usize, usize, bool, bool)>,
    /// The directories shown expanded when the page opens.
    expanded: Vec<usize>,
}

fn data(m: &Matrix) -> Data<'_> {
    let nodes = m.tree.nodes.iter().map(|n| Node {
        name: &n.name,
        path: n.path.display().to_string(),
        children: &n.children,
        file: n.file,
    });
    let edges = m.edges.iter().zip(&m.uses).zip(&m.cyclic);
    let expanded = m.lines.iter().filter_map(|line| match line {
        Line::Group { node, .. } => Some(*node),
        Line::Row { .. } => None,
    });
    Data {
        nodes: nodes.collect(),
        files: m.files.iter().map(|f| f.display().to_string()).collect(),
        edges: edges
            .map(|((e, &(from, to)), &cyclic)| {
                (from, to, e.kind <= super::graph::EdgeKind::Load, cyclic)
            })
            .collect(),
        expanded: expanded.collect(),
    }
}

/// The page of `m`, titled after `name`, the directory analysed.
pub fn render(m: &Matrix, name: &str) -> serde_json::Result<String> {
    let title = format!("Dependency matrix · {name}");
    let body = format!(
        r#"<header class="page">
<h1>{title}</h1>
<p id="summary"></p>
</header>
<div class="toolbar">
<button id="expand">Expand all</button>
<button id="collapse">Collapse all</button>
<span class="legend"><i class="swatch cycle"></i>in a cycle <i class="swatch layering"></i>against the layering <i class="swatch weak"></i>type-only or deferred <i class="swatch uses"></i>uses <i class="swatch used"></i>used by</span>
</div>
<main>
<div id="scroll"><table id="matrix"></table></div>
<aside id="details"><p class="hint">A cell counts the files of its column that use files of its row. Click a name to see what it uses and what uses it, a cell to see the uses behind it.</p></aside>
</main>"#,
        title = html::escape(&title)
    );
    html::page(&title, &body, (STYLE, SCRIPT), CREDITS, &data(m))
}

#[cfg(test)]
#[path = "matrix_html_test.rs"]
mod tests;
