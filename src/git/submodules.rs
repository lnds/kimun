//! The tree of a ref written to a directory as a checkout writes it: its
//! symbolic links by `core.symlinks`, and, when they are read, its
//! submodules at the commit each gitlink records; and the files a submodule
//! changed since a ref.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use git2::{
    Config, ConfigEntry, Delta, DiffDelta, FileMode, ObjectType, Oid, Repository, Tree, TreeEntry,
};

use super::changeset::diff_workdir;
use super::{FileChange, GitRepo};

/// A submodule the extraction could not read, with what to run about it.
pub struct LeftOut {
    /// Its path from the root of the superproject.
    pub path: PathBuf,
    /// What is wrong, and what to do about it.
    pub advice: String,
}

/// Why a submodule cannot be read.
enum Unreadable {
    /// No checkout in the working tree; `parent` is the submodule that
    /// should hold it, none at the top.
    NotCheckedOut {
        parent: Option<PathBuf>,
    },
    /// The commit is not in the repository; `checkout` is where the working
    /// tree has it now, if it does.
    MissingCommit {
        id: Oid,
        shallow: bool,
        checkout: Option<PathBuf>,
    },
    /// A gitlink that `.gitmodules` does not name.
    NoUrl,
    /// A submodule removed or moved since, of which git keeps no repository.
    Gone,
    NoRepository,
    Failed(String),
}

impl Unreadable {
    /// What is wrong, and what to do about it, for a submodule at `shown`.
    fn advice(&self, shown: &Path) -> String {
        let at = shown.display();
        match self {
            Self::NotCheckedOut { parent: None } => {
                format!("it is not checked out (git submodule update --init {at})")
            }
            Self::NotCheckedOut {
                parent: Some(parent),
            } => {
                let rel = shown.strip_prefix(parent).unwrap_or(shown);
                let (parent, rel) = (parent.display(), rel.display());
                format!("it is not checked out (git -C {parent} submodule update --init {rel})")
            }
            Self::MissingCommit {
                id,
                shallow,
                checkout,
            } => {
                let short = &id.to_string()[..7];
                match checkout {
                    Some(checkout) => {
                        let fetch = if *shallow {
                            "fetch --unshallow"
                        } else {
                            "fetch"
                        };
                        let checkout = checkout.display();
                        format!(
                            "commit {short} is not in its repository (git -C {checkout} {fetch})"
                        )
                    }
                    None => format!("commit {short} is not in the repository git keeps for it"),
                }
            }
            Self::NoUrl => "it has no url in .gitmodules".to_string(),
            Self::Gone => {
                "it was moved or removed since, and git keeps no repository for it".to_string()
            }
            Self::NoRepository => {
                format!("no repository holds it (git submodule update --init {at})")
            }
            Self::Failed(why) => format!("it could not be read ({why})"),
        }
    }
}

/// Whether `name` is a valid name for a submodule, as git checks it: it
/// names a directory under `modules`, so it is not empty, not absolute, has
/// no backslash and no component equal to `.` or `..`.
fn is_submodule_name(name: &str) -> bool {
    !name.is_empty()
        && !Path::new(name).is_absolute()
        && !name.contains('\\')
        && name.split('/').all(|part| part != "." && part != "..")
}

/// Whether the name of a tree entry may be written inside its directory:
/// it is no `.`, `..` or `.git` (in any case) and holds no separator.
fn is_safe_entry_name(name: &str) -> bool {
    !matches!(name, "." | "..") && !name.eq_ignore_ascii_case(".git") && !name.contains(['/', '\\'])
}

/// Where an entry of a tree goes: its file on disk, its path in the
/// repository that holds the tree, and its path from the superproject.
pub(super) struct Place {
    dest: PathBuf,
    rel: PathBuf,
    shown: PathBuf,
}

