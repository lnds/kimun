//! Circle packing, as `d3.pack` lays it out: siblings are placed one by
//! one tangent to two circles of a front chain (Wang et al., 2006), and a
//! parent is the smallest circle enclosing its children (Welzl's
//! algorithm). The order of the input is the order of placing, and the
//! shuffle of the enclosing step has a fixed seed, so the layout is the
//! same on every run.

use super::enclose::enclose;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

impl Circle {
    pub(super) fn at(x: f64, y: f64, r: f64) -> Circle {
        Circle { x, y, r }
    }
}

/// Place `c` tangent to both `a` and `b`.
fn place(b: Circle, a: Circle, c: &mut Circle) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let d2 = dx * dx + dy * dy;
    if d2 == 0.0 {
        (c.x, c.y) = (a.x + c.r, a.y);
        return;
    }
    let a2 = (a.r + c.r).powi(2);
    let b2 = (b.r + c.r).powi(2);
    if a2 > b2 {
        let x = (d2 + b2 - a2) / (2.0 * d2);
        let y = (b2 / d2 - x * x).max(0.0).sqrt();
        (c.x, c.y) = (b.x - x * dx - y * dy, b.y - x * dy + y * dx);
    } else {
        let x = (d2 + a2 - b2) / (2.0 * d2);
        let y = (a2 / d2 - x * x).max(0.0).sqrt();
        (c.x, c.y) = (a.x + x * dx - y * dy, a.y + x * dy + y * dx);
    }
}

fn intersects(a: Circle, b: Circle) -> bool {
    let dr = a.r + b.r - 1e-6;
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    dr > 0.0 && dr * dr > dx * dx + dy * dy
}

/// The front chain: circles by index, each linked to the next and the
/// previous one around the packing.
struct Chain {
    next: Vec<usize>,
    prev: Vec<usize>,
}

impl Chain {
    fn link(&mut self, a: usize, b: usize) {
        self.next[a] = b;
        self.prev[b] = a;
    }

    /// How far from the origin the pair `a`, `a.next` sits, weighed by radii.
    fn score(&self, circles: &[Circle], a: usize) -> f64 {
        let (a, b) = (circles[a], circles[self.next[a]]);
        let ab = a.r + b.r;
        let dx = (a.x * b.r + b.x * a.r) / ab;
        let dy = (a.y * b.r + b.y * a.r) / ab;
        dx * dx + dy * dy
    }

    /// The circles of the chain from `start` around back to it.
    fn around(&self, start: usize) -> Vec<usize> {
        let mut found = vec![start];
        let mut c = self.next[start];
        while c != start {
            found.push(c);
            c = self.next[c];
        }
        found
    }

    /// Of the pairs that start on the chain, from `from` around, the one
    /// nearest to the origin; the pair starting at `placed`, the circle just
    /// placed, is not one.
    fn nearest(&self, circles: &[Circle], from: usize, placed: usize) -> usize {
        let mut pairs = self.around(from);
        pairs.retain(|&c| c != placed);
        let by_score =
            |&a: &usize, &b: &usize| self.score(circles, a).total_cmp(&self.score(circles, b));
        // The first of equal ones, as the walk around the chain meets them.
        pairs
            .into_iter()
            .reduce(|best, c| match by_score(&c, &best) {
                std::cmp::Ordering::Less => c,
                _ => best,
            })
            .unwrap_or(from)
    }

    /// The circle of the chain nearest to `a` and `b` that `c` overlaps,
    /// walking ahead of `b` and behind `a` by distance along the chain.
    fn overlapped(
        &self,
        circles: &[Circle],
        a: usize,
        b: usize,
        c: Circle,
    ) -> Option<(usize, bool)> {
        let (mut j, mut k) = (self.next[b], self.prev[a]);
        let (mut sj, mut sk) = (circles[b].r, circles[a].r);
        loop {
            if sj <= sk {
                if intersects(circles[j], c) {
                    return Some((j, true));
                }
                sj += circles[j].r;
                j = self.next[j];
            } else {
                if intersects(circles[k], c) {
                    return Some((k, false));
                }
                sk += circles[k].r;
                k = self.prev[k];
            }
            if j == self.next[k] {
                return None;
            }
        }
    }
}

/// Lay out circles of the given radii side by side around the origin; the
/// result is their centres and the radius of the circle enclosing them.
pub fn siblings(radii: &[f64]) -> (Vec<Circle>, f64) {
    let mut circles: Vec<Circle> = radii.iter().map(|&r| Circle::at(0.0, 0.0, r)).collect();
    let n = circles.len();
    match n {
        0 => return (circles, 0.0),
        1 => return (circles, radii[0]),
        _ => {}
    }
    circles[0].x = -circles[1].r;
    circles[1].x = circles[0].r;
    if n == 2 {
        return (circles, radii[0] + radii[1]);
    }
    let (a0, b0) = (circles[0], circles[1]);
    place(b0, a0, &mut circles[2]);
    let mut chain = Chain {
        next: vec![0; n],
        prev: vec![0; n],
    };
    chain.link(0, 1);
    chain.link(1, 2);
    chain.link(2, 0);
    let (mut a, mut b) = (0, 1);
    let mut i = 3;
    while i < n {
        let (ca, cb) = (circles[a], circles[b]);
        place(ca, cb, &mut circles[i]);
        if let Some((hit, ahead)) = chain.overlapped(&circles, a, b, circles[i]) {
            match ahead {
                true => b = hit,
                false => a = hit,
            }
            chain.link(a, b);
            continue;
        }
        chain.link(a, i);
        chain.link(i, b);
        // The pair nearest to the origin is where the next circle goes.
        a = chain.nearest(&circles, a, i);
        b = chain.next[a];
        i += 1;
    }
    let front = chain.around(b).into_iter().map(|i| circles[i]).collect();
    let e = enclose(front);
    for circle in &mut circles {
        circle.x -= e.x;
        circle.y -= e.y;
    }
    (circles, e.r)
}

#[cfg(test)]
#[path = "pack_test.rs"]
mod tests;
