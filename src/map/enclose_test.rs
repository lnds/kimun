use super::*;

#[test]
fn enclose_holds_every_circle() {
    let circles = vec![
        Circle::at(0.0, 0.0, 1.0),
        Circle::at(5.0, 0.0, 2.0),
        Circle::at(2.0, 4.0, 1.5),
    ];
    let e = enclose(circles.clone());
    for c in circles {
        assert!(((c.x - e.x).powi(2) + (c.y - e.y).powi(2)).sqrt() + c.r <= e.r + 1e-6);
    }
}
