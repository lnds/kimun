//! Changed files that reach nothing.
//!
//! A change to documentation breaks no build and no test. Counting it as a
//! change to its project would reach every dependent, and outside every
//! project it would make the reach unknown — in a repository with a
//! knowledge base, for most changes.

use std::error::Error;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

/// Extensions of documentation, inert whatever the configuration says.
const DOCUMENTATION: [&str; 6] = ["md", "mdx", "markdown", "rst", "adoc", "txt"];

/// Tells the changed files that reach nothing.
pub struct Inert {
    configured: GlobSet,
}

impl Inert {
    /// Documentation, plus the paths matching `patterns`: globs relative to
    /// the repository, as `[impact] inert` of `.kimun.toml` lists them.
    pub fn new(patterns: &[String]) -> Result<Self, Box<dyn Error>> {
        let mut globs = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(pattern)
                .map_err(|e| format!("invalid pattern in [impact] inert: {e}"))?;
            globs.add(glob);
        }
        Ok(Self {
            configured: globs.build()?,
        })
    }

    pub fn matches(&self, path: &Path) -> bool {
        let documentation = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| DOCUMENTATION.contains(&e.to_ascii_lowercase().as_str()));
        documentation || self.configured.is_match(path)
    }
}

#[cfg(test)]
#[path = "inert_test.rs"]
mod tests;
