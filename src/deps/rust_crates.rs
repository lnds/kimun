//! Where the paths of Rust files lead: the module tree of each crate, and
//! the file behind a path.
//!
//! A crate starts at a root file (`src/lib.rs`, `src/main.rs`, a file of
//! `src/bin`, `tests`, `examples` or `benches`, `build.rs`) and grows by
//! `mod` declarations. A path is made absolute from the module it is
//! written in and leads to the file of the deepest module it names: the
//! item itself may be defined there or only re-exported, which a lexical
//! reading cannot tell apart.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Component, Path, PathBuf};

use super::rust::{ModDecl, PathRef, RustFile};

/// A module: the crate it belongs to, by its root, and its path there.
type ModuleId = (usize, Vec<String>);

/// Directories whose files are each the root of a crate of their own.
const TARGET_DIRS: &[&str] = &["tests", "examples", "benches"];

fn name_of(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

/// `path` without `.` and with each `..` undone.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// The directory of the package `path` is the root of a crate in, if it
/// is one. `has` tells whether a file is among those read.
fn package_of(path: &Path, has: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    let dir = path.parent()?;
    let above = dir.parent().unwrap_or(Path::new(""));
    let in_src = |dir: &Path| name_of(dir) == "src";
    match name_of(path) {
        "lib.rs" | "main.rs" if in_src(dir) => Some(above.to_path_buf()),
        // `src/bin/tool/main.rs`
        "main.rs" if name_of(above) == "bin" => above
            .parent()
            .filter(|src| in_src(src))
            .and_then(Path::parent)
            .map(Path::to_path_buf),
        "build.rs" => Some(dir.to_path_buf()),
        _ if name_of(dir) == "bin" && in_src(above) => above.parent().map(Path::to_path_buf),
        // A file right under `tests`, beside the `src` of its package.
        _ if TARGET_DIRS.contains(&name_of(dir)) => ["lib.rs", "main.rs"]
            .iter()
            .any(|root| has(&above.join("src").join(root)))
            .then(|| above.to_path_buf()),
        _ => None,
    }
}

/// The directory the files of the modules declared in `file` are looked
/// for in. A root, a `mod.rs` and a file named by `#[path]` look beside
/// themselves; any other file, in the directory that carries its name.
fn children_dir(file: &Path, beside: bool) -> PathBuf {
    let dir = file.parent().unwrap_or(Path::new(""));
    if beside || name_of(file) == "mod.rs" {
        dir.to_path_buf()
    } else {
        dir.join(file.file_stem().unwrap_or_default())
    }
}

/// The module trees of the Rust files of a graph.
struct Crates<'a> {
    files: &'a [PathBuf],
    parsed: &'a [Option<RustFile>],
    by_path: HashMap<&'a Path, usize>,
    /// The file of each module.
    modules: HashMap<ModuleId, usize>,
    /// The module each file is, in the first crate that reaches it.
    identity: Vec<Option<ModuleId>>,
    /// The library crate each name stands for, as others write it.
    names: HashMap<String, usize>,
}

impl<'a> Crates<'a> {
    fn new(
        files: &'a [PathBuf],
        parsed: &'a [Option<RustFile>],
        packages: &[(PathBuf, String)],
    ) -> Self {
        let by_path: HashMap<&Path, usize> = (0..files.len())
            .filter(|&f| parsed[f].is_some())
            .map(|f| (files[f].as_path(), f))
            .collect();
        let has = |path: &Path| by_path.contains_key(path);
        // Libraries first: a file a library and a binary both declare is
        // the library's.
        let mut roots: Vec<(usize, PathBuf)> = by_path
            .values()
            .filter_map(|&f| Some((f, package_of(&files[f], &has)?)))
            .collect();
        roots.sort_by_key(|(f, _)| (name_of(&files[*f]) != "lib.rs", files[*f].clone()));

        let mut crates = Crates {
            files,
            parsed,
            by_path,
            modules: HashMap::new(),
            identity: vec![None; files.len()],
            names: HashMap::new(),
        };
        for (krate, (root, package)) in roots.iter().enumerate() {
            if name_of(&files[*root]) == "lib.rs" {
                let declared = packages.iter().find(|(dir, _)| dir == package);
                let name = declared.map_or(name_of(package), |(_, name)| name.as_str());
                crates.names.entry(name.replace('-', "_")).or_insert(krate);
            }
            crates.grow(krate, *root);
        }
        crates.grow_strays(roots.len());
        crates
    }

    /// A `lib.rs` or `main.rs` no crate reached is a root too: the tree
    /// analysed may start inside `src`, where nothing tells its layout.
    fn grow_strays(&mut self, known: usize) {
        let mut strays: Vec<usize> = self
            .by_path
            .values()
            .copied()
            .filter(|&f| matches!(name_of(&self.files[f]), "lib.rs" | "main.rs"))
            .collect();
        strays.sort_by_key(|&f| (name_of(&self.files[f]) != "lib.rs", self.files[f].clone()));
        for (at, root) in strays.into_iter().enumerate() {
            if self.identity[root].is_none() {
                self.grow(known + at, root);
            }
        }
    }

