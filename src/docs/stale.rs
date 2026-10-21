//! Documents behind the code they describe: files a document covers or
//! names changed after the document last did.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::describes::{Rules, globs, is_document, names_of};
use super::markdown;
use crate::git::CommitsByFile;

/// A described file that changed after its document.
#[derive(Debug, PartialEq)]
pub struct Behind {
    pub path: PathBuf,
    /// When the file last changed.
    pub changed: i64,
    /// Its commits since the document last changed.
    pub commits: usize,
}

/// A document with the files it describes that changed after it.
#[derive(Debug, PartialEq)]
pub struct StaleDocument {
    pub document: PathBuf,
    /// When the document last changed.
    pub updated: i64,
    /// Distinct commits to the files since then.
    pub commits: usize,
    /// Most recent change first.
    pub files: Vec<Behind>,
}

impl StaleDocument {
    /// Seconds from the document's last change to the last change of what
    /// it describes.
    pub fn behind(&self) -> i64 {
        self.files.first().map_or(0, |f| f.changed - self.updated)
    }
}

/// The files of the repository, to resolve what documents name.
pub struct Files {
    present: HashSet<PathBuf>,
    /// Each file under every trailing part of its path.
    by_end: HashMap<PathBuf, Vec<PathBuf>>,
    /// Each file under every directory of two names or more above it.
    by_dir: HashMap<PathBuf, Vec<PathBuf>>,
}

impl Files {
    pub fn new(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut files = Files {
            present: HashSet::new(),
            by_end: HashMap::new(),
            by_dir: HashMap::new(),
        };
        for path in paths {
            let names: Vec<&std::ffi::OsStr> = path.iter().collect();
            for start in 0..names.len() {
                let end: PathBuf = names[start..].iter().collect();
                files.by_end.entry(end).or_default().push(path.clone());
            }
            for depth in 2..names.len() {
                let dir: PathBuf = names[..depth].iter().collect();
                files.by_dir.entry(dir).or_default().push(path.clone());
            }
            files.present.insert(path);
        }
        files
    }

    /// The files a reference to `name` names: the file itself, those it
    /// is the end of when it need not be whole, and those under it.
    fn named(&self, name: &Path, whole: bool) -> Vec<&PathBuf> {
        let exact = self.present.get(name).into_iter();
        let by_end = (!whole)
            .then(|| self.by_end.get(name))
            .flatten()
            .into_iter()
            .flatten();
        let under = self.by_dir.get(name).into_iter().flatten();
        exact.chain(by_end).chain(under).collect()
    }
}

/// The files `document` describes: those it covers and those it names.
/// Documents are left out: they describe, they are not described.
fn described<'a>(
    document: &Path,
    text: &str,
    files: &'a Files,
    rules: &Rules,
) -> BTreeSet<&'a PathBuf> {
    let mut found = BTreeSet::new();
    let covered = rules.covered_by(document, text);
    if !covered.is_empty()
        && let Ok(set) = globs(&covered)
    {
        found.extend(files.present.iter().filter(|path| set.is_match(path)));
    }
    for reference in markdown::scan(text).references {
        if let Some((name, whole)) = names_of(document, &reference) {
            found.extend(files.named(&name, whole));
        }
    }
    found.retain(|path| !is_document(path) && path.as_path() != document);
    found
}

/// The documents behind what they describe, furthest behind first.
/// `documents` are relative to the root, with their text; `files` the
/// files of the working tree; `history` the commits of every file.
pub fn stale(
    documents: &[(PathBuf, String)],
    files: &Files,
    history: &CommitsByFile,
    rules: &Rules,
) -> Vec<StaleDocument> {
    let last = |path: &Path| history.get(path)?.iter().map(|c| c.1).max();
    let mut stale: Vec<StaleDocument> = documents
        .iter()
        .filter(|(document, _)| !rules.ignores(document))
        .filter_map(|(document, text)| {
            let updated = last(document)?;
            let mut commits: HashSet<usize> = HashSet::new();
            let mut behind: Vec<Behind> = described(document, text, files, rules)
                .into_iter()
                .filter_map(|path| {
                    let after: Vec<usize> = history
                        .get(path)?
                        .iter()
                        .filter(|(_, time)| *time > updated)
                        .map(|(commit, _)| *commit)
                        .collect();
                    commits.extend(&after);
                    Some(Behind {
                        path: path.clone(),
                        changed: last(path)?,
                        commits: after.len(),
                    })
                    .filter(|b| b.commits > 0)
                })
                .collect();
            behind.sort_by(|a, b| b.changed.cmp(&a.changed).then_with(|| a.path.cmp(&b.path)));
            (!behind.is_empty()).then(|| StaleDocument {
                document: document.clone(),
                updated,
                commits: commits.len(),
                files: behind,
            })
        })
        .collect();
    stale.sort_by(|a, b| {
        b.behind()
            .cmp(&a.behind())
            .then_with(|| a.document.cmp(&b.document))
    });
    stale
}

#[cfg(test)]
#[path = "stale_test.rs"]
mod tests;
