use std::path::PathBuf;

use super::*;
use crate::map::layout::layout;
use crate::map::metric::{ColorBy, Shade, words};

fn shade(heat: f64) -> Option<Shade> {
    Some(Shade {
        heat,
        label: String::new(),
    })
}

/// A risky directory, a friendly one, and the rest in between.
fn nodes() -> Vec<Placed> {
    let files = vec![
        (PathBuf::from("bad/a.rs"), 300, shade(-1.0)),
        (PathBuf::from("bad/b.rs"), 100, shade(-1.0)),
        (PathBuf::from("good/c.rs"), 200, shade(1.0)),
        (PathBuf::from("good/d.rs"), 50, shade(0.0)),
        (PathBuf::from("mid/e.rs"), 300, shade(0.0)),
        (PathBuf::from("mid/f.rs"), 50, None),
    ];
    layout(&files)
}

fn name(nodes: &[Placed], i: usize) -> &str {
    &nodes[i].name
}

#[test]
fn shares_count_lines_with_a_value_on_each_side() {
    let nodes = nodes();
    let shares = shares(&nodes, &words(ColorBy::Llm));
    let of = |label: &str| shares.iter().find(|s| s.label == label).unwrap().share;
    assert!((of("an LLM risk") - 400.0 / 950.0).abs() < 1e-9);
    assert!((of("needing care") - 350.0 / 950.0).abs() < 1e-9);
    assert!((of("LLM-friendly") - 200.0 / 950.0).abs() < 1e-9);
}

#[test]
fn a_metric_without_a_good_side_has_two_shares_and_one_callout() {
    let nodes = nodes();
    let words = words(ColorBy::Cycles);
    assert_eq!(shares(&nodes, &words).len(), 2);
    let callouts = callouts(&nodes, &words);
    assert_eq!(callouts.len(), 1);
    assert_eq!(callouts[0].label, "Dependency cycles");
}

#[test]
fn callouts_point_at_the_directories_where_each_side_gathers() {
    let nodes = nodes();
    let callouts = callouts(&nodes, &words(ColorBy::Llm));
    assert_eq!(name(&nodes, callouts[0].node), "bad/");
    assert_eq!(callouts[0].label, "Unhealthy code: LLM risk");
    assert_eq!(name(&nodes, callouts[1].node), "good/");
    assert_eq!(callouts[1].heat, 1.0);
}

#[test]
fn no_side_no_callout() {
    let files = vec![
        (PathBuf::from("a/x.rs"), 10, shade(0.0)),
        (PathBuf::from("b/y.rs"), 10, shade(0.0)),
    ];
    assert!(callouts(&layout(&files), &words(ColorBy::Llm)).is_empty());
}
