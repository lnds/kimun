//! The Ruby files of a graph and what each uses: the files it requires and
//! those that define the constants it names.
//!
//! `require_relative` resolves from the requiring file. `require`, `load`
//! and `autoload` look the path up in the load path, approximated by the
//! directories of a project a path is required from: `lib`, `test`,
//! `spec`, and the autoload roots of Rails (`app/*`, `app/*/concerns`);
//! what resolves nowhere is a gem or the standard library.
//!
//! Constants are looked up always, whatever loads the files (`require`,
//! Zeitwerk, Bundler): once loaded, a constant is global. A reference is
//! looked up whole from the namespaces it is written in, innermost first.
//! When several files define a constant, the one named after it as
//! Zeitwerk names files (`Shop::LineItem` in `shop/line_item.rb`) is the
//! definition and the others reopen it; when none is, the nearest to the
//! reference count, and a reference is ambiguous when they are spread over
//! directories with none nearer. A constant no file defines is found by
//! its path under the autoload roots, for files that define it by
//! metaprogramming. The core classes of Ruby (`String`, `Hash`) are never
//! a use, even when a file reopens them.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

use super::graph::Source;
use super::ruby::{Reference, Require, RubyFile, qualified, read};
use super::types::nearest_all;
use crate::walk::{is_test_dir, is_test_file};

/// The classes and modules of Ruby itself, which a file may reopen.
const CORE: &[&str] = &[
    "Array",
    "BasicObject",
    "Class",
    "Comparable",
    "Data",
    "Enumerable",
    "Exception",
    "FalseClass",
    "Float",
    "Hash",
    "Integer",
    "Kernel",
    "Module",
    "NilClass",
    "Numeric",
    "Object",
    "Proc",
    "Range",
    "Regexp",
    "StandardError",
    "String",
    "Struct",
    "Symbol",
    "Time",
    "TrueClass",
];

/// The directories a path is required or autoloaded from.
const ROOTS: &[&str] = &["lib", "test", "spec"];

/// `Shop::LineItem` as Zeitwerk names its file: `shop/line_item`.
pub fn underscore(constant: &str) -> String {
    constant
        .split("::")
        .map(underscore_one)
        .collect::<Vec<_>>()
        .join("/")
}

