use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};

use git2::{BlameOptions, DiffOptions, Repository, Sort};

pub struct GitRepo {
    repo: Repository,
    root: PathBuf,
}

pub struct FileFrequency {
    pub path: PathBuf,
    pub commits: usize,
    pub first_commit: i64,
    pub last_commit: i64,
}

pub struct BlameInfo {
    pub author: String,
    pub email: String,
    pub lines: usize,
    pub last_commit_time: i64,
}

impl GitRepo {
    pub fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
        let repo = Repository::discover(path)?;
        let root = repo
            .workdir()
            .ok_or("bare repositories are not supported")?
            .to_path_buf();
        Ok(Self { repo, root })
    }

    #[allow(dead_code)]
    pub fn is_git_repo(path: &Path) -> bool {
        Repository::discover(path).is_ok()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn file_frequencies(
        &self,
        since: Option<i64>,
    ) -> Result<Vec<FileFrequency>, Box<dyn Error>> {
        let mut map: HashMap<PathBuf, FileFrequency> = HashMap::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            // Skip merge commits
            if commit.parent_count() > 1 {
                continue;
            }

            let time = commit.time().seconds();
            if let Some(since_ts) = since
                && time < since_ts
            {
                continue;
            }

            let paths = self.changed_files(&commit)?;
            for path in paths {
                map.entry(path.clone())
                    .and_modify(|f| {
                        f.commits += 1;
                        if time < f.first_commit {
                            f.first_commit = time;
                        }
                        if time > f.last_commit {
                            f.last_commit = time;
                        }
                    })
                    .or_insert(FileFrequency {
                        path,
                        commits: 1,
                        first_commit: time,
                        last_commit: time,
                    });
            }
        }

        let mut result: Vec<FileFrequency> = map.into_values().collect();
        result.sort_by(|a, b| b.commits.cmp(&a.commits));
        Ok(result)
    }

    #[allow(dead_code)]
    pub fn co_changing_commits(
        &self,
        since: Option<i64>,
    ) -> Result<Vec<Vec<PathBuf>>, Box<dyn Error>> {
        let mut result = Vec::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            if commit.parent_count() > 1 {
                continue;
            }

            let time = commit.time().seconds();
            if let Some(since_ts) = since
                && time < since_ts
            {
                continue;
            }

            let paths = self.changed_files(&commit)?;
            if paths.len() >= 2 {
                result.push(paths);
            }
        }

        Ok(result)
    }

    /// Run git blame on a file and return per-author contributions.
    /// `rel_path` is relative to the git root.
    pub fn blame_file(&self, rel_path: &Path) -> Result<Vec<BlameInfo>, Box<dyn Error>> {
        let mut opts = BlameOptions::new();
        let blame = self.repo.blame_file(rel_path, Some(&mut opts))?;

        let mut map: HashMap<String, BlameInfo> = HashMap::new();

        for hunk in blame.iter() {
            let sig = hunk.final_signature();
            let email = sig.email().unwrap_or("unknown").to_string();
            let author = sig.name().unwrap_or("unknown").to_string();
            let time = hunk.final_commit_id();
            let commit_time = self
                .repo
                .find_commit(time)
                .map(|c| c.time().seconds())
                .unwrap_or(0);
            let lines = hunk.lines_in_hunk();

            map.entry(email.clone())
                .and_modify(|info| {
                    info.lines += lines;
                    if commit_time > info.last_commit_time {
                        info.last_commit_time = commit_time;
                    }
                })
                .or_insert(BlameInfo {
                    author,
                    email,
                    lines,
                    last_commit_time: commit_time,
                });
        }

        let mut result: Vec<BlameInfo> = map.into_values().collect();
        result.sort_by(|a, b| b.lines.cmp(&a.lines));
        Ok(result)
    }

    /// Collect authors who have commits since the given timestamp.
    pub fn recent_authors(&self, since: Option<i64>) -> Result<HashSet<String>, Box<dyn Error>> {
        let mut authors = HashSet::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let time = commit.time().seconds();
            if let Some(since_ts) = since
                && time < since_ts
            {
                continue;
            }
            if let Some(email) = commit.author().email() {
                authors.insert(email.to_string());
            }
        }

        Ok(authors)
    }

    fn changed_files(&self, commit: &git2::Commit) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut opts = DiffOptions::new();
        let diff =
            self.repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;

        let mut paths = Vec::new();
        for delta in diff.deltas() {
            if let Some(path) = delta.new_file().path() {
                paths.push(path.to_path_buf());
            }
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Configure identity for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        (dir, repo)
    }

    fn make_commit_at(
        repo: &Repository,
        files: &[(&str, &str)],
        message: &str,
        epoch: i64,
    ) -> git2::Oid {
        let sig =
            git2::Signature::new("Test", "test@test.com", &git2::Time::new(epoch, 0)).unwrap();
        let mut index = repo.index().unwrap();

        for (path, content) in files {
            let full_path = repo.workdir().unwrap().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full_path, content).unwrap();
            index.add_path(Path::new(path)).unwrap();
        }

        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();

        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    fn make_commit(repo: &Repository, files: &[(&str, &str)], message: &str) -> git2::Oid {
        make_commit_at(repo, files, message, 1_700_000_000)
    }

    #[test]
    fn test_open_repo() {
        let (dir, _repo) = create_test_repo();
        let git_repo = GitRepo::open(dir.path());
        assert!(git_repo.is_ok());
        assert!(GitRepo::is_git_repo(dir.path()));
    }

    #[test]
    fn test_open_not_repo() {
        let dir = tempfile::tempdir().unwrap();
        // Don't init git — just a plain directory
        let sub = dir.path().join("not_a_repo");
        fs::create_dir_all(&sub).unwrap();
        assert!(GitRepo::open(&sub).is_err());
        assert!(!GitRepo::is_git_repo(&sub));
    }

    #[test]
    fn test_file_frequencies() {
        let (dir, repo) = create_test_repo();

        make_commit(&repo, &[("a.rs", "fn a() {}")], "add a");
        make_commit(&repo, &[("b.rs", "fn b() {}")], "add b");
        make_commit(&repo, &[("a.rs", "fn a() { 1 }")], "modify a");

        let git_repo = GitRepo::open(dir.path()).unwrap();
        let freqs = git_repo.file_frequencies(None).unwrap();

        assert_eq!(freqs.len(), 2);

        let a = freqs.iter().find(|f| f.path == Path::new("a.rs")).unwrap();
        assert_eq!(a.commits, 2);

        let b = freqs.iter().find(|f| f.path == Path::new("b.rs")).unwrap();
        assert_eq!(b.commits, 1);
    }

    #[test]
    fn test_file_frequencies_since() {
        let (dir, repo) = create_test_repo();

        make_commit_at(&repo, &[("a.rs", "v1")], "first", 1_000_000);
        make_commit_at(&repo, &[("b.rs", "v1")], "second", 2_000_000);

        let git_repo = GitRepo::open(dir.path()).unwrap();
        // Filter: only commits at or after 1_500_000 → only the second commit
        let freqs = git_repo.file_frequencies(Some(1_500_000)).unwrap();

        assert_eq!(freqs.len(), 1);
        assert_eq!(freqs[0].path, Path::new("b.rs"));
    }

    #[test]
    fn test_co_changing_commits() {
        let (dir, repo) = create_test_repo();

        // Single-file commit — should NOT appear
        make_commit(&repo, &[("a.rs", "v1")], "one file");

        // Multi-file commit — should appear
        make_commit(&repo, &[("b.rs", "v1"), ("c.rs", "v1")], "two files");

        let git_repo = GitRepo::open(dir.path()).unwrap();
        let co = git_repo.co_changing_commits(None).unwrap();

        assert_eq!(co.len(), 1);
        assert_eq!(co[0].len(), 2);
        assert!(co[0].contains(&PathBuf::from("b.rs")));
        assert!(co[0].contains(&PathBuf::from("c.rs")));
    }

    #[test]
    fn test_blame_single_author() {
        let (dir, repo) = create_test_repo();
        make_commit(&repo, &[("a.rs", "line1\nline2\nline3\n")], "add a");

        let git_repo = GitRepo::open(dir.path()).unwrap();
        let blames = git_repo.blame_file(Path::new("a.rs")).unwrap();

        assert_eq!(blames.len(), 1, "single author should produce 1 entry");
        assert_eq!(blames[0].email, "test@test.com");
        assert_eq!(blames[0].lines, 3);
    }

    #[test]
    fn test_blame_multiple_authors() {
        let (dir, repo) = create_test_repo();

        // First author commits line1 and line2
        let sig1 = git2::Signature::new(
            "Alice",
            "alice@test.com",
            &git2::Time::new(1_700_000_000, 0),
        )
        .unwrap();
        let mut index = repo.index().unwrap();
        let full_path = repo.workdir().unwrap().join("a.rs");
        fs::write(&full_path, "line1\nline2\n").unwrap();
        index.add_path(Path::new("a.rs")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig1, &sig1, "alice adds", &tree, &[])
            .unwrap();

        // Second author modifies line1 but keeps line2
        let sig2 = git2::Signature::new("Bob", "bob@test.com", &git2::Time::new(1_700_001_000, 0))
            .unwrap();
        let mut index = repo.index().unwrap();
        fs::write(&full_path, "modified\nline2\n").unwrap();
        index.add_path(Path::new("a.rs")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &sig2,
            &sig2,
            "bob modifies",
            &tree,
            &[&parent],
        )
        .unwrap();

        let git_repo = GitRepo::open(dir.path()).unwrap();
        let blames = git_repo.blame_file(Path::new("a.rs")).unwrap();

        assert_eq!(blames.len(), 2, "two authors should produce 2 entries");
        let total_lines: usize = blames.iter().map(|b| b.lines).sum();
        assert_eq!(total_lines, 2, "total blamed lines should be 2");
    }

    #[test]
    fn test_blame_nonexistent_file() {
        let (dir, repo) = create_test_repo();
        make_commit(&repo, &[("a.rs", "content\n")], "add a");

        let git_repo = GitRepo::open(dir.path()).unwrap();
        let result = git_repo.blame_file(Path::new("nonexistent.rs"));
        assert!(result.is_err(), "blame on missing file should fail");
    }

    #[test]
    fn test_recent_authors() {
        let (dir, repo) = create_test_repo();
        make_commit_at(&repo, &[("a.rs", "v1")], "first", 1_000_000);
        make_commit_at(&repo, &[("b.rs", "v1")], "second", 2_000_000);

        let git_repo = GitRepo::open(dir.path()).unwrap();
        let authors = git_repo.recent_authors(Some(1_500_000)).unwrap();
        assert!(
            authors.contains("test@test.com"),
            "should contain author of recent commit"
        );

        let all_authors = git_repo.recent_authors(None).unwrap();
        assert!(!all_authors.is_empty());
    }

    #[test]
    fn test_empty_repo() {
        let (dir, _repo) = create_test_repo();
        let git_repo = GitRepo::open(dir.path()).unwrap();

        // Empty repo has no HEAD, revwalk.push_head() will fail
        let freqs = git_repo.file_frequencies(None);
        assert!(freqs.is_err() || freqs.unwrap().is_empty());

        let co = git_repo.co_changing_commits(None);
        assert!(co.is_err() || co.unwrap().is_empty());
    }
}
