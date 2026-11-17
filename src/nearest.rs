//! Choosing among several files by the directories they share with
//! another, for the readers of `deps` and the links of `docs`.

use std::path::Path;

/// How many leading directories two paths share.
pub fn shared_depth(a: &Path, b: &Path) -> usize {
    a.components()
        .zip(b.components())
        .take_while(|(x, y)| x == y)
        .count()
}

/// The one file of `pool` deepest by `depth`, or `Err` when none or
/// several are.
pub fn nearest(pool: &[usize], depth: impl Fn(usize) -> usize) -> Result<usize, ()> {
    let deepest = pool.iter().map(|&f| depth(f)).max().ok_or(())?;
    let mut at_deepest = pool.iter().filter(|&&f| depth(f) == deepest);
    match (at_deepest.next(), at_deepest.next()) {
        (Some(&only), None) => Ok(only),
        _ => Err(()),
    }
}
