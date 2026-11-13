/// Per-author summary computed from git blame across all source files.
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::git::BlameInfo;

/// Aggregated code ownership data for a single author.
pub struct AuthorSummary {
    /// Author display name from git signature.
    pub name: String,
    /// Author email; authors are told apart by [`crate::git::identity`].
    pub email: String,
    /// Number of files where this author is the primary owner.
    pub owned_files: usize,
    /// Total lines attributed to this author across all files.
    pub lines: usize,
    /// Unique languages this author has contributed to, sorted.
    pub languages: Vec<String>,
    /// Unix timestamp of this author's most recent commit.
    pub last_active: i64,
}

/// Accumulator used while walking files before finalizing summaries.
struct AuthorAccum {
    name: String,
    email: String,
    lines: usize,
    languages: BTreeSet<String>,
    last_active: i64,
    owned_files: usize,
}

impl AuthorAccum {
    fn new(blame: &BlameInfo) -> Self {
        Self {
            name: blame.author.clone(),
            email: blame.email.clone(),
            lines: 0,
            languages: BTreeSet::new(),
            last_active: 0,
            owned_files: 0,
        }
    }

    /// Add the lines of one file, which this author may own.
    fn add(&mut self, blame: &BlameInfo, language: &str, owns_file: bool) {
        self.lines += blame.lines;
        self.languages.insert(language.to_string());
        self.last_active = self.last_active.max(blame.last_commit_time);
        self.owned_files += usize::from(owns_file);
    }

    fn summary(self) -> AuthorSummary {
        AuthorSummary {
            name: self.name,
            email: self.email,
            owned_files: self.owned_files,
            lines: self.lines,
            languages: self.languages.into_iter().collect(),
            last_active: self.last_active,
        }
    }
}

/// Aggregate per-author blame data across all files into `AuthorSummary` records.
///
/// For each file, the first author of its blame (most lines, ties as
/// [`crate::git::GitRepo::blame_file`] breaks them) is its primary owner.
/// Authors come by lines, most first, and with as many by identity.
pub fn compute_authors(file_blames: &[(&str, &[BlameInfo])]) -> Vec<AuthorSummary> {
    let mut accum: BTreeMap<String, AuthorAccum> = BTreeMap::new();
    for (language, blames) in file_blames {
        let primary = blames.first().map(BlameInfo::identity);
        for blame in *blames {
            let identity = blame.identity();
            let owns_file = primary.as_ref() == Some(&identity);
            accum
                .entry(identity)
                .or_insert_with(|| AuthorAccum::new(blame))
                .add(blame, language, owns_file);
        }
    }
    let mut result: Vec<AuthorSummary> = accum.into_values().map(AuthorAccum::summary).collect();
    result.sort_by_key(|a| Reverse(a.lines));
    result
}

/// Date each author by their last commit (`last`, by identity) rather than
/// by the newest of their lines that blame still shows: lines get rewritten,
/// and a last commit may touch only files that are not measured.
pub fn set_last_commits(authors: &mut [AuthorSummary], last: &HashMap<String, i64>) {
    for author in authors {
        if let Some(&time) = last.get(&crate::git::identity(&author.name, &author.email)) {
            author.last_active = time;
        }
    }
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
