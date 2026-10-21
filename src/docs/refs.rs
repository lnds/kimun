//! Dead references of documents: links and images that lead to no file or
//! to no heading, and paths in code spans that name nothing in the
//! repository.

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use ignore::WalkBuilder;
use serde::Serialize;

use super::markdown::{self, Form};

/// What a dead reference leads to that is not there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Missing {
    /// A link or image leads to no file.
    File,
    /// A link leads to a heading its document does not have.
    Heading,
    /// A code span names a path that is nowhere in the repository.
    Path,
}

impl Missing {
    pub fn label(self) -> &'static str {
        match self {
            Missing::File => "no such file",
            Missing::Heading => "no such heading",
            Missing::Path => "no such path",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Dead {
    pub line: usize,
    pub reference: String,
    pub form: Form,
    pub missing: Missing,
    /// The path the reference names, relative to the repository root.
    pub names: PathBuf,
}

/// What a document refers to, and which of those references are dead.
pub struct Checked {
    pub references: usize,
    pub dead: Vec<Dead>,
}

/// The files and directories of a repository, to tell whether a path
/// names one of them.
pub struct Repository {
    root: PathBuf,
    /// Every path of the working tree and its trailing parts: `src/a/b.rs`,
    /// `a/b.rs`, `b.rs`, and the same for directories.
    present: HashSet<PathBuf>,
    /// The name of every file and directory of the working tree.
    names: HashSet<String>,
    /// The paths the history has held, with their trailing parts, read
    /// the first time a code span names a path the working tree lacks.
    history: OnceCell<Option<HashSet<PathBuf>>>,
    load_history: Box<dyn Fn() -> Option<HashSet<PathBuf>>>,
    anchors: RefCell<HashMap<PathBuf, HashSet<String>>>,
}

impl Repository {
    /// The repository under `root`, ignored files left out; `history`
    /// gives the file paths its history has held, when it has one.
    pub fn index(root: &Path, history: impl Fn() -> Option<HashSet<PathBuf>> + 'static) -> Self {
        let paths: Vec<PathBuf> = WalkBuilder::new(root)
            .hidden(false)
            .filter_entry(|entry| entry.file_name() != ".git")
            .build()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let relative = entry.path().strip_prefix(root).ok()?;
                (!relative.as_os_str().is_empty()).then(|| relative.to_path_buf())
            })
            .collect();
        let names = paths
            .iter()
            .flat_map(|path| path.iter())
            .filter_map(|name| name.to_str().map(String::from))
            .collect();
        Self {
            root: root.to_path_buf(),
            present: parts_of(&paths),
            names,
            history: OnceCell::new(),
            load_history: Box::new(history),
            anchors: RefCell::default(),
        }
    }

