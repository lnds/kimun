use super::*;

fn info_of(json: &str) -> PrInfo {
    serde_json::from_str(json).unwrap()
}

const OPEN: &str = r#"{"baseRefOid":"base","headRefOid":"head","mergeCommit":null}"#;
const MERGED: &str =
    r#"{"baseRefOid":"base","headRefOid":"head","mergeCommit":{"oid":"merge"},"state":"MERGED"}"#;

fn refs(since: &str, until: &str) -> Plan {
    Plan::Refs {
        since: since.to_string(),
        until: until.to_string(),
    }
}

fn merged(merge: &str, before: &str) -> Plan {
    Plan::Merged {
        merge: merge.to_string(),
        before: before.to_string(),
    }
}

/// The first parent of `merge` is `parent`, and no other commit is known.
fn parent_of_merge(commit: &str) -> Option<String> {
    (commit == "merge").then(|| "parent".to_string())
}

#[test]
fn with_both_ends_local_the_commits_are_measured() {
    // Open, or merged with a merge commit: the head is in the history.
    assert_eq!(
        plan(&info_of(OPEN), |_| true, |_| None),
        refs("base", "head")
    );
    assert_eq!(
        plan(&info_of(MERGED), |_| true, parent_of_merge),
        refs("base", "head")
    );
}

#[test]
fn a_squashed_or_rebased_pull_request_is_measured_from_its_patch_at_the_merge() {
    // The head never reached this repository; what merged it did. The base
    // may be many merges behind, so the two are not compared.
    let plan = plan(&info_of(MERGED), |c| c != "head", parent_of_merge);
    assert_eq!(plan, merged("merge", "parent"));
}

#[test]
fn the_merge_is_enough_without_the_base() {
    let plan = plan(&info_of(MERGED), |c| c == "merge", parent_of_merge);
    assert_eq!(plan, merged("merge", "parent"));
}

#[test]
fn an_open_pull_request_without_its_head_is_measured_from_its_patch() {
    // From a fork, or not fetched: only the base is here.
    let plan = plan(&info_of(OPEN), |c| c == "base", |_| None);
    assert_eq!(
        plan,
        Plan::Patch {
            base: Some("base".to_string())
        }
    );
}

#[test]
fn nothing_local_leaves_only_the_patch() {
    // Not even the merge commit was fetched.
    for json in [OPEN, MERGED] {
        let plan = plan(&info_of(json), |_| false, |_| None);
        assert_eq!(plan, Plan::Patch { base: None });
    }
}

#[test]
fn a_missing_gh_is_reported_with_how_to_get_it() {
    let dir = tempfile::tempdir().unwrap();
    let err = info("kimun-no-such-gh", dir.path(), 7)
        .unwrap_err()
        .to_string();
    assert!(err.contains("--pr requires the GitHub CLI"), "{err}");
    assert!(err.contains("`kimun-no-such-gh`"), "{err}");
    assert!(err.contains("https://cli.github.com"), "{err}");
}

#[cfg(unix)]
mod with_a_fake_gh {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    /// A program that answers as `gh` would: prints `stdout` and exits with
    /// `status`, after recording its arguments next to itself.
    fn fake(dir: &Path, stdout: &str, status: i32) -> PathBuf {
        let program = dir.join("fake-gh");
        fs::write(dir.join("stdout"), stdout).unwrap();
        fs::write(
            &program,
            format!(
                "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/args\"\n\
                 cat \"$(dirname \"$0\")/stdout\"\necho 'not logged in' >&2\nexit {status}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        program
    }

    #[test]
    fn info_asks_for_the_commits_of_the_pull_request() {
        let dir = tempfile::tempdir().unwrap();
        let gh = fake(dir.path(), MERGED, 0);

        let info = info(gh.to_str().unwrap(), dir.path(), 75).unwrap();

        assert_eq!(info.base_ref_oid, "base");
        assert_eq!(info.head_ref_oid, "head");
        assert_eq!(info.merge_commit.unwrap().oid, "merge");
        assert_eq!(
            fs::read_to_string(dir.path().join("args")).unwrap().trim(),
            "pr view 75 --json baseRefOid,headRefOid,mergeCommit"
        );
    }

    #[test]
    fn patch_returns_what_gh_prints() {
        let dir = tempfile::tempdir().unwrap();
        let gh = fake(dir.path(), "diff --git a/x b/x\n", 0);

        let patch = patch(gh.to_str().unwrap(), dir.path(), 12).unwrap();

        assert_eq!(patch, b"diff --git a/x b/x\n");
        assert_eq!(
            fs::read_to_string(dir.path().join("args")).unwrap().trim(),
            "pr diff 12 --color=never"
        );
    }

    #[test]
    fn a_failure_of_gh_carries_what_it_said() {
        let dir = tempfile::tempdir().unwrap();
        let gh = fake(dir.path(), "", 1);

        let err = info(gh.to_str().unwrap(), dir.path(), 75)
            .unwrap_err()
            .to_string();

        assert!(err.contains("pr view 75"), "{err}");
        assert!(err.contains("failed: not logged in"), "{err}");
    }

    #[test]
    fn an_answer_that_is_not_the_expected_json_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let gh = fake(dir.path(), "{\"unexpected\": true}", 0);

        let err = info(gh.to_str().unwrap(), dir.path(), 75)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unexpected answer"), "{err}");
    }
}
