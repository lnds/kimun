use std::path::PathBuf;

use crate::git::{ChangeKind, FileDiffStat};
use crate::impact::structural::{Changed, Structural, compute};

/// `A.a` changes. `B.b1` calls it, `B.b2` does not; `C` calls only `b2`,
/// `D` calls `b1`, `E` imports `B` whole, `F` calls `D`.
fn chain() -> Vec<(PathBuf, String)> {
    [
        (
            "lib/a.ex",
            "defmodule A do\n  def a, do: 1\n  def other, do: 2\nend\n",
        ),
        (
            "lib/b.ex",
            "defmodule B do\n  def b1, do: A.a()\n  def b2, do: :x\nend\n",
        ),
        ("lib/c.ex", "defmodule C do\n  def c, do: B.b2()\nend\n"),
        ("lib/d.ex", "defmodule D do\n  def d, do: B.b1()\nend\n"),
        (
            "lib/e.ex",
            "defmodule E do\n  import B\n  def e, do: b2()\nend\n",
        ),
        ("lib/f.ex", "defmodule F do\n  def f, do: D.d()\nend\n"),
        (
            "lib/g.ex",
            "defmodule G do\n  @top B.b1()\n  def g, do: @top\nend\n",
        ),
    ]
    .iter()
    .map(|(p, t)| (PathBuf::from(p), t.to_string()))
    .collect()
}

fn radius(max_distance: Option<usize>) -> Structural {
    let change = FileDiffStat {
        path: PathBuf::from("lib/a.ex"),
        old_path: Some(PathBuf::from("lib/a.ex")),
        kind: ChangeKind::Modified,
        added: 1,
        deleted: 0,
        lines: vec![2],
        old_lines: vec![2],
        probe: Vec::new(),
    };
    let changed: Vec<Changed> = vec![&change];
    compute(chain(), &changed, &|_, _| false, max_distance)
}

fn reached(radius: &Structural) -> Vec<Vec<String>> {
    let name = |p: &PathBuf| p.file_name().unwrap().to_string_lossy().into_owned();
    radius
        .reached_files
        .iter()
        .map(|level| level.iter().map(name).collect())
        .collect()
}

#[test]
fn the_radius_goes_on_only_through_the_functions_that_reach_the_change() {
    let radius = radius(None);
    // C calls only `B.b2`, which does not reach `A.a`.
    assert_eq!(
        reached(&radius),
        vec![vec!["b.ex"], vec!["d.ex", "e.ex", "g.ex"], vec!["f.ex"]]
    );
    assert_eq!(radius.cut_at, None);
}

#[test]
fn a_whole_import_and_a_call_outside_functions_count_the_whole_file() {
    let names = reached(&radius(None));
    // `E` imports `B` without `only:`; `G` calls `b1` in its module body.
    assert!(names[1].contains(&"e.ex".to_string()) && names[1].contains(&"g.ex".to_string()));
}

#[test]
fn max_distance_stops_the_radius_and_says_so() {
    let radius = radius(Some(2));
    assert_eq!(radius.by_distance, vec![1, 3]);
    assert_eq!(radius.cut_at, Some(2));
    assert_eq!(
        super::super::structural::compute(chain(), &[], &|_, _| false, Some(1)).cut_at,
        None
    );
}

#[test]
fn the_cut_is_told_in_the_reach_line_and_the_json() {
    let radius = radius(Some(2));
    let line = crate::impact::structural_report::reach_line(&radius);
    assert!(
        line.ends_with("; stopped at distance 2 (--max-distance)"),
        "{line}"
    );
    let json = serde_json::to_value(crate::impact::structural_json::JsonStructural::from(
        &radius,
    ))
    .unwrap();
    assert_eq!(json["cut_at"], 2);
}
