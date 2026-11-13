//! Blame: who last changed each line of a file.
//!
//! `git blame` is run when the `git` program is there: it skips changes of
//! whitespace (`-w`), follows lines moved within a file or from the files
//! the same commit changed (`-M`, `-C`), and honors `.git-blame-ignore-revs`
//! and `blame.ignoreRevsFile`. libgit2, the fallback, skips whitespace but
//! can do none of the rest. Both blame the file as committed at `HEAD`.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::process::Command;
use std::sync::Once;

use git2::BlameOptions;

use super::{BlameHunkInfo, BlameInfo, GitRepo, identity, name_and_email};

/// The file `git blame` reads revisions to ignore from, by convention.
const IGNORE_REVS_FILE: &str = ".git-blame-ignore-revs";

/// Said once, when `git blame` rejects the revisions to ignore.
static IGNORE_REVS_REJECTED: Once = Once::new();

/// The lines of each author, and when the newest of them was written,
/// ordered as [`GitRepo::blame_file`] says.
fn per_author(hunks: impl Iterator<Item = Hunk>) -> Vec<BlameInfo> {
    let mut map: HashMap<String, BlameInfo> = HashMap::new();
    for hunk in hunks {
        let info = map
            .entry(identity(&hunk.author, &hunk.email))
            .or_insert_with(|| BlameInfo {
                author: hunk.author.clone(),
                email: hunk.email.clone(),
                lines: 0,
                last_commit_time: hunk.time,
            });
        info.lines += hunk.lines;
        info.last_commit_time = info.last_commit_time.max(hunk.time);
    }
    let mut authors: Vec<(String, BlameInfo)> = map.into_iter().collect();
    authors.sort_by_key(owner_rank);
    authors.into_iter().map(|(_, info)| info).collect()
}

/// Most lines first, then the most recent, then the identity.
fn owner_rank((identity, info): &(String, BlameInfo)) -> (Reverse<usize>, Reverse<i64>, String) {
    (
        Reverse(info.lines),
        Reverse(info.last_commit_time),
        identity.clone(),
    )
}

/// A run of lines last changed by one author at one time.
#[derive(Debug, PartialEq)]
struct Hunk {
    author: String,
    email: String,
    /// When the lines were written (author date).
    time: i64,
    /// When they were committed.
    committed: i64,
    /// First line of the run (1-based).
    start_line: usize,
    lines: usize,
}

impl GitRepo {
    /// Run git blame on a file and return per-author contributions.
    /// `rel_path` is relative to the git root.
    ///
    /// Authors come by lines, most first; a tie goes to the author of the
    /// most recent of those lines, then to the identity first in
    /// alphabetical order, so the first one is the file's owner on every run.
    pub fn blame_file(&self, rel_path: &Path) -> Result<Vec<BlameInfo>, Box<dyn Error>> {
        self.blame_file_since(rel_path, None)
    }

    /// Like [`Self::blame_file`], counting only the lines committed since
    /// `since`, by commit date as the history walks read it.
    pub fn blame_file_since(
        &self,
        rel_path: &Path,
        since: Option<i64>,
    ) -> Result<Vec<BlameInfo>, Box<dyn Error>> {
        let hunks = self.hunks(rel_path)?.into_iter();
        Ok(per_author(
            hunks.filter(|h| since.is_none_or(|ts| h.committed >= ts)),
        ))
    }

    /// Run git blame on a file and return per-hunk data preserving line ranges.
    /// Used to join blame attribution with per-line FSM classification.
    pub fn blame_hunks(&self, rel_path: &Path) -> Result<Vec<BlameHunkInfo>, Box<dyn Error>> {
        let hunks = self.hunks(rel_path)?;
        Ok(hunks
            .into_iter()
            .map(|h| BlameHunkInfo {
                author: h.author,
                email: h.email,
                start_line: h.start_line,
                lines: h.lines,
            })
            .collect())
    }

    /// The blame of a file, through `git blame` or, failing that, libgit2.
    fn hunks(&self, rel_path: &Path) -> Result<Vec<Hunk>, Box<dyn Error>> {
        self.warn_if_shallow();
        match self.blame_with("git", rel_path) {
            Some(hunks) => Ok(hunks),
            None => self.blame_with_libgit2(rel_path),
        }
    }