impl Place {
    /// The place of the root of a tree, written into `dest`.
    pub(super) fn root(dest: &Path) -> Self {
        Self {
            dest: dest.to_path_buf(),
            rel: PathBuf::new(),
            shown: PathBuf::new(),
        }
    }

    /// The place of the entry `name` of this tree.
    fn child(&self, name: &str) -> Self {
        Self {
            dest: self.dest.join(name),
            rel: self.rel.join(name),
            shown: self.shown.join(name),
        }
    }

    /// The root of the tree of a submodule found here.
    fn inside(&self) -> Self {
        Self {
            dest: self.dest.clone(),
            rel: PathBuf::new(),
            shown: self.shown.clone(),
        }
    }

    /// The submodule whose tree holds this place, none at the top.
    fn parent(&self) -> Option<PathBuf> {
        let mut parent = self.shown.clone();
        for _ in self.rel.components() {
            parent.pop();
        }
        (!parent.as_os_str().is_empty()).then_some(parent)
    }

    /// Whether a submodule here is under `under`, or holds it, both from the
    /// superproject; `under` empty is the whole tree.
    fn is_near(&self, under: &Path) -> bool {
        self.shown.starts_with(under) || under.starts_with(&self.shown)
    }
}

/// One repository of the extraction: the superproject, or a submodule.
pub(super) struct Level<'r> {
    repo: &'r Repository,
    /// Where the working tree holds the checkout of this repository, if it
    /// does.
    workdir: Option<PathBuf>,
    /// The name of each submodule by its path, from the `.gitmodules` of
    /// the tree being written.
    names: HashMap<PathBuf, String>,
    /// Whether a checkout makes symbolic links links (`core.symlinks`).
    links: bool,
}

/// The repository of a submodule, and where the working tree has it.
struct Opened {
    repo: Repository,
    checkout: Option<PathBuf>,
}

impl Opened {
    /// `repo`, whose checkout is its working directory when that exists.
    fn of(repo: Repository) -> Self {
        let checkout = repo.workdir().filter(|dir| dir.exists()).map(PathBuf::from);
        Self { repo, checkout }
    }
}

impl<'r> Level<'r> {
    /// The level of `repo`, checked out at `workdir`. `gitmodules` is the
    /// tree whose `.gitmodules` names its submodules, when they are read.
    pub(super) fn of(
        repo: &'r Repository,
        workdir: Option<PathBuf>,
        gitmodules: Option<&Tree>,
    ) -> Self {
        Self {
            repo,
            workdir,
            names: gitmodules
                .map(|t| gitmodules_names(repo, t))
                .unwrap_or_default(),
            links: checks_out_links(repo),
        }
    }

    /// The repository of the submodule at `at`: its checkout in the working
    /// tree, else the copy this repository keeps in `modules`, for one
    /// moved or removed since, or when this level has no checkout.
    fn open_submodule(&self, at: &Place) -> Result<Opened, Unreadable> {
        let rel = &at.rel;
        if let Some(workdir) = &self.workdir {
            let checkout = workdir.join(rel);
            if checkout.join(".git").exists() {
                return Repository::open(&checkout)
                    .map(Opened::of)
                    .map_err(|_| self.no_repository(rel));
            }
            if self.has_gitlink(rel) {
                return Err(Unreadable::NotCheckedOut {
                    parent: at.parent(),
                });
            }
        }
        let name = match self.names.get(rel) {
            Some(name) => name.clone(),
            None => rel.to_string_lossy().into_owned(),
        };
        if !is_submodule_name(&name) {
            return Err(self.no_repository(rel));
        }
        let kept = self.repo.path().join("modules").join(name);
        // Open for its `core.worktree`, where a moved submodule has its
        // checkout now; as bare when that is gone.
        Repository::open(&kept)
            .or_else(|_| Repository::open_bare(&kept))
            .map(Opened::of)
            .map_err(|_| self.no_repository(rel))
    }

