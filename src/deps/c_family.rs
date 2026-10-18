//! C and C++, where a file uses the headers it includes.
//!
//! An include is resolved as a compiler would, without the include
//! directories of the build, which kimun does not read: a quoted one from
//! the directory of the includer first; then from each directory above it
//! and the `include` directory there; then by the path it names, wherever
//! it is in the repository, when one file nearest to the includer has it.
//! A header of the system (`<stdio.h>`) names no file of the repository.
//!
//! Including a header uses what it declares, and so what its source
//! defines: an includer also uses, implied, the sources beside the header
//! with its name (`foo.h` and `foo.c`, or `include/x/foo.h` and
//! `src/x/foo.c`), so that a change to a source reaches the users of its
//! header. The code does not write that use, so it makes no cycle.
//!
//! A quoted include that names no file of the repository is noted as
//! unresolved, unless it names a header of the system: it is often one the
//! build generates.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::graph::{EdgeKind, Found, Source, Use, nearest, shared_depth};
use super::javascript::normalize;

/// An `#include` and how it names its file.
#[derive(Debug, PartialEq)]
pub struct Include {
    pub path: String,
    /// `"…"` rather than `<…>`.
    pub quoted: bool,
}

/// Whether a language is read here.
fn is_c_family(language: &str) -> bool {
    matches!(language, "C" | "C++")
}

const HEADERS: &[&str] = &["h", "hh", "hpp", "hxx", "h++"];
const SOURCES: &[&str] = &["c", "cc", "cpp", "cxx", "c++"];

/// Headers of the C library and of POSIX, which a quoted include may name
/// too: the compiler finds them among those of the system.
const SYSTEM: &[&str] = &[
    "assert.h",
    "complex.h",
    "ctype.h",
    "errno.h",
    "fenv.h",
    "float.h",
    "inttypes.h",
    "iso646.h",
    "limits.h",
    "locale.h",
    "math.h",
    "setjmp.h",
    "signal.h",
    "stdalign.h",
    "stdarg.h",
    "stdatomic.h",
    "stdbool.h",
    "stddef.h",
    "stdint.h",
    "stdio.h",
    "stdlib.h",
    "stdnoreturn.h",
    "string.h",
    "tgmath.h",
    "threads.h",
    "time.h",
    "uchar.h",
    "wchar.h",
    "wctype.h",
    "unistd.h",
    "fcntl.h",
    "pthread.h",
    "dirent.h",
    "dlfcn.h",
    "strings.h",
    "windows.h",
];

/// Whether an include that names no file names one of the project, by its
/// form: quoted, and not a header of the system.
fn is_internal(include: &Include) -> bool {
    let path = include.path.as_str();
    include.quoted && !SYSTEM.contains(&path) && !path.starts_with("sys/")
}

/// The includes of a source, outside comments, in order.
pub fn includes(source: &str) -> Vec<Include> {
    let mut found = Vec::new();
    let mut in_comment = false;
    for line in source.lines() {
        let code = uncommented(line, &mut in_comment);
        found.extend(include_of(&code));
    }
    found
}

/// `line` without its comments, `in_comment` telling whether a block
/// comment is open before it and after it.
fn uncommented(line: &str, in_comment: &mut bool) -> String {
    let mut code = String::new();
    let mut rest = line;
    loop {
        if *in_comment {
            match rest.find("*/") {
                Some(end) => {
                    rest = &rest[end + 2..];
                    *in_comment = false;
                }
                None => return code,
            }
        }
        let line_comment = rest.find("//");
        match rest.find("/*") {
            Some(open) if line_comment.is_none_or(|lc| open < lc) => {
                code.push_str(&rest[..open]);
                code.push(' ');
                rest = &rest[open + 2..];
                *in_comment = true;
            }
            _ => {
                code.push_str(&rest[..line_comment.unwrap_or(rest.len())]);
                return code;
            }
        }
    }
}

