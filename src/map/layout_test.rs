use super::*;

fn files(entries: &[(&str, usize)]) -> Vec<(PathBuf, usize, Option<Shade>)> {
    entries
        .iter()
        .map(|&(p, n)| (PathBuf::from(p), n, None))
        .collect()
}

fn distance(a: &Placed, b: &Placed) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

#[test]
fn the_root_fills_the_map_and_every_node_follows_its_parent() {
    let placed = layout(&files(&[
        ("a.rs", 100),
        ("core/b.rs", 50),
        ("core/c.rs", 25),
        ("web/d.rs", 400),
    ]));
    assert_eq!(placed[0].parent, None);
    assert!((placed[0].r - SIZE / 2.0).abs() < 1e-9);
    for (i, node) in placed.iter().enumerate().skip(1) {
        assert!(node.parent.unwrap() < i);
    }
}

#[test]
fn every_node_lies_inside_its_parent() {
    let placed = layout(&files(&[
        ("a.rs", 100),
        ("core/b.rs", 50),
        ("core/c.rs", 25),
        ("core/x/e.rs", 9),
        ("web/d.rs", 400),
    ]));
    for node in placed.iter().skip(1) {
        let parent = &placed[node.parent.unwrap()];
        assert!(
            distance(node, parent) + node.r <= parent.r + 1e-6,
            "{} outside {}",
            node.path,
            parent.path
        );
    }
}

#[test]
fn area_follows_lines_and_directories_sum_them() {
    let placed = layout(&files(&[("a/x.rs", 100), ("a/y.rs", 400), ("b.rs", 1)]));
    let file = |name: &str| placed.iter().find(|n| n.name == name).unwrap();
    assert!((file("y.rs").r / file("x.rs").r - 2.0).abs() < 1e-9);
    assert_eq!(file("a/").lines, 500);
    assert_eq!(placed[0].lines, 501);
}

#[test]
fn files_carry_their_shade_and_directories_none() {
    let shade = Shade {
        heat: -1.0,
        label: "in a cycle".to_string(),
    };
    let input = vec![
        (PathBuf::from("a/x.rs"), 10, Some(shade.clone())),
        (PathBuf::from("b.rs"), 10, None),
    ];
    let placed = layout(&input);
    let x = placed.iter().find(|n| n.name == "x.rs").unwrap();
    assert_eq!(x.shade, Some(shade));
    assert!(placed.iter().filter(|n| n.dir).all(|n| n.shade.is_none()));
}