/// `LineItem` → `line_item`, `HTTPClient` → `http_client`.
fn underscore_one(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|p| chars[p]);
        let next = chars.get(i + 1);
        let boundary = c.is_ascii_uppercase()
            && prev.is_some_and(|p| {
                p.is_ascii_lowercase()
                    || p.is_ascii_digit()
                    || (p.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase()))
            });
        if boundary {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// The paths `path` may be required by, without `.rb`: relative to each
/// root above it.
fn loadable(path: &Path) -> Vec<String> {
    let parts: Vec<&str> = path.iter().filter_map(|c| c.to_str()).collect();
    let Some((file, dirs)) = parts.split_last() else {
        return Vec::new();
    };
    let Some(stem) = file.strip_suffix(".rb") else {
        return Vec::new();
    };
    let rest = |from: usize| {
        let mut key = dirs[from..].join("/");
        if !key.is_empty() {
            key.push('/');
        }
        key + stem
    };
    let mut keys = Vec::new();
    for (i, dir) in dirs.iter().enumerate() {
        if ROOTS.contains(dir) {
            keys.push(rest(i + 1));
        }
        if *dir == "app" && i + 1 < dirs.len() {
            keys.push(rest(i + 2));
            if dirs.get(i + 2) == Some(&"concerns") {
                keys.push(rest(i + 3));
            }
        }
    }
    keys
}

/// `a/./b/../c` as `a/c`.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// The Ruby files of a graph, read, and the constants they define.
pub struct Files {
    paths: Vec<PathBuf>,
    files: Vec<Option<RubyFile>>,
    by_path: HashMap<PathBuf, usize>,
    /// The files that define each constant, qualified.
    constants: HashMap<String, Vec<usize>>,
    /// The files under each path a `require` or the autoloader names.
    loadable: HashMap<String, Vec<usize>>,
    ambiguous: Cell<usize>,
    /// Whether each file is a test or sits among tests.
    tests: Vec<bool>,
}

/// Whether `path` is a test or sits among tests: shipping code never
/// loads it.
fn is_test(path: &Path) -> bool {
    let in_test_dir = path
        .parent()
        .into_iter()
        .flat_map(Path::iter)
        .filter_map(|c| c.to_str())
        .any(is_test_dir);
    in_test_dir || is_test_file(path)
}

impl Files {
    pub fn of(sources: &[Source]) -> Self {
        let files: Vec<Option<RubyFile>> = sources
            .iter()
            .map(|s| (s.language == "Ruby").then(|| read(&s.text)))
            .collect();
        let paths: Vec<PathBuf> = sources.iter().map(|s| s.path.clone()).collect();
        let mut constants: HashMap<String, Vec<usize>> = HashMap::new();
        let mut loadable_files: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_path = HashMap::new();
        for (at, file) in files.iter().enumerate() {
            let Some(file) = file else { continue };
            by_path.insert(paths[at].clone(), at);
            let defined: BTreeSet<&String> = file.defines.iter().collect();
            for constant in defined {
                constants.entry(constant.clone()).or_default().push(at);
            }
            for key in loadable(&paths[at]) {
                loadable_files.entry(key).or_default().push(at);
            }
        }
        Files {
            files,
            by_path,
            constants,
            loadable: loadable_files,
            ambiguous: Cell::new(0),
            tests: paths.iter().map(|p| is_test(p)).collect(),
            paths,
        }
    }

    /// The files the Ruby file at `at` uses. `None` when it is not Ruby.
    pub fn uses(&self, at: usize) -> Option<BTreeMap<usize, BTreeSet<String>>> {
        let file = self.files.get(at)?.as_ref()?;
        let from = &self.paths[at];
        let mut used: BTreeSet<usize> = file
            .requires
            .iter()
            .flat_map(|r| self.required(r, from))
            .collect();
        let mut ambiguous = 0;
        for reference in &file.refs {
            match self.defining(reference, from) {
                Ok(files) => used.extend(files),
                Err(()) => ambiguous += 1,
            }
        }
        self.ambiguous.set(self.ambiguous.get() + ambiguous);
        Some(used.into_iter().map(|to| (to, BTreeSet::new())).collect())
    }

    /// References left out so far because several files define the
    /// constant and none of them is nearer than the others.
    pub fn ambiguous(&self) -> usize {
        self.ambiguous.get()
    }

    fn required(&self, require: &Require, from: &Path) -> Vec<usize> {
        match require {
            Require::Relative(path) => {
                let dir = from.parent().unwrap_or(Path::new(""));
                let mut target = normalize(&dir.join(path));
                if target.extension().is_none_or(|e| e != "rb") {
                    target.as_mut_os_string().push(".rb");
                }
                self.by_path.get(&target).copied().into_iter().collect()
            }
            Require::LoadPath(path) => {
                let key = path.strip_suffix(".rb").unwrap_or(path);
                self.nearest(self.loadable.get(key), from)
            }
        }
    }

    /// The file a path names for `from`: the nearest, and of several as
    /// near, the shallowest, as the load path of the repository's own
    /// project comes first.
    fn nearest(&self, files: Option<&Vec<usize>>, from: &Path) -> Vec<usize> {
        let seen = self.seen(files.map_or(&[], Vec::as_slice), from);
        let nearest = nearest_all(&seen, from, &self.paths);
        let depth = |f: &usize| self.paths[*f].components().count();
        let shallowest = nearest.iter().map(depth).min().unwrap_or(0);
        nearest
            .into_iter()
            .filter(|f| depth(f) == shallowest)
            .collect()
    }

    /// The files that define the constant `reference` names, looked up
    /// from its namespaces, innermost first. A constant must be found
    /// whole: a namespace of a gem that the repository reopens would
    /// otherwise take every reference to the constants of the gem.
    fn defining(&self, reference: &Reference, from: &Path) -> Result<Vec<usize>, ()> {
        let chain = &reference.chain;
        let written = chain.join("::");
        let core = CORE.contains(&chain[0].as_str());
        let names: Vec<String> = reference
            .scopes
            .iter()
            .filter(|scope| !scope.is_empty() || !core)
            .map(|scope| qualified(scope, &written))
            .collect();
        if let Some(found) = names.iter().find_map(|n| self.defined(n, from)) {
            return found;
        }
        let autoloaded = names.iter().find_map(|n| self.loadable.get(&underscore(n)));
        Ok(self.nearest(autoloaded, from))
    }

    /// The files that define `constant` for a reference from `from`: the
    /// one named after it; or else the nearest, all of them when they sit
    /// in one directory, as the parts of a class split among files do.
    /// `Err` when they are spread and none is nearer.
    fn defined(&self, constant: &str, from: &Path) -> Option<Result<Vec<usize>, ()>> {
        let files = self.seen(self.constants.get(constant)?, from);
        if files.is_empty() {
            return None;
        }
        let named = underscore(constant);
        let owners: Vec<usize> = files
            .iter()
            .copied()
            .filter(|&f| self.ends_with(f, &named))
            .collect();
        let pool = if owners.is_empty() { &files } else { &owners };
        let nearest = nearest_all(pool, from, &self.paths);
        let dir = |f: &usize| self.paths[*f].parent();
        let together = nearest.iter().all(|f| dir(f) == dir(&nearest[0]));
        Some(if together { Ok(nearest) } else { Err(()) })
    }

    /// Those of `files` a file at `from` may load: tests only from tests.
    fn seen(&self, files: &[usize], from: &Path) -> Vec<usize> {
        let from_test = is_test(from);
        let seen = files.iter().copied();
        seen.filter(|&f| from_test || !self.tests[f]).collect()
    }

    /// Whether the path of `file`, without `.rb`, ends with `named`.
    fn ends_with(&self, file: usize, named: &str) -> bool {
        let path = self.paths[file].with_extension("");
        path.ends_with(named)
    }
}

#[cfg(test)]
#[path = "ruby_files_test.rs"]
mod tests;
