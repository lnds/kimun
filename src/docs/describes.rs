//! What a document describes: the code it says it covers (`covers:` in its
//! front matter, `[docs] covers` in `.kimun.toml`) and the paths it names.

use std::error::Error;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::markdown::{self, Form};
use super::refs::{decode, is_external, path_in, relative};
use crate::config::DocsConfig;

/// Documents that record history as it was and never follow the code.
const RECORDS: [&str; 3] = ["changelog", "changes", "history"];

/// What tells which documents describe which files.
pub struct Rules {
    /// Code globs and the documents that cover them, from `[docs] covers`.
    covers: Vec<(String, GlobSet)>,
    ignore: GlobSet,
}

impl Rules {
    pub fn new(config: &DocsConfig) -> Result<Self, Box<dyn Error>> {
        let covers = config
            .covers
            .iter()
            .map(|(code, documents)| Ok((code.clone(), globs(documents.as_slice())?)))
            .collect::<Result<_, Box<dyn Error>>>()?;
        Ok(Self {
            covers,
            ignore: globs(&config.ignore)?,
        })
    }

    /// Whether a document never needs to follow the code: a changelog, or
    /// a path of `[docs] ignore`.
    pub fn ignores(&self, document: &Path) -> bool {
        let stem = document
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        RECORDS.contains(&stem.as_str()) || self.ignore.is_match(document)
    }

    /// The code globs `document` covers, by its front matter and by
    /// `[docs] covers`.
    pub fn covered_by(&self, document: &Path, text: &str) -> Vec<String> {
        let mut covered = declared_in(text);
        covered.extend(
            self.covers
                .iter()
                .filter(|(_, documents)| documents.is_match(document))
                .map(|(code, _)| code.clone()),
        );
        covered
    }
}

/// The set of `patterns`, each also as a directory: `lib/billing` covers
/// what is under it.
pub fn globs(patterns: &[String]) -> Result<GlobSet, Box<dyn Error>> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let trimmed = pattern.trim_end_matches('/');
        for glob in [trimmed.to_string(), format!("{trimmed}/**")] {
            builder.add(Glob::new(&glob).map_err(|e| format!("invalid pattern {pattern}: {e}"))?);
        }
    }
    Ok(builder.build()?)
}

/// The globs of `covers:` in a document's front matter: a list in
/// brackets, a block list, or a single value.
fn declared_in(text: &str) -> Vec<String> {
    let unquote = |s: &str| s.trim().trim_matches(['"', '\'']).to_string();
    let mut globs = Vec::new();
    let mut in_list = false;
    for line in markdown::front_matter(text) {
        if let Some(value) = line.strip_prefix("covers:").map(str::trim) {
            in_list = value.is_empty();
            match value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                Some(items) => globs.extend(items.split(',').map(unquote)),
                None if !value.is_empty() => globs.push(unquote(value)),
                None => {}
            }
        } else if in_list {
            match line.trim_start().strip_prefix("- ") {
                Some(item) => globs.push(unquote(item)),
                None => in_list = line.trim().is_empty(),
            }
        }
    }
    globs.retain(|glob| !glob.is_empty());
    globs
}

/// The path a reference names, relative to the root, and whether it must
/// be named whole: a link leads to one path, while a path in a code span
/// may be the end of a longer one (`deps/graph.rs`).
pub fn names_of(document: &Path, reference: &markdown::Reference) -> Option<(PathBuf, bool)> {
    let dir = document.parent().unwrap_or(Path::new(""));
    let from_document = |path: &str| relative(&dir.join(path), Path::new(""));
    match reference.form {
        Form::Link | Form::Image => {
            let target = decode(&reference.target);
            if is_external(&target) {
                return None;
            }
            let path = target.split(['#', '?']).next().filter(|p| !p.is_empty())?;
            Some(match path.strip_prefix('/') {
                Some(rooted) => (relative(Path::new(rooted), Path::new("")), true),
                None => (from_document(path), true),
            })
        }
        Form::Code => {
            let path = path_in(&reference.target)?;
            Some(match path.starts_with("./") || path.starts_with("../") {
                true => (from_document(path), true),
                false => (PathBuf::from(path.trim_end_matches('/')), false),
            })
        }
    }
}

/// Whether a reference to `name` names `changed`: the same path, the end
/// of it for a path in code, or a directory of two or more names above it
/// (a top directory such as `src/` holds too much to describe a file).
pub fn names(name: &Path, whole: bool, changed: &Path) -> bool {
    name == changed
        || (!whole && changed.ends_with(name))
        || (name.components().count() >= 2 && changed.starts_with(name))
}

/// Whether a path is a document whose references can be read.
pub fn is_document(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "md" | "mdx" | "markdown"))
}

#[cfg(test)]
#[path = "describes_test.rs"]
mod tests;
