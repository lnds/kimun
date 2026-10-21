//! The links between documents, as a graph of notes: relative Markdown
//! links and `[[wikilinks]]`, resolved by path or by note name as Obsidian,
//! Foam and Logseq resolve them.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::markdown::{self, Form};
use super::refs::{decode, is_external, relative};
use crate::deps::graph::{nearest, shared_depth};

/// Where a link leads.
#[derive(Debug, PartialEq, Eq)]
pub enum Resolved {
    /// To the file of this index.
    To(usize),
    /// To nothing: no file has that path, no note that name.
    Broken,
    /// Outside the files of the graph: an existing file of another kind,
    /// or a note named by several files, none nearer.
    Outside,
}

/// A link of a document, as written.
#[derive(Debug, PartialEq, Eq)]
pub struct Link {
    pub line: usize,
    pub target: String,
    /// A wikilink names a note; a Markdown link a path.
    pub wiki: bool,
}

/// The links of a document to other files: Markdown links and images
/// that are no URL, and wikilinks. Anchors alone link nowhere.
pub fn links(text: &str) -> Vec<Link> {
    markdown::scan(text)
        .references
        .into_iter()
        .filter_map(|r| {
            let wiki = match r.form {
                Form::Note => true,
                Form::Link | Form::Image => false,
                Form::Code => return None,
            };
            let target = match wiki {
                true => r.target,
                false => decode(&r.target),
            };
            let path = target.split(['#', '?']).next().unwrap_or("");
            (!path.is_empty() && !is_external(&target)).then_some(Link {
                line: r.line,
                target,
                wiki,
            })
        })
        .collect()
}

/// The files of a graph, by normalized path and, for Markdown notes, by
/// name.
pub struct Notes {
    paths: Vec<PathBuf>,
    by_path: HashMap<PathBuf, usize>,
    by_name: HashMap<String, Vec<usize>>,
}

/// A note's name as a wikilink writes it: its path without the Markdown
/// extension, compared without case.
fn note_key(path: &Path) -> String {
    let text = path.to_string_lossy().to_lowercase();
    text.strip_suffix(".md")
        .or_else(|| text.strip_suffix(".markdown"))
        .unwrap_or(&text)
        .to_string()
}

impl Notes {
    /// The files of `paths`, as walked; each keeps its index.
    pub fn new(paths: &[PathBuf]) -> Self {
        let mut notes = Notes {
            paths: paths.iter().map(|p| relative(p, Path::new(""))).collect(),
            by_path: HashMap::new(),
            by_name: HashMap::new(),
        };
        for (index, path) in notes.paths.iter().enumerate() {
            notes.by_path.insert(path.clone(), index);
            if let Some(name) = path.file_name() {
                let key = note_key(Path::new(name));
                notes.by_name.entry(key).or_default().push(index);
            }
        }
        notes
    }

    /// Where the link of the file `from` leads. `exists` tells whether a
    /// path, relative to the walk, is a file outside the graph.
    pub fn resolve(&self, from: usize, link: &Link, exists: impl Fn(&Path) -> bool) -> Resolved {
        let path = link.target.split(['#', '?']).next().unwrap_or("");
        let dir = self.paths[from].parent().unwrap_or(Path::new(""));
        if !link.wiki {
            let full = match path.strip_prefix('/') {
                Some(rooted) => relative(Path::new(rooted), Path::new("")),
                None => relative(&dir.join(path), Path::new("")),
            };
            // An absolute path without extension is a route of a site.
            let route = path.starts_with('/') && full.extension().is_none();
            return match self.by_path.get(&full) {
                Some(&to) => Resolved::To(to),
                None if route || exists(&full) => Resolved::Outside,
                None => Resolved::Broken,
            };
        }
        self.note(from, path, &exists)
    }

    /// The note a wikilink names: by its path from the root, else by its
    /// name, the nearest of several.
    fn note(&self, from: usize, name: &str, exists: &impl Fn(&Path) -> bool) -> Resolved {
        let key = note_key(Path::new(name.trim()));
        let by_path = self
            .paths
            .iter()
            .position(|p| note_key(p) == key || p.to_string_lossy().to_lowercase() == key);
        if let Some(to) = by_path {
            return Resolved::To(to);
        }
        let last = key.rsplit('/').next().unwrap_or(&key);
        let pool: Vec<usize> = self
            .by_name
            .get(last)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&n| {
                let note = note_key(&self.paths[n]);
                note == key || note.ends_with(&format!("/{key}"))
            })
            .collect();
        match nearest(&pool, |n| shared_depth(&self.paths[n], &self.paths[from])) {
            Ok(to) => Resolved::To(to),
            Err(()) if !pool.is_empty() || exists(Path::new(name)) => Resolved::Outside,
            Err(()) => Resolved::Broken,
        }
    }
}

/// A link that leads nowhere.
#[derive(Debug, PartialEq, Eq)]
pub struct Broken {
    pub from: usize,
    pub line: usize,
    pub target: String,
}

/// The graph of notes: who links to whom, and the links that lead nowhere.
#[derive(Debug, Default)]
pub struct Graph {
    /// The notes each note links to.
    pub out: Vec<BTreeSet<usize>>,
    pub broken: Vec<Broken>,
}

impl Graph {
    /// The graph of `documents`, each a path and its text; `exists` tells
    /// whether a path outside them is a file.
    pub fn build(documents: &[(PathBuf, String)], exists: impl Fn(&Path) -> bool) -> Self {
        let paths: Vec<PathBuf> = documents.iter().map(|(p, _)| p.clone()).collect();
        let notes = Notes::new(&paths);
        let mut graph = Graph::default();
        for (from, (_, text)) in documents.iter().enumerate() {
            let mut out = BTreeSet::new();
            for link in links(text) {
                match notes.resolve(from, &link, &exists) {
                    Resolved::To(to) if to != from => {
                        out.insert(to);
                    }
                    Resolved::Broken => graph.broken.push(Broken {
                        from,
                        line: link.line,
                        target: link.target,
                    }),
                    _ => {}
                }
            }
            graph.out.push(out);
        }
        graph
    }

    /// How many notes link to each.
    pub fn fan_in(&self) -> Vec<usize> {
        let mut fan_in = vec![0; self.out.len()];
        for to in self.out.iter().flatten() {
            fan_in[*to] += 1;
        }
        fan_in
    }

    /// The groups of notes linked to each other, whatever the direction,
    /// largest first.
    pub fn groups(&self) -> Vec<Vec<usize>> {
        let mut root: Vec<usize> = (0..self.out.len()).collect();
        fn find(root: &mut [usize], n: usize) -> usize {
            let mut n = n;
            while root[n] != n {
                root[n] = root[root[n]];
                n = root[n];
            }
            n
        }
        for (from, out) in self.out.iter().enumerate() {
            for &to in out {
                let (a, b) = (find(&mut root, from), find(&mut root, to));
                root[a] = b;
            }
        }
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for n in 0..self.out.len() {
            let r = find(&mut root, n);
            groups.entry(r).or_default().push(n);
        }
        let mut groups: Vec<Vec<usize>> = groups.into_values().collect();
        groups.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
        groups
    }
}

/// Notes that are read first rather than linked to: an index, a README,
/// the instructions of an agent.
pub fn is_entry_point(path: &Path) -> bool {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        stem.as_str(),
        "readme"
            | "index"
            | "_index"
            | "home"
            | "summary"
            | "claude"
            | "agents"
            | "changelog"
            | "contributing"
            | "license"
            | "security"
            | "code_of_conduct"
    )
}

#[cfg(test)]
#[path = "links_test.rs"]
mod tests;
