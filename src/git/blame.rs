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

use git2::BlameOptions;

use super::{BlameHunkInfo, BlameInfo, GitRepo, identity, name_and_email};

/// The file `git blame` reads revisions to ignore from, by convention.
const IGNORE_REVS_FILE: &str = ".git-blame-ignore-revs";

/// A run of lines last changed by one author at one time.
#[derive(Debug, PartialEq)]
struct Hunk {
    author: String,
    email: String,
    time: i64,
    /// First line of the run (1-based).
    start_line: usize,
    lines: usize,
}

impl GitRepo {
    /// Run git blame on a file and return per-author contributions.
    /// `rel_path` is relative to the git root.
    pub fn blame_file(&self, rel_path: &Path) -> Result<Vec<BlameInfo>, Box<dyn Error>> {
        let mut map: HashMap<String, BlameInfo> = HashMap::new();
        for hunk in self.hunks(rel_path)? {
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
        let mut result: Vec<BlameInfo> = map.into_values().collect();
        result.sort_by_key(|r| Reverse(r.lines));
        Ok(result)
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
        match self.blame_with("git", rel_path) {
            Some(hunks) => Ok(hunks),
            None => self.blame_with_libgit2(rel_path),
        }
    }

    /// The blame of a file through `program` (`git`); `None` when it cannot
    /// run or fails, such as when the program is missing.
    fn blame_with(&self, program: &str, rel_path: &Path) -> Option<Vec<Hunk>> {
        let mut command = Command::new(program);
        command
            .arg("-C")
            .arg(&self.root)
            .args(["blame", "--line-porcelain", "-w", "-M", "-C"]);
        if self.root.join(IGNORE_REVS_FILE).is_file() {
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
                Hunk {
                    author,
                    email,
                    time: sig.map_or(0, |s| s.when().seconds()),
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
        start_line: 0,
        lines: 1,
    };
    for row in text.lines() {
        if row.starts_with('\t') {
            push_line(&mut hunks, &line);
        } else if let Some(mail) = row.strip_prefix("author-mail ") {
            line.email = mail
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string();
        } else if let Some(time) = row.strip_prefix("author-time ") {
            line.time = time.trim().parse().unwrap_or(0);
        } else if let Some(name) = row.strip_prefix("author ") {
            line.author = name.to_string();
        } else if let Some(number) = final_line(row) {
            line.start_line = number;
        }
    }
    hunks
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