    /// Place every file the crate rooted at `root` reaches.
    fn grow(&mut self, krate: usize, root: usize) {
        let mut seen = HashSet::from([root]);
        let mut queue = VecDeque::from([(root, Vec::new(), true)]);
        while let Some((file, path, beside)) = queue.pop_front() {
            self.modules.insert((krate, path.clone()), file);
            self.identity[file].get_or_insert((krate, path.clone()));
            let below = children_dir(&self.files[file], beside);
            for decl in self.parsed[file].iter().flat_map(|p| &p.mods) {
                let mut module = path.clone();
                module.extend(decl.scope.iter().cloned());
                module.push(decl.name.clone());
                if decl.inline {
                    self.modules.insert((krate, module), file);
                } else if let Some(child) = self.file_of(file, &below, decl)
                    && seen.insert(child)
                {
                    queue.push_back((child, module, decl.path.is_some()));
                }
            }
        }
    }

    /// The file a `mod` declaration in `file` names.
    fn file_of(&self, file: usize, below: &Path, decl: &ModDecl) -> Option<usize> {
        let find = |path: PathBuf| self.by_path.get(normalize(&path).as_path()).copied();
        if let Some(named) = &decl.path {
            let dir = self.files[file].parent().unwrap_or(Path::new(""));
            return find(dir.join(named));
        }
        let dir = decl
            .scope
            .iter()
            .fold(below.to_path_buf(), |d, m| d.join(m));
        find(dir.join(format!("{}.rs", decl.name)))
            .or_else(|| find(dir.join(&decl.name).join("mod.rs")))
    }

    /// What a name `use` brought into `file` stands for.
    fn import(&self, file: usize, name: &str) -> Option<&PathRef> {
        let imports = &self.parsed[file].as_ref()?.imports;
        imports
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, path)| path)
    }

    /// The file a path written in `file` leads to. `hops` bounds how many
    /// imports are followed, since one may name itself.
    fn target(&self, file: usize, path: &PathRef, hops: usize) -> Option<usize> {
        let (krate, here) = self.identity[file].as_ref()?;
        let mut base = here.clone();
        base.extend(path.scope.iter().cloned());
        let segments = &path.segments;
        let first = segments.first()?;

        let (krate, absolute) = match first.as_str() {
            "crate" => (*krate, segments[1..].to_vec()),
            "self" => (*krate, [&base[..], &segments[1..]].concat()),
            "super" => {
                let ups = segments.iter().take_while(|s| *s == "super").count();
                let kept = base.len().checked_sub(ups)?;
                (*krate, [&base[..kept], &segments[ups..]].concat())
            }
            _ => {
                let child = [&base[..], &segments[..1]].concat();
                if self.modules.contains_key(&(*krate, child)) {
                    (*krate, [&base[..], &segments[..]].concat())
                } else if let Some(import) = self.import(file, first).filter(|_| hops > 0) {
                    let through = PathRef {
                        scope: import.scope.clone(),
                        segments: [&import.segments[..], &segments[1..]].concat(),
                    };
                    return self.target(file, &through, hops - 1);
                } else {
                    (*self.names.get(first)?, segments[1..].to_vec())
                }
            }
        };
        (0..=absolute.len())
            .rev()
            .find_map(|n| self.modules.get(&(krate, absolute[..n].to_vec())).copied())
    }
}

/// The files each Rust file uses, by index, for the files of a graph.
/// `parsed` holds what was read of each Rust file, and `packages` the
/// name each package directory declares.
pub fn uses(
    files: &[PathBuf],
    parsed: &[Option<RustFile>],
    packages: &[(PathBuf, String)],
) -> Vec<BTreeSet<usize>> {
    let crates = Crates::new(files, parsed, packages);
    let led = |file: usize, paths: &[PathRef]| -> BTreeSet<usize> {
        let targets = paths.iter().filter_map(|path| crates.target(file, path, 3));
        targets.collect()
    };
    let read = |file: usize| parsed[file].as_ref();
    let mut uses: Vec<BTreeSet<usize>> = (0..files.len())
        .map(|file| read(file).map_or_else(BTreeSet::new, |r| led(file, &r.paths)))
        .collect();
    // A type uses the files that hold its `impl` blocks: what they define
    // is reached through the type, with no path that names them.
    for file in 0..files.len() {
        for owner in read(file).map_or_else(BTreeSet::new, |r| led(file, &r.impls)) {
            uses[owner].insert(file);
        }
    }
    uses
}

/// Read the Rust files of a graph, given as the text of each file that
/// is one: the files each uses, and whether each holds tests of its own.
pub fn read(
    files: &[PathBuf],
    texts: &[Option<&str>],
    packages: &[(PathBuf, String)],
) -> (Vec<Option<BTreeSet<usize>>>, Vec<bool>) {
    let parsed: Vec<Option<RustFile>> = texts.iter().map(|t| t.map(super::rust::parse)).collect();
    let own_tests = parsed
        .iter()
        .map(|read| read.as_ref().is_some_and(|read| read.has_tests))
        .collect();
    let used = uses(files, &parsed, packages)
        .into_iter()
        .zip(texts)
        .map(|(used, text)| text.map(|_| used))
        .collect();
    (used, own_tests)
}

#[cfg(test)]
#[path = "rust_crates_test.rs"]
mod tests;
