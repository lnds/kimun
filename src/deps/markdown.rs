//! Markdown documents in the graph: each links to the documents and files
//! its links lead to (`crate::docs::links`), when `--kind docs` asks for
//! them.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::graph::{EdgeKind, Found, Source};
use crate::docs::links::{self, Notes, Resolved};

/// The files a Markdown document links to; a link that leads nowhere is
/// unresolved. `None` for a source that is no Markdown document.
pub fn linked(
    notes: &Notes,
    source: &Source,
    by_path: &HashMap<&Path, usize>,
    known: &HashSet<PathBuf>,
) -> Option<Found> {
    if source.language != "Markdown" {
        return None;
    }
    let from = *by_path.get(source.path.as_path())?;
    let mut found = Found::default();
    for link in links::links(&source.text) {
        match notes.resolve(from, &link, |path| known.contains(path)) {
            Resolved::To(to) if to != from => {
                found.add(to, EdgeKind::Load);
            }
            Resolved::Broken => {
                found.unresolved.insert(link.target);
            }
            _ => {}
        }
    }
    Some(found)
}