    /// Why no repository was found for the submodule at `rel`.
    fn no_repository(&self, rel: &Path) -> Unreadable {
        if !self.names.contains_key(rel) {
            Unreadable::NoUrl
        } else if !self.has_gitlink(rel) {
            Unreadable::Gone
        } else {
            Unreadable::NoRepository
        }
    }

    /// Whether the index of this repository holds a gitlink at `rel`.
    fn has_gitlink(&self, rel: &Path) -> bool {
        let gitlink = u32::from(FileMode::Commit);
        self.repo
            .index()
            .ok()
            .and_then(|index| index.get_path(rel, 0))
            .is_some_and(|entry| entry.mode == gitlink)
    }
}

/// Mark `dir`, where a tree is written, as a repository: an empty `.git`
/// holding the `info/exclude` the walk applies at `checkout`, where the
/// working tree has that repository, when it applies one there
/// ([`walked_exclude`]). Writes only inside `dir`; an exclude file that
/// cannot be read is left out, as the walk leaves it out.
pub(super) fn mark_as_repository(
    checkout: Option<&Path>,
    dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let info = dir.join(".git").join("info");
    fs::create_dir_all(&info)?;
    if let Some(exclude) = checkout.and_then(walked_exclude) {
        let _ = fs::copy(exclude, info.join("exclude"));
    }
    Ok(())
}

/// The `info/exclude` the walk applies to the checkout at `checkout`, found
/// as the ignore crate finds it: in `.git` when that is a directory, and
/// through a `.git` file only when the directory it names (from the current
/// directory, when relative) holds a `commondir` file, as that of a linked
/// worktree does. The `.git` file of a submodule, or of a clone with
/// `--separate-git-dir`, leads to none.
fn walked_exclude(checkout: &Path) -> Option<PathBuf> {
    let dot_git = checkout.join(".git");
    let git_dir = if dot_git.is_dir() {
        dot_git
    } else {
        let text = fs::read_to_string(&dot_git).ok()?;
        let named = PathBuf::from(text.lines().next()?.strip_prefix("gitdir: ")?);
        let common = fs::read_to_string(named.join("commondir")).ok()?;
        let common = common.lines().next()?;
        if common.starts_with('.') {
            named.join(common)
        } else {
            PathBuf::from(common)
        }
    };
    Some(git_dir.join("info").join("exclude")).filter(|path| path.is_file())
}

/// Whether a checkout of `repo` makes symbolic links links rather than
/// files holding the path they point to: `core.symlinks`, which is on by
/// default, and off by default on Windows.
fn checks_out_links(repo: &Repository) -> bool {
    repo.config()
        .and_then(|config| config.get_bool("core.symlinks"))
        .unwrap_or(!cfg!(windows))
}

/// The name of each submodule by its path, from the `.gitmodules` at the
/// root of `tree`. Read by libgit2's parser through a temporary file.
fn gitmodules_names(repo: &Repository, tree: &Tree) -> HashMap<PathBuf, String> {
    read_gitmodules(repo, tree).unwrap_or_default()
}

/// The names of [`gitmodules_names`], or none when `tree` has no
/// `.gitmodules` or it cannot be parsed.
fn read_gitmodules(repo: &Repository, tree: &Tree) -> Option<HashMap<PathBuf, String>> {
    let blob = repo.find_blob(tree.get_name(".gitmodules")?.id()).ok()?;
    let file = tempfile::NamedTempFile::new().ok()?;
    fs::write(file.path(), blob.content()).ok()?;
    let config = Config::open(file.path()).ok()?;
    let mut entries = config.entries(Some(r"submodule\..*\.path")).ok()?;
    let mut names = HashMap::new();
    while let Some(Ok(entry)) = entries.next() {
        names.extend(path_and_name(entry));
    }
    Some(names)
}

/// The path and the name of a `submodule.<name>.path` entry.
fn path_and_name(entry: &ConfigEntry) -> Option<(PathBuf, String)> {
    let key = entry.name().ok()?;
    let name = key.strip_prefix("submodule.")?.strip_suffix(".path")?;
    Some((PathBuf::from(entry.value().ok()?), name.to_string()))
}

