use super::*;

fn radii(n: usize) -> Vec<f64> {
    // Sizes as files have them: a few large, many small.
    (0..n)
        .map(|i| 30.0 / (1.0 + i as f64).sqrt() + 1.0)
        .collect()
}

fn overlap(a: Circle, b: Circle) -> f64 {
    a.r + b.r - ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

#[test]
fn siblings_never_overlap() {
    let (circles, _) = siblings(&radii(60));
    for (i, &a) in circles.iter().enumerate() {
        for &b in &circles[i + 1..] {
            assert!(overlap(a, b) < 1e-6, "{a:?} overlaps {b:?}");
        }
    }
}

#[test]
fn siblings_sit_inside_the_circle_returned_around_the_origin() {
    let (circles, r) = siblings(&radii(40));
    for c in circles {
        assert!((c.x * c.x + c.y * c.y).sqrt() + c.r <= r + 1e-6);
    }
}

#[test]
fn the_packing_is_tight() {
    let rs = radii(40);
    let (_, r) = siblings(&rs);
    let area: f64 = rs.iter().map(|r| r * r).sum();
    // Circles cover well over half of the circle around them.
    assert!(area / (r * r) > 0.55, "{}", area / (r * r));
}

#[test]
fn few_circles_are_placed_side_by_side() {
    assert_eq!(siblings(&[]).1, 0.0);
    assert_eq!(siblings(&[3.0]).1, 3.0);
    let (two, r) = siblings(&[2.0, 1.0]);
    assert_eq!(r, 3.0);
    assert!(overlap(two[0], two[1]).abs() < 1e-9);
}

#[test]
fn the_layout_is_the_same_every_time() {
    assert_eq!(siblings(&radii(50)), siblings(&radii(50)));
}
