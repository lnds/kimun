//! Files that are run rather than used: command-line tasks and scripts.
//!
//! Nothing in the project uses them, so no test of something else passes
//! through them, and few have a test of their own. Warning that an
//! integration test is missing for each would bury the warnings that matter.

use std::error::Error;
use std::path::Path;

use globset::GlobSet;

/// Directories that hold what is run from the command line, by the
/// convention of each ecosystem, as the directories that name them.
const TASK_DIRS: &[&[&str]] = &[
    &["mix", "tasks"],
    &["management", "commands"],
    &["bin"],
    &["scripts"],
    &["examples"],
    &["benches"],
];

/// Files that are the start of a program wherever they are, as the end of
/// their path: the binary of a Rust package and its build script.
const PROGRAMS: &[&str] = &["src/main.rs", "build.rs"];

/// Tells the entry points among the files of a repository.
pub struct EntryPoints {
    configured: GlobSet,
}

impl EntryPoints {
    /// The conventional ones, plus the paths matching `patterns`: globs
    /// relative to the repository, as `[impact] entry_points` lists them.
    pub fn new(patterns: &[String]) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            configured: super::inert::globs(patterns, "[impact] entry_points")?,
        })
    }

    /// Whether `path` is where its ecosystem keeps what is run. A file
    /// there that other code uses is not an entry point for that alone.
    pub fn conventional(&self, path: &Path) -> bool {
        let dirs: Vec<&str> = path
            .parent()
            .into_iter()
            .flat_map(|dir| dir.components())
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        let in_tasks = TASK_DIRS
            .iter()
            .any(|task| dirs.windows(task.len()).any(|window| window == *task));
        in_tasks || PROGRAMS.iter().any(|program| path.ends_with(program))
    }

    /// Whether the repository declares `path` an entry point.
    pub fn configured(&self, path: &Path) -> bool {
        self.configured.is_match(path)
    }
}

#[cfg(test)]
#[path = "entry_test.rs"]
mod tests;