impl GitRepo {
    /// Whether the working tree holds a submodule at `path`, from the root
    /// of the repository: its directory holds `.git`, or the index records a
    /// gitlink there.
    pub fn is_submodule_now(&self, path: &Path) -> bool {
        let gitlink = u32::from(FileMode::Commit);
        self.root.join(path).join(".git").symlink_metadata().is_ok()
            || self
                .repo
                .index()
                .ok()
                .and_then(|index| index.get_path(path, 0))
                .is_some_and(|entry| entry.mode == gitlink)
    }
}

/// The writing of a tree into a directory: its files, and the submodules
/// of the tree when they are read.
pub(super) struct Extraction {
    read_submodules: bool,
    /// The path analyzed, from the superproject: only the submodules on its
    /// way, or under it, are read; empty is the whole tree.
    under: PathBuf,
    /// The root of the superproject, to show where a checkout is.
    root: PathBuf,
    left_out: Vec<LeftOut>,
}

impl Extraction {
    /// The writing of a tree of the superproject at `root`, reading its
    /// submodules when `read_submodules`, those near `under` only.
    pub(super) fn new(read_submodules: bool, under: &Path, root: &Path) -> Self {
        Self {
            read_submodules,
            under: under.to_path_buf(),
            root: root.to_path_buf(),
            left_out: Vec::new(),
        }
    }

    /// The submodules that could not be written.
    pub(super) fn into_left_out(self) -> Vec<LeftOut> {
        self.left_out
    }

    /// Write `tree`, of the repository of `level`, at `place`.
    pub(super) fn write(
        &mut self,
        level: &Level,
        tree: &Tree,
        place: &Place,
    ) -> Result<(), Box<dyn Error>> {
        for entry in tree.iter() {
            let name = entry
                .name()
                .map_err(|_| format!("non-UTF-8 entry in tree: {:?}", entry.id()))?;
            if is_safe_entry_name(name) {
                self.write_entry(level, &entry, &place.child(name))?;
            }
        }
        Ok(())
    }

    /// Write one entry of a tree at `at`: a file, a directory, or, when
    /// submodules are read and it lies near the path analyzed, a submodule.
    fn write_entry(
        &mut self,
        level: &Level,
        entry: &TreeEntry,
        at: &Place,
    ) -> Result<(), Box<dyn Error>> {
        match entry.kind() {
            Some(ObjectType::Blob)
                if level.links && entry.filemode() == i32::from(FileMode::Link) => {}
            Some(ObjectType::Blob) => {
                fs::write(&at.dest, level.repo.find_blob(entry.id())?.content())?;
            }
            Some(ObjectType::Tree) => {
                fs::create_dir_all(&at.dest)?;
                self.write(level, &level.repo.find_tree(entry.id())?, at)?;
            }
            Some(ObjectType::Commit) if self.read_submodules && at.is_near(&self.under) => {
                self.write_gitlink(level, entry.id(), at);
            }
            _ => {} // submodules, not read or not analyzed
        }
        Ok(())
    }

    /// Write the tree of the commit a gitlink records, or leave the
    /// submodule out when it cannot be read.
    fn write_gitlink(&mut self, level: &Level, id: Oid, at: &Place) {
        if let Err(why) = self.write_submodule(level, id, at) {
            self.left_out.push(LeftOut {
                path: at.shown.clone(),
                advice: why.advice(&at.shown),
            });
        }
    }

