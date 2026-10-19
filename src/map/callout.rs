//! What the page says about the map as a whole: the share of the lines on
//! each side of the scale, and the directory where each side gathers, to
//! label on the map.

use serde::Serialize;

use super::layout::Placed;
use super::metric::Words;

#[derive(Debug, PartialEq, Serialize)]
pub struct Share {
    pub label: &'static str,
    /// Of the lines with a value, from 0 to 1.
    pub share: f64,
    pub heat: f64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Callout {
    /// The directory labelled, by index into the nodes.
    pub node: usize,
    /// What gathers there, and where: `Most LLM risk: src/tui/`.
    pub label: String,
    pub heat: f64,
}

/// The lines of the files under each node whose heat `side` accepts.
fn lines_on(nodes: &[Placed], side: impl Fn(f64) -> bool) -> Vec<usize> {
    let mut lines = vec![0usize; nodes.len()];
    // Every node comes after its parent: from the last, each adds to it.
    for i in (0..nodes.len()).rev() {
        let n = &nodes[i];
        if !n.dir && n.shade.as_ref().is_some_and(|s| side(s.heat)) {
            lines[i] += n.lines;
        }
        if let Some(parent) = n.parent {
            lines[parent] += lines[i];
        }
    }
    lines
}

/// The share of the lines with a value that are risk, fine and good.
pub fn shares(nodes: &[Placed], words: &Words) -> Vec<Share> {
    let risk = lines_on(nodes, |h| h < 0.0)[0];
    let fine = lines_on(nodes, |h| h == 0.0)[0];
    let good = lines_on(nodes, |h| h > 0.0)[0];
    let total = (risk + fine + good).max(1) as f64;
    let mut shares = vec![
        Share {
            label: words.risk,
            share: risk as f64 / total,
            heat: -1.0,
        },
        Share {
            label: words.fine,
            share: fine as f64 / total,
            heat: 0.0,
        },
    ];
    if let Some(label) = words.good {
        shares.push(Share {
            label,
            share: good as f64 / total,
            heat: 1.0,
        });
    }
    shares
}

/// The directory below the root where lines of one side gather most:
/// many of them, a large share of the directory, and the directory a part
/// of the code rather than nearly all of it, which says nothing.
fn gathering(nodes: &[Placed], side: impl Fn(f64) -> bool) -> Option<usize> {
    let on_side = lines_on(nodes, side);
    let total = nodes[0].lines.max(1) as f64;
    let weight = |i: usize| {
        let lines = nodes[i].lines.max(1) as f64;
        let side = on_side[i] as f64;
        side * side / lines * (1.0 - lines / total)
    };
    (1..nodes.len())
        .filter(|&i| nodes[i].dir && on_side[i] > 0)
        .max_by(|&a, &b| weight(a).total_cmp(&weight(b)).then(b.cmp(&a)))
}

/// The labels to put on the map: where the risk gathers, and where the
/// good side does when the metric has one.
pub fn callouts(nodes: &[Placed], words: &Words) -> Vec<Callout> {
    let label = |what: &str, node: usize| format!("{what}: {}/", nodes[node].path);
    let mut found = Vec::new();
    if let Some(node) = gathering(nodes, |h| h < 0.0) {
        let label = label(words.risk_callout, node);
        found.push(Callout {
            node,
            label,
            heat: -1.0,
        });
    }
    let good = words.good_callout.zip(gathering(nodes, |h| h > 0.0));
    let apart = |(_, node): &(&str, usize)| found.first().is_none_or(|c| c.node != *node);
    if let Some((what, node)) = good.filter(apart) {
        found.push(Callout {
            node,
            label: label(what, node),
            heat: 1.0,
        });
    }
    found
}

#[cfg(test)]
#[path = "callout_test.rs"]
mod tests;