    /// Whether a code span that names `names`, starting at `first`, is
    /// read as a path. With history, a path the repository has held at
    /// some time: an example of a layout or a file still planned is no
    /// dead path. Without, a path that starts at a name of the working
    /// tree.
    fn knows(&self, names: &Path, first: &str, from_document: bool) -> bool {
        let history = self
            .history
            .get_or_init(|| (self.load_history)().map(|paths| parts_of(&paths)));
        match history {
            Some(paths) => paths.contains(names),
            None => from_document || self.names.contains(first),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn anchors_of(&self, document: &Path) -> bool {
        self.anchors.borrow().contains_key(document)
    }

    fn has_anchor(&self, document: &Path, anchor: &str) -> bool {
        if !self.anchors_of(document) {
            let text = std::fs::read_to_string(document).unwrap_or_default();
            let found = markdown::scan(&text).anchors;
            self.anchors
                .borrow_mut()
                .insert(document.to_path_buf(), found);
        }
        self.anchors
            .borrow()
            .get(document)
            .is_some_and(|anchors| has_anchor(anchors, anchor))
    }
}

/// Every run of consecutive names of each path: `a/b/c.rs` gives
/// `a/b/c.rs`, `b/c.rs`, `c.rs`, and the directories `a/b`, `a`, `b`.
fn parts_of<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> HashSet<PathBuf> {
    let mut parts = HashSet::new();
    for path in paths {
        let names: Vec<&str> = path.iter().filter_map(|p| p.to_str()).collect();
        for start in 0..names.len() {
            for end in start + 1..=names.len() {
                parts.insert(names[start..end].iter().collect());
            }
        }
    }
    parts
}

fn has_anchor(anchors: &HashSet<String>, anchor: &str) -> bool {
    anchors.contains(anchor) || anchors.contains(&anchor.to_lowercase())
}

/// Check the references of `document`, whose text is `text`.
pub fn check(document: &Path, text: &str, repository: &Repository) -> Checked {
    let scan = markdown::scan(text);
    let dir = document.parent().unwrap_or(Path::new(""));
    let mut checked = Checked {
        references: 0,
        dead: Vec::new(),
    };
    for reference in &scan.references {
        let verdict = match reference.form {
            Form::Link | Form::Image => {
                check_link(dir, &reference.target, &scan.anchors, repository)
            }
            Form::Code => check_path(dir, &reference.target, repository),
            Form::Note => None,
        };
        let Some(verdict) = verdict else {
            continue;
        };
        checked.references += 1;
        if let Err((missing, names)) = verdict {
            checked.dead.push(Dead {
                line: reference.line,
                reference: reference.target.clone(),
                form: reference.form,
                missing,
                names,
            });
        }
    }
    checked
}

/// `None` for a reference that is not checked; else whether it is alive,
/// or why not and the path it names.
type Verdict = Option<Result<(), (Missing, PathBuf)>>;

fn check_link(dir: &Path, target: &str, own: &HashSet<String>, repository: &Repository) -> Verdict {
    let target = decode(target);
    if is_external(&target) {
        return None;
    }
    let (path, anchor) = target.split_once('#').unwrap_or((&target, ""));
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    if path.is_empty() {
        if anchor.is_empty() {
            return None;
        }
        return Some(match has_anchor(own, anchor) {
            true => Ok(()),
            false => Err((Missing::Heading, PathBuf::new())),
        });
    }
    let full = match path.strip_prefix('/') {
        Some(rooted) => repository.root.join(rooted),
        None => dir.join(path),
    };
    let names = relative(&full, &repository.root);
    if !full.exists() && path.starts_with('/') && Path::new(path).extension().is_none() {
        // A route of a site built from the documents, not a file.
        return None;
    }
    if !full.exists() {
        return Some(Err((Missing::File, names)));
    }
    if !anchor.is_empty() && is_markdown(&full) && !repository.has_anchor(&full, anchor) {
        return Some(Err((Missing::Heading, names)));
    }
    Some(Ok(()))
}

/// A code span shaped like a path: alive when it exists from the document
/// or from the root, or when it is the trailing part of a path of the
/// working tree; dead when the repository knows it
/// ([`Repository::knows`]); else no path at all.
fn check_path(dir: &Path, span: &str, repository: &Repository) -> Verdict {
    let path = path_in(span)?;
    let first = path.split('/').next()?;
    let from_document = first == "." || first == "..";
    let names = match from_document {
        true => relative(&dir.join(path), &repository.root),
        false => PathBuf::from(path.trim_start_matches("./").trim_end_matches('/')),
    };
    let alive = dir.join(path).exists()
        || (!from_document
            && (repository.root.join(path).exists() || repository.present.contains(&names)));
    if alive {
        Some(Ok(()))
    } else if repository.knows(&names, first, from_document) {
        Some(Err((Missing::Path, names)))
    } else {
        None
    }
}

/// The path a code span holds, without a trailing `:line` or `:line:col`;
/// none when it holds anything but a relative path with a `/`.
pub(crate) fn path_in(span: &str) -> Option<&str> {
    let mut path = span;
    for _ in 0..2 {
        if let Some((head, tail)) = path.rsplit_once(':')
            && !tail.is_empty()
            && tail.chars().all(|c| c.is_ascii_digit())
        {
            path = head;
        }
    }
    let shaped = path.contains('/')
        && !path.starts_with('/')
        && path
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+'));
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let whole = parts.iter().all(|p| !p.is_empty())
        && parts
            .iter()
            .skip_while(|p| matches!(**p, "." | ".."))
            .count()
            > 0;
    (shaped && whole).then_some(path)
}

pub(crate) fn is_external(target: &str) -> bool {
    let scheme = target.split_once(':').is_some_and(|(scheme, _)| {
        scheme.len() > 1
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    });
    scheme || target.starts_with("//") || target.contains("{{") || target.contains('$')
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "md" | "markdown"))
}

/// `%XX` escapes of a link decoded.
pub(crate) fn decode(target: &str) -> String {
    let bytes = target.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                i += 3;
            }
            (byte, _) => {
                decoded.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// `path` relative to `root`, with `.` and `..` resolved by name.
pub(crate) fn relative(path: &Path, root: &Path) -> PathBuf {
    let mut parts: Vec<Component> = Vec::new();
    for component in path.strip_prefix(root).unwrap_or(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.iter().collect()
}

#[cfg(test)]
#[path = "refs_test.rs"]
mod tests;