    /// Write the tree of the commit `id` of the submodule at `at`, with the
    /// submodules it holds.
    fn write_submodule(&mut self, level: &Level, id: Oid, at: &Place) -> Result<(), Unreadable> {
        let opened = level.open_submodule(at)?;
        let sub = &opened.repo;
        let tree = sub
            .find_commit(id)
            .map_err(|_| Unreadable::MissingCommit {
                id,
                shallow: sub.is_shallow(),
                checkout: opened
                    .checkout
                    .as_deref()
                    .map(|dir| self.relative_to_root(dir)),
            })?
            .tree()
            .map_err(|e| Unreadable::Failed(e.to_string()))?;
        let inner = Level::of(sub, opened.checkout.clone(), Some(&tree));
        let before = self.left_out.len();
        let written = fs::create_dir_all(&at.dest)
            .map_err(Box::from)
            .and_then(|()| self.write(&inner, &tree, &at.inside()))
            .and_then(|()| mark_as_repository(opened.checkout.as_deref(), &at.dest));
        written.map_err(|e| {
            self.left_out.truncate(before);
            let _ = fs::remove_dir_all(&at.dest);
            Unreadable::Failed(e.to_string())
        })
    }

    /// `dir` from the root of the superproject, as given when it cannot be.
    fn relative_to_root(&self, dir: &Path) -> PathBuf {
        let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        dir.strip_prefix(&self.root)
            .map(Path::to_path_buf)
            .or_else(|_| {
                canonical(dir)
                    .strip_prefix(canonical(&self.root))
                    .map(Path::to_path_buf)
            })
            .unwrap_or_else(|_| dir.to_path_buf())
    }
}

/// Collect into `out` the files of `repo` that differ from `old_tree`
/// (none: all of them) in the working tree, under `prefix`. With
/// `submodules`, a changed gitlink stands for the files its submodule
/// changed.
pub(super) fn collect_changes(
    repo: &Repository,
    old_tree: Option<&Tree>,
    prefix: &Path,
    submodules: bool,
    out: &mut Vec<FileChange>,
) -> Result<(), Box<dyn Error>> {
    let diff = diff_workdir(repo, old_tree, false)?;
    for delta in diff.deltas() {
        if submodules && is_gitlink(&delta) {
            collect_submodule_changes(repo, &delta, prefix, out);
        } else if let Some(change) = file_change(&delta, prefix) {
            out.push(change);
        }
    }
    Ok(())
}

/// The change a delta stands for, under `prefix`: files added, modified or
/// renamed, not deleted.
fn file_change(delta: &DiffDelta, prefix: &Path) -> Option<FileChange> {
    let old_path = match delta.status() {
        Delta::Added | Delta::Untracked | Delta::Copied => None,
        Delta::Modified | Delta::Renamed | Delta::Typechange => {
            delta.old_file().path().map(|p| prefix.join(p))
        }
        _ => return None,
    };
    let path = delta.new_file().path()?;
    Some(FileChange {
        path: prefix.join(path),
        old_path,
    })
}

/// Whether either side of a delta is a submodule.
fn is_gitlink(delta: &DiffDelta) -> bool {
    delta.old_file().mode() == FileMode::Commit || delta.new_file().mode() == FileMode::Commit
}

/// The changes inside the submodule a delta is about: from the tree of the
/// commit it was at, to its working tree. A submodule that cannot be read
/// contributes nothing.
fn collect_submodule_changes(
    repo: &Repository,
    delta: &DiffDelta,
    prefix: &Path,
    out: &mut Vec<FileChange>,
) {
    let (Some(workdir), Some(path)) = (repo.workdir(), delta.new_file().path()) else {
        return;
    };
    let Ok(sub) = Repository::open(workdir.join(path)) else {
        return;
    };
    let old = delta.old_file();
    let old_tree = if old.mode() == FileMode::Commit {
        match sub.find_commit(old.id()).and_then(|c| c.tree()) {
            Ok(tree) => Some(tree),
            Err(_) => return,
        }
    } else {
        None
    };
    let _ = collect_changes(&sub, old_tree.as_ref(), &prefix.join(path), true, out);
}

#[cfg(test)]
#[path = "submodules_test.rs"]
mod tests;
