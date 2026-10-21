use super::*;
use crate::config::KindsConfig;
use crate::kind::{Kind, Selection};
use crate::walk::ExcludeFilter;
use std::fs;

fn docs_filter() -> ExcludeFilter {
    ExcludeFilter::default().with_kinds(Selection::new(&[Kind::Docs], &KindsConfig::default()))
}

fn commit_all(repo: &git2::Repository, message: &str) {
    let sig =
        git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0)).unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .unwrap();
}

fn write(root: &Path, path: &str, text: &str) {
    let full = root.join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, text).unwrap();
}

/// A repository whose documents name `src/old.rs`, then the file deleted
/// and one document edited.
fn repository_with_a_deletion() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let root = dir.path();
    write(root, "src/old.rs", "fn old() {}\n");
    write(root, "src/new.rs", "fn new() {}\n");
    write(
        root,
        "README.md",
        "Read `src/old.rs`.\n[gone](docs/gone.md)\n",
    );
    write(root, "docs/guide.md", "# Guide\n[gone](gone.md)\n");
    write(root, "docs/notes.md", "# Notes\n");
    commit_all(&repo, "initial");
    fs::remove_file(root.join("src/old.rs")).unwrap();
    write(root, "docs/notes.md", "# Notes\n[x](missing.md)\n");
    dir
}

fn dead_of(report: &RefsReport) -> Vec<(String, String)> {
    report
        .with_dead
        .iter()
        .flat_map(|d| {
            let name = d.path.file_name().unwrap().to_string_lossy().into_owned();
            d.dead
                .iter()
                .map(move |r| (name.clone(), r.reference.clone()))
        })
        .collect()
}

#[test]
fn every_dead_reference_of_every_document() {
    let dir = repository_with_a_deletion();
    let filter = docs_filter();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let repository = repository(&dir.path().canonicalize().unwrap());
    let report = collect(&cfg, &repository, None).unwrap();
    assert_eq!(report.documents, 3);
    assert_eq!(report.dead(), 4);
}

#[test]
fn since_a_ref_changed_documents_and_references_to_changed_files() {
    let dir = repository_with_a_deletion();
    let filter = docs_filter();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let repository = repository(&dir.path().canonicalize().unwrap());
    let changed = changed_since(dir.path(), "HEAD").unwrap();
    let report = collect(&cfg, &repository, Some(&changed)).unwrap();
    assert_eq!(
        dead_of(&report),
        vec![
            ("README.md".to_string(), "src/old.rs".to_string()),
            ("notes.md".to_string(), "missing.md".to_string()),
        ]
    );
}

#[test]
fn fail_on_dead_fails_only_when_there_is_one() {
    let dir = repository_with_a_deletion();
    let filter = docs_filter();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let failing = RefsOptions {
        fail_on_dead: true,
        since_ref: None,
    };
    let err = refs(&cfg, OutputMode::Terse, &failing).unwrap_err();
    assert!(err.to_string().contains("4 dead reference"), "{err}");
    let lenient = RefsOptions {
        fail_on_dead: false,
        since_ref: None,
    };
    assert!(refs(&cfg, OutputMode::Json, &lenient).is_ok());
}

#[test]
fn outside_a_repository_the_path_is_the_root() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.md", "[b](b.md) [c](c.md)\n");
    write(dir.path(), "b.md", "");
    let filter = docs_filter();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let root = repository_root(dir.path()).unwrap();
    let report = collect(&cfg, &repository(&root), None).unwrap();
    assert_eq!(
        dead_of(&report),
        vec![("a.md".to_string(), "c.md".to_string())]
    );
}

#[test]
fn names_a_change_by_the_whole_path_or_its_end() {
    let changed: HashSet<PathBuf> = [PathBuf::from("src/deps/graph.rs")].into();
    assert!(names_a_change(Path::new("src/deps/graph.rs"), &changed));
    assert!(names_a_change(Path::new("deps/graph.rs"), &changed));
    assert!(!names_a_change(Path::new("ps/graph.rs"), &changed));
    assert!(!names_a_change(Path::new(""), &changed));
}

#[test]
fn stale_needs_a_repository_and_runs_in_one() {
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "a.md", "`src/a.rs`\n");
    let filter = docs_filter();
    let cfg = WalkConfig::new(outside.path(), true, &filter);
    let err = stale(&cfg, OutputMode::Table, None).unwrap_err();
    assert!(err.to_string().contains("not a git repository"), "{err}");

    let dir = repository_with_a_deletion();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    for output in [OutputMode::Table, OutputMode::Json, OutputMode::Short] {
        stale(&cfg, output, None).unwrap();
    }
}
