//! The order of a dependency matrix: the children of each directory are
//! sorted so that a sibling comes before the siblings it uses; in a layered
//! system every use then falls below the diagonal.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap, HashMap};

use super::analyzer::tarjan_scc;
use crate::file_tree::{ROOT, Tree};

impl Tree {
    /// Order the children of every directory by the uses between files:
    /// `(from, to)` by file index.
    pub fn order(&mut self, uses: &[(usize, usize)]) {
        let mut between: HashMap<usize, BTreeSet<(usize, usize)>> = HashMap::new();
        for &(from, to) in uses {
            let (a, b) = (&self.chains[from], &self.chains[to]);
            let k = a.iter().zip(b).take_while(|(x, y)| x == y).count();
            if k < a.len() && k < b.len() {
                let parent = if k == 0 { ROOT } else { a[k - 1] };
                between.entry(parent).or_default().insert((a[k], b[k]));
            }
        }
        for (parent, uses) in between {
            let children = &self.nodes[parent].children;
            self.nodes[parent].children = layered(children, &uses);
        }
    }
}

/// `nodes` (in their order of ties) so that each comes before those it
/// uses; the members of a cycle stay together.
fn layered(nodes: &[usize], uses: &BTreeSet<(usize, usize)>) -> Vec<usize> {
    let local: HashMap<usize, usize> = nodes.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    let mut adj = vec![vec![]; nodes.len()];
    for (a, b) in uses {
        adj[local[a]].push(local[b]);
    }
    let comps = Components::of(&adj);
    let order = comps.topological(&adj);
    let members = order.into_iter().flat_map(|c| comps.members[c].iter());
    members.map(|&i| nodes[i]).collect()
}

/// The cycles of a graph, each node alone when in none.
struct Components {
    members: Vec<Vec<usize>>,
    of: Vec<usize>,
}

impl Components {
    /// Inside a cycle, those that use more of it than it uses them go
    /// first, then by index.
    fn of(adj: &[Vec<usize>]) -> Components {
        let members = tarjan_scc(adj.len(), adj);
        let mut of = vec![0; adj.len()];
        for (c, comp) in members.iter().enumerate() {
            comp.iter().for_each(|&i| of[i] = c);
        }
        let mut balance = vec![0isize; adj.len()];
        let inside = adj
            .iter()
            .enumerate()
            .flat_map(|(i, out)| out.iter().map(move |&j| (i, j)));
        for (i, j) in inside.filter(|&(i, j)| of[i] == of[j]) {
            balance[i] += 1;
            balance[j] -= 1;
        }
        let mut comps = Components { members, of };
        for comp in &mut comps.members {
            comp.sort_by_key(|&i| (Reverse(balance[i]), i));
        }
        comps
    }

    /// The components so that each comes before those it uses; of those
    /// ready at once, the one with the lowest index first.
    fn topological(&self, adj: &[Vec<usize>]) -> Vec<usize> {
        let count = self.members.len();
        let mut next: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); count];
        for (i, out) in adj.iter().enumerate() {
            let (from, of) = (self.of[i], &self.of);
            next[from].extend(out.iter().map(|&j| of[j]).filter(|&c| c != from));
        }
        let mut waiting = vec![0usize; count];
        next.iter().flatten().for_each(|&c| waiting[c] += 1);
        let first = |c: usize| self.members[c].iter().copied().min().unwrap_or(0);
        let mut ready: BinaryHeap<Reverse<(usize, usize)>> = (0..count)
            .filter(|&c| waiting[c] == 0)
            .map(|c| Reverse((first(c), c)))
            .collect();
        let mut order = Vec::with_capacity(count);
        while let Some(Reverse((_, c))) = ready.pop() {
            order.push(c);
            for &d in &next[c] {
                waiting[d] -= 1;
                if waiting[d] == 0 {
                    ready.push(Reverse((first(d), d)));
                }
            }
        }
        order
    }
}

#[cfg(test)]
#[path = "matrix_tree_test.rs"]
mod tests;
