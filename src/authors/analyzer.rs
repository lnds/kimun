/// Per-author summary computed from git blame across all source files.
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

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
    languages: HashSet<String>,
    last_active: i64,
    owned_files: usize,
}

/// Aggregate per-author blame data across all files into `AuthorSummary` records.
///
/// For each file, the author with the most lines is counted as its primary owner.
pub fn compute_authors(file_blames: &[(&str, &[BlameInfo])]) -> Vec<AuthorSummary> {
    let mut accum: HashMap<String, AuthorAccum> = HashMap::new();

    for (language, blames) in file_blames {
        let primary = blames.first().map(BlameInfo::identity);

        for blame in *blames {
            let identity = blame.identity();
            let owns_file = primary.as_ref() == Some(&identity);
            let entry = accum.entry(identity).or_insert_with(|| AuthorAccum {
                name: blame.author.clone(),
                email: blame.email.clone(),
                lines: 0,
                languages: HashSet::new(),
                last_active: 0,
                owned_files: 0,
            });

            entry.lines += blame.lines;
            entry.languages.insert(language.to_string());
            if blame.last_commit_time > entry.last_active {
                entry.last_active = blame.last_commit_time;
            }
            if owns_file {
                entry.owned_files += 1;
            }
        }
    }

    let mut result: Vec<AuthorSummary> = accum
        .into_values()
        .map(|a| {
            let mut langs: Vec<String> = a.languages.into_iter().collect();
            langs.sort();
            AuthorSummary {
                name: a.name,
                email: a.email,
                owned_files: a.owned_files,
                lines: a.lines,
                languages: langs,
                last_active: a.last_active,
            }
        })
        .collect();

    result.sort_by_key(|r| Reverse(r.lines));
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
