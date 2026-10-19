//! Files as a tree of directories, rooted at the deepest directory holding
//! every file; a directory holding only another one is merged with it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
}

#[cfg(test)]
#[path = "file_tree_test.rs"]
mod tests;