/// The include a line of code makes, if it makes one.
fn include_of(code: &str) -> Option<Include> {
    let directive = code.trim_start().strip_prefix('#')?.trim_start();
    let rest = ["include_next", "include", "import"]
        .iter()
        .find_map(|word| directive.strip_prefix(word))?
        .trim_start();
    let (close, quoted) = match rest.chars().next()? {
        '"' => ('"', true),
        '<' => ('>', false),
        _ => return None,
    };
    let inner = &rest[1..];
    let path = &inner[..inner.find(close)?];
    (!path.is_empty()).then(|| Include {
        path: path.to_string(),
        quoted,
    })
}

/// The C and C++ files of a graph, by path and by file name.
pub struct Files<'s> {
    by_path: HashMap<&'s Path, usize>,
    by_name: HashMap<&'s str, Vec<usize>>,
    paths: Vec<&'s Path>,
}

impl<'s> Files<'s> {
    pub fn of(sources: &'s [Source]) -> Self {
        let mut files = Files {
            by_path: HashMap::new(),
            by_name: HashMap::new(),
            paths: sources.iter().map(|s| s.path.as_path()).collect(),
        };
        for (at, source) in sources.iter().enumerate() {
            if !is_c_family(&source.language) {
                continue;
            }
            files.by_path.insert(&source.path, at);
            if let Some(name) = source.path.file_name().and_then(|n| n.to_str()) {
                files.by_name.entry(name).or_default().push(at);
            }
        }
        files
    }

    fn at(&self, path: &Path) -> Option<usize> {
        self.by_path.get(normalize(path)?.as_path()).copied()
    }

    /// The files `source` includes, and the quoted includes that name no
    /// file. `None` when it is neither C nor C++.
    pub fn uses(&self, source: &Source) -> Option<Found> {
        if !is_c_family(&source.language) {
            return None;
        }
        let mut found = Found::default();
        for include in includes(&source.text) {
            match self.resolve(&source.path, &include) {
                Some(file) => {
                    found.add(file, EdgeKind::Load);
                }
                None if is_internal(&include) => {
                    found.unresolved.insert(include.path);
                }
                None => {}
            }
        }
        Some(found)
    }

    /// The sources behind the headers `source` includes, given as the uses
    /// it was found to make.
    pub fn implied(&self, source: &Source, uses: &[Use]) -> Vec<usize> {
        if !is_c_family(&source.language) {
            return Vec::new();
        }
        uses.iter()
            .flat_map(|u| self.companions(self.paths[u.to]))
            .collect()
    }

    /// The file an include names, from `importer`.
    fn resolve(&self, importer: &Path, include: &Include) -> Option<usize> {
        let dir = importer.parent().unwrap_or(Path::new(""));
        let named = Path::new(&include.path);
        let beside = include.quoted.then(|| dir.join(named));
        let above = dir
            .ancestors()
            .flat_map(|a| [a.join(named), a.join("include").join(named)]);
        beside
            .into_iter()
            .chain(above)
            .find_map(|candidate| self.at(&candidate))
            .or_else(|| self.anywhere(importer, include))
    }

    /// The file whose path ends with the one an include names, nearest to
    /// the importer, when one is. A bare name in angle brackets is left to
    /// the system.
    fn anywhere(&self, importer: &Path, include: &Include) -> Option<usize> {
        let named = Path::new(&include.path);
        if !include.quoted && named.components().count() < 2 {
            return None;
        }
        let name = named.file_name()?.to_str()?;
        let candidates: Vec<usize> = self
            .by_name
            .get(name)?
            .iter()
            .copied()
            .filter(|&c| self.paths[c].ends_with(named))
            .collect();
        nearest(&candidates, |c| shared_depth(self.paths[c], importer)).ok()
    }

    /// The sources that define what a header declares: those of the same
    /// name beside it, or under `src` where it is under `include`.
    fn companions(&self, header: &Path) -> Vec<usize> {
        let is_header = header
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| HEADERS.contains(&e));
        if !is_header {
            return Vec::new();
        }
        let mirrored: PathBuf = header
            .components()
            .map(|c| match c.as_os_str().to_str() {
                Some("include") => std::ffi::OsStr::new("src"),
                _ => c.as_os_str(),
            })
            .collect();
        [header.to_path_buf(), mirrored]
            .iter()
            .flat_map(|base| SOURCES.iter().map(move |ext| base.with_extension(ext)))
            .filter_map(|candidate| self.at(&candidate))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
#[path = "c_family_test.rs"]
mod tests;
