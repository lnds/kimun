//! The smallest circle enclosing others, by Welzl's algorithm with the
//! move-to-front of `d3.packEnclose`; the circles are shuffled with a fixed
//! seed, so the result is the same on every run.

use super::pack::Circle;

/// Whether `a` does not hold `b` whole.
fn encloses_not(a: Circle, b: Circle) -> bool {
    let dr = a.r - b.r;
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    dr < 0.0 || dr * dr < dx * dx + dy * dy
}

fn encloses_weak(a: Circle, b: Circle) -> bool {
    let dr = a.r - b.r + a.r.max(b.r).max(1.0) * 1e-9;
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    dr > 0.0 && dr * dr > dx * dx + dy * dy
}

fn encloses_all(a: Circle, basis: &[Circle]) -> bool {
    basis.iter().all(|&b| encloses_weak(a, b))
}

fn enclose_two(a: Circle, b: Circle) -> Circle {
    let (x21, y21, r21) = (b.x - a.x, b.y - a.y, b.r - a.r);
    let l = (x21 * x21 + y21 * y21).sqrt();
    Circle::at(
        (a.x + b.x + x21 / l * r21) / 2.0,
        (a.y + b.y + y21 / l * r21) / 2.0,
        (l + a.r + b.r) / 2.0,
    )
}

fn enclose_three(a: Circle, b: Circle, c: Circle) -> Circle {
    let (x1, y1, r1) = (a.x, a.y, a.r);
    let (a2, a3) = (x1 - b.x, x1 - c.x);
    let (b2, b3) = (y1 - b.y, y1 - c.y);
    let (c2, c3) = (b.r - r1, c.r - r1);
    let d1 = x1 * x1 + y1 * y1 - r1 * r1;
    let d2 = d1 - b.x * b.x - b.y * b.y + b.r * b.r;
    let d3 = d1 - c.x * c.x - c.y * c.y + c.r * c.r;
    let ab = a3 * b2 - a2 * b3;
    let xa = (b2 * d3 - b3 * d2) / (ab * 2.0) - x1;
    let xb = (b3 * c2 - b2 * c3) / ab;
    let ya = (a3 * d2 - a2 * d3) / (ab * 2.0) - y1;
    let yb = (a2 * c3 - a3 * c2) / ab;
    let qa = xb * xb + yb * yb - 1.0;
    let qb = 2.0 * (r1 + xa * xb + ya * yb);
    let qc = xa * xa + ya * ya - r1 * r1;
    let r = -match qa.abs() > 1e-6 {
        true => (qb + (qb * qb - 4.0 * qa * qc).sqrt()) / (2.0 * qa),
        false => qc / qb,
    };
    Circle::at(x1 + xa + xb * r, y1 + ya + yb * r, r)
}

fn enclose_basis(basis: &[Circle]) -> Circle {
    match basis {
        [a] => *a,
        [a, b] => enclose_two(*a, *b),
        [a, b, c] => enclose_three(*a, *b, *c),
        _ => Circle::at(0.0, 0.0, 0.0),
    }
}

/// The smallest basis holding `basis` and `p`, with `p` on its boundary.
fn extend_basis(basis: &[Circle], p: Circle) -> Vec<Circle> {
    if encloses_all(p, basis) {
        return vec![p];
    }
    for &b in basis {
        if encloses_not(p, b) && encloses_all(enclose_two(b, p), basis) {
            return vec![b, p];
        }
    }
    three_of(basis, p).unwrap_or_else(|| vec![p])
}

/// Two circles of `basis` that with `p` make the basis; only rounding
/// finds none, and the circle alone then keeps the search going.
fn three_of(basis: &[Circle], p: Circle) -> Option<Vec<Circle>> {
    let pairs = basis
        .iter()
        .enumerate()
        .flat_map(|(i, &bi)| basis[i + 1..].iter().map(move |&bj| (bi, bj)));
    pairs.into_iter().find_map(|(bi, bj)| {
        let holds = encloses_not(enclose_two(bi, bj), p)
            && encloses_not(enclose_two(bi, p), bj)
            && encloses_not(enclose_two(bj, p), bi);
        (holds && encloses_all(enclose_three(bi, bj, p), basis)).then(|| vec![bi, bj, p])
    })
}

/// A fixed sequence in [0, 1), the linear congruential generator d3 uses.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = (1_664_525 * self.0 + 1_013_904_223) % 4_294_967_296;
        self.0 as f64 / 4_294_967_296.0
    }
}

/// The smallest circle enclosing `circles`.
pub fn enclose(mut circles: Vec<Circle>) -> Circle {
    let mut random = Lcg(1);
    for m in (1..=circles.len()).rev() {
        let i = (random.next() * m as f64) as usize;
        circles.swap(m - 1, i);
    }
    let mut basis: Vec<Circle> = Vec::new();
    let mut e: Option<Circle> = None;
    let mut i = 0;
    while i < circles.len() {
        let p = circles[i];
        if e.is_some_and(|e| encloses_weak(e, p)) {
            i += 1;
        } else {
            basis = extend_basis(&basis, p);
            e = Some(enclose_basis(&basis));
            i = 0;
        }
    }
    e.unwrap_or(Circle::at(0.0, 0.0, 0.0))
}

#[cfg(test)]
#[path = "enclose_test.rs"]
mod tests;