    /// The blame of a file through `program` (`git`); `None` when it cannot
    /// run or fails, such as when the program is missing. When it fails
    /// only with the revisions to ignore, which `git blame` rejects whole
    /// for one bad line, it runs again without them, and says so once.
    fn blame_with(&self, program: &str, rel_path: &Path) -> Option<Vec<Hunk>> {
        let ignore_revs = self.root.join(IGNORE_REVS_FILE).is_file();
        if let Some(hunks) = self.run_blame(program, rel_path, ignore_revs) {
            return Some(hunks);
        }
        if !ignore_revs {
            return None;
        }
        let hunks = self.run_blame(program, rel_path, false)?;
        IGNORE_REVS_REJECTED.call_once(|| {
            eprintln!("warning: git blame cannot read {IGNORE_REVS_FILE}; blaming without it")
        });
        Some(hunks)
    }

    /// One run of `program blame`, with the revisions to ignore or without.
    fn run_blame(&self, program: &str, rel_path: &Path, ignore_revs: bool) -> Option<Vec<Hunk>> {
        let mut command = Command::new(program);
        command
            .arg("-C")
            .arg(&self.root)
            .args(["blame", "--line-porcelain", "-w", "-M", "-C"]);
        if ignore_revs {
            command.args(["--ignore-revs-file", IGNORE_REVS_FILE]);
        }
        command.arg("HEAD").arg("--").arg(rel_path);
        let output = command.output().ok()?;
        output
            .status
            .success()
            .then(|| parse_porcelain(&String::from_utf8_lossy(&output.stdout)))
    }

    /// The blame of a file through libgit2, with authors read through
    /// `.mailmap` and changes of whitespace skipped.
    fn blame_with_libgit2(&self, rel_path: &Path) -> Result<Vec<Hunk>, Box<dyn Error>> {
        let mut opts = BlameOptions::new();
        opts.use_mailmap(true).ignore_whitespace(true);
        let blame = self.repo.blame_file(rel_path, Some(&mut opts))?;
        Ok(blame
            .iter()
            .map(|hunk| {
                let sig = hunk.final_signature();
                let (author, email) = name_and_email(sig.as_ref());
                let time = sig.map_or(0, |s| s.when().seconds());
                Hunk {
                    author,
                    email,
                    time,
                    committed: hunk.final_committer().map_or(time, |s| s.when().seconds()),
                    start_line: hunk.final_start_line(),
                    lines: hunk.lines_in_hunk(),
                }
            })
            .collect())
    }
}

/// The runs of lines of `git blame --line-porcelain`. Each line comes
/// after a header (`<sha> <orig> <final> [<count>]`) and the fields of its
/// commit (`author`, `author-mail`, `author-time`), and starts with a tab.
fn parse_porcelain(text: &str) -> Vec<Hunk> {
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut line = Hunk {
        author: String::new(),
        email: String::new(),
        time: 0,
        committed: 0,
        start_line: 0,
        lines: 1,
    };
    for row in text.lines() {
        match row.starts_with('\t') {
            true => push_line(&mut hunks, &line),
            false => line.read(row),
        }
    }
    hunks
}

impl Hunk {
    /// Take what a row of the porcelain, other than a line of the file, says.
    fn read(&mut self, row: &str) {
        let (key, value) = row.split_once(' ').unwrap_or((row, ""));
        let time = || value.trim().parse().unwrap_or(0);
        match key {
            "author" => self.author = value.to_string(),
            "author-mail" => {
                self.email = value
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string()
            }
            "author-time" => self.time = time(),
            "committer-time" => self.committed = time(),
            _ => self.start_line = final_line(row).unwrap_or(self.start_line),
        }
    }
}

/// The final line number of a header row: its first word is a commit id.
fn final_line(row: &str) -> Option<usize> {
    let mut words = row.split(' ');
    let id = words.next()?;
    if id.len() < 40 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    words.nth(1)?.parse().ok()
}

/// Add one line to the runs, joining it to the last one when it follows it
/// and has the same author and time.
fn push_line(hunks: &mut Vec<Hunk>, line: &Hunk) {
    if let Some(last) = hunks.last_mut()
        && last.author == line.author
        && last.email == line.email
        && last.time == line.time
        && last.committed == line.committed
        && last.start_line + last.lines == line.start_line
    {
        last.lines += 1;
        return;
    }
    hunks.push(Hunk {
        author: line.author.clone(),
        email: line.email.clone(),
        ..*line
    });
}

#[cfg(test)]
#[path = "blame_test.rs"]
mod tests;
