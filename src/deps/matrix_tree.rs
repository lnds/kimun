//! The directories and files of a dependency matrix as a tree. The children
//! of each directory are ordered so that a sibling comes before the siblings
//! it uses: in a layered system every use then falls below the diagonal.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap, HashMap};
use std::path::{Path, PathBuf};

use super::analyzer::tarjan_scc;

/// The node every other one is under: the deepest directory holding every file.
pub const ROOT: usize = 0;

pub struct Node {
    /// As shown in a row: `graph.rs`, `deps/`, or `src/deps/` for a
    /// directory that holds only another one.
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<usize>,
    /// The file a leaf is, by index.
    pub file: Option<usize>,
}

impl Node {
    pub fn is_dir(&self) -> bool {
        self.file.is_none()
    }
}

pub struct Tree {
    pub nodes: Vec<Node>,
    /// For each file, the nodes from a child of the root down to the file.
    pub chains: Vec<Vec<usize>>,
}

/// The directory components every file sits under.
fn common_dir(files: &[PathBuf]) -> PathBuf {
    let dirs = files.iter().map(|f| f.parent().unwrap_or(Path::new("")));
    let mut common: Option<Vec<_>> = None;
    for dir in dirs {
        let parts: Vec<_> = dir.components().collect();
        common = Some(match common {
            None => parts,
            Some(c) => c
                .into_iter()
                .zip(parts)
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| a)
                .collect(),
        });
    }
    common.unwrap_or_default().iter().collect()
}

impl Tree {
    /// The tree of `files`, children by name.
    pub fn of(files: &[PathBuf]) -> Tree {
        let root = common_dir(files);
        let mut tree = Tree {
            nodes: vec![Node {
                name: String::new(),
                path: root.clone(),
                children: vec![],
                file: None,
            }],
            chains: vec![],
        };
        let mut index: HashMap<PathBuf, usize> = HashMap::new();
        for (i, file) in files.iter().enumerate() {
            let mut parent = ROOT;
            let mut path = root.clone();
            let rest: Vec<_> = file
                .strip_prefix(&root)
                .unwrap_or(file)
                .components()
                .collect();
            for (depth, part) in rest.iter().enumerate() {
                path.push(part);
                let is_file = depth + 1 == rest.len();
                let node = *index.entry(path.clone()).or_insert_with(|| {
                    let name = part.as_os_str().to_string_lossy();
                    tree.nodes.push(Node {
                        name: if is_file {
                            name.to_string()
                        } else {
                            format!("{name}/")
                        },
                        path: path.clone(),
                        children: vec![],
                        file: is_file.then_some(i),
                    });
                    let added = tree.nodes.len() - 1;
                    tree.nodes[parent].children.push(added);
                    added
                });
                parent = node;
            }
        }
        tree.compact(ROOT);
        tree.chains = vec![vec![]; files.len()];
        tree.chain(ROOT, &mut vec![]);
        for node in 0..tree.nodes.len() {
            let mut children = std::mem::take(&mut tree.nodes[node].children);
            children.sort_by(|a, b| tree.nodes[*a].name.cmp(&tree.nodes[*b].name));
            tree.nodes[node].children = children;
        }
        tree
    }

    /// Merge each directory holding only another directory into it.
    fn compact(&mut self, node: usize) {
        for child in self.nodes[node].children.clone() {
            while let [only] = self.nodes[child].children[..] {
                if !self.nodes[only].is_dir() {
                    break;
                }
                let inner = std::mem::take(&mut self.nodes[only].children);
                let name = std::mem::take(&mut self.nodes[only].name);
                self.nodes[child].name.push_str(&name);
                self.nodes[child].path = self.nodes[only].path.clone();
                self.nodes[child].children = inner;
            }
            self.compact(child);
        }
    }

    fn chain(&mut self, node: usize, above: &mut Vec<usize>) {
        if let Some(file) = self.nodes[node].file {
            self.chains[file] = above.clone();
        }
        for child in self.nodes[node].children.clone() {
            above.push(child);
            self.chain(child, above);
            above.pop();
        }
    }

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
