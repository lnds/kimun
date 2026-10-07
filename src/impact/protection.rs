//! How tests protect a source file.
//!
//! A test rarely names everything it exercises. A controller or a live view
//! is tested through its route, a helper through the views that use it. So
//! protection comes in degrees, from a test that refers to the file down to
//! the tests of what uses it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// How well tests protect a source file, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Protection {
    /// A test refers to it, or sits at the same place in the layout.
    Direct,
    /// A test is named after it, a directory apart: `live/page_live.ex` and
    /// `page_live_test.exs`, or `page_live/index.ex` and `page_live_test.exs`.
    Named,
    /// No test of its own, but a file that uses it has one.
    Users,
    /// No test reaches it.
    None,
}

/// Directories that separate sources from their tests without being part
/// of what a file is: `lib/a/b.ex` and `test/a/b_test.exs` are one place.
const LAYOUT_DIRS: &[&str] = &["lib", "src", "test", "tests", "spec", "__tests__"];

/// What marks a file name as a test, around the name of what it tests.
const TEST_AFFIXES: &[&str] = &["_test", ".test", "_spec", ".spec", "Test", "Spec"];

/// The place of a file in its project, whatever side of the source and test
/// layout it is on: its directories without the layout ones, and its name
/// without extension or test mark. A test mirrors a source when both are at
/// the same place. That is how a test protects code it never names, as a
/// controller test that only issues requests.
pub fn place(path: &Path) -> Vec<String> {
    let mut place: Vec<String> = path
        .parent()
        .into_iter()
        .flat_map(|dir| dir.components())
        .filter_map(|c| c.as_os_str().to_str())
        .filter(|c| !LAYOUT_DIRS.contains(c))
        .map(str::to_string)
        .collect();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let name = TEST_AFFIXES
        .iter()
        .find_map(|affix| stem.strip_suffix(affix))
        .or_else(|| stem.strip_prefix("test_"))
        .unwrap_or(stem);
    place.push(name.to_string());
    place
}

/// Whether `short` is `long` with at most one directory left out.
fn a_directory_apart(short: &[String], long: &[String]) -> bool {
    let mut rest = long.iter();
    long.len() <= short.len() + 1 && short.iter().all(|dir| rest.any(|other| other == dir))
}

/// Whether a test at `test` is named after the source at `source`, both
/// given as places, without being at the same place:
///
/// - the same name, with one directory more on either side;
/// - or the name of the directory the source is in, in the directory above,
///   as the test of a view that has a directory of its own.
fn named_after(test: &[String], source: &[String]) -> bool {
    let (Some((test_name, test_dirs)), Some((name, dirs))) =
        (test.split_last(), source.split_last())
    else {
        return false;
    };
    let beside = test_dirs != dirs
        && (a_directory_apart(test_dirs, dirs) || a_directory_apart(dirs, test_dirs));
    let after_directory = dirs
        .split_last()
        .is_some_and(|(last, above)| last == test_name && a_directory_apart(test_dirs, above));
    (test_name == name && beside) || after_directory
}

/// The tests of each file of a graph, by how they protect it.
pub struct Tests {
    /// Test files that refer to each file, or sit at its place.
    direct: Vec<Vec<usize>>,
    /// Test files named after each file.
    named: Vec<Vec<usize>>,
}

impl Tests {
    /// Sort out the tests of `files`. `referring` holds, for each file, the
    /// tests that refer to it; `is_test` and `is_source` tell what each is.
    pub fn of(
        files: &[PathBuf],
        mut referring: Vec<Vec<usize>>,
        is_test: impl Fn(usize) -> bool,
        is_source: impl Fn(usize) -> bool,
    ) -> Self {
        let places: Vec<Vec<String>> = files.iter().map(|f| place(f)).collect();
        // Tests by the name they carry: the only ones a source can match.
        let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
        for test in (0..files.len()).filter(|&f| is_test(f)) {
            if let Some(name) = places[test].last() {
                by_name.entry(name).or_default().push(test);
            }
        }

        let mut named = vec![Vec::new(); files.len()];
        for source in (0..files.len()).filter(|&f| is_source(f)) {
            let place = &places[source];
            // Its own name, and the name of the directory it is in.
            let candidates = place
                .iter()
                .rev()
                .take(2)
                .filter_map(|name| by_name.get(name.as_str()))
                .flatten();
            for &test in candidates {
                if places[test] == *place {
                    referring[source].push(test);
                } else if named_after(&places[test], place) {
                    named[source].push(test);
                }
            }
            referring[source].sort_unstable();
            referring[source].dedup();
        }
        Self {
            direct: referring,
            named,
        }
    }

    /// Test files that refer to `file` or sit at its place.
    pub fn direct(&self, file: usize) -> &[usize] {
        &self.direct[file]
    }

    /// Test files that protect `file` by themselves: direct and named.
    pub fn own(&self, file: usize) -> impl Iterator<Item = usize> + '_ {
        self.direct[file].iter().chain(&self.named[file]).copied()
    }

    /// How `file` is protected, given the source files that use it.
    pub fn protection(&self, file: usize, users: &[usize]) -> Protection {
        if !self.direct[file].is_empty() {
            Protection::Direct
        } else if !self.named[file].is_empty() {
            Protection::Named
        } else if users.iter().any(|&user| self.own(user).next().is_some()) {
            Protection::Users
        } else {
            Protection::None
        }
    }
}

#[cfg(test)]
#[path = "protection_test.rs"]
mod tests;
