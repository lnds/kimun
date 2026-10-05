use super::*;
use std::fs;

use git2::Repository;

const SIG_TIME: i64 = 1_700_000_000;

fn create_test_repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    (dir, repo)
}

fn write(repo: &Repository, path: &str, content: &str) {
    let full_path = repo.workdir().unwrap().join(path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full_path, content).unwrap();
}

/// Commit `files` as written, and `removed` as deletions.
fn commit(repo: &Repository, files: &[(&str, &str)], removed: &[&str]) {
    let sig = git2::Signature::new("Test", "test@test.com", &git2::Time::new(SIG_TIME, 0)).unwrap();
    let mut index = repo.index().unwrap();
    for (path, content) in files {
        write(repo, path, content);
        index.add_path(Path::new(path)).unwrap();
    }
    for path in removed {
        fs::remove_file(repo.workdir().unwrap().join(path)).unwrap();
        index.remove_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &parents)
        .unwrap();
}

/// Commit `paths` together `times` times, with new content each time.
fn co_change(repo: &Repository, paths: &[&str], times: usize) {
    for i in 0..times {
        let content = format!("line {i}\n").repeat(i + 1);
        let files: Vec<(&str, &str)> = paths.iter().map(|p| (*p, content.as_str())).collect();
        commit(repo, &files, &[]);
    }
}

fn opts(since_ref: &str) -> ImpactOptions {
    ImpactOptions {
        output: OutputMode::Table,
        source: DiffSource::Refs {
            since: since_ref.to_string(),
            until: None,
        },
        since: None,
        min_confidence: 0.5,
        min_shared: 3,
        max_changeset: 30,
        top: 20,
        affected: false,
    }
}

fn missing_paths(impact: &Impact) -> Vec<&Path> {
    impact.missing.iter().map(|m| m.path.as_path()).collect()
}

#[test]
fn fails_outside_a_git_repository() {
    let dir = tempfile::tempdir().unwrap();
    let err = run(dir.path(), &opts("HEAD")).unwrap_err();
    assert!(
        err.to_string().contains("not a git repository"),
        "got: {err}"
    );
}

#[test]
fn fails_on_an_unknown_ref() {
    let (dir, repo) = create_test_repo();
    commit(&repo, &[("a.rs", "a\n")], &[]);
    let err = run(dir.path(), &opts("no-such-ref")).unwrap_err();
    assert!(
        err.to_string().contains("cannot resolve ref 'no-such-ref'"),
        "got: {err}"
    );
}

#[test]
fn rejects_thresholds_out_of_range() {
    let (dir, repo) = create_test_repo();
    commit(&repo, &[("a.rs", "a\n")], &[]);

    for bad in [0.0, -0.1, 1.5, f64::NAN] {
        let o = ImpactOptions {
            min_confidence: bad,
            ..opts("HEAD")
        };
        let err = run(dir.path(), &o).unwrap_err();
        assert!(err.to_string().contains("--min-confidence"), "got: {err}");
    }

    let o = ImpactOptions {
        min_shared: 0,
        ..opts("HEAD")
    };
    let err = run(dir.path(), &o).unwrap_err();
    assert!(err.to_string().contains("--min-shared"), "got: {err}");
}

#[test]
fn rejects_an_invalid_since() {
    let (dir, repo) = create_test_repo();
    commit(&repo, &[("a.rs", "a\n")], &[]);
    let o = ImpactOptions {
        since: Some("soon".to_string()),
        ..opts("HEAD")
    };
    assert!(run(dir.path(), &o).is_err());
}

#[test]
fn clean_tree_has_no_impact() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["a.rs", "b.rs"], 3);

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert_eq!(impact.diffusion.files, 0);
    assert!(impact.missing.is_empty());
    assert!(impact.without_history.is_empty());
}

#[test]
fn reports_the_file_left_out_of_an_uncommitted_change() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["src/a.rs", "src/b.rs"], 4);
    write(&repo, "src/a.rs", "changed\nagain\n");

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();

    assert_eq!(impact.diffusion.files, 1);
    assert_eq!(missing_paths(&impact), [Path::new("src/b.rs")]);
    let best = impact.missing[0].best();
    assert_eq!(best.path, PathBuf::from("src/a.rs"));
    assert_eq!((best.shared_commits, best.commits), (4, 4));
    assert!(impact.without_history.is_empty());
}

#[test]
fn a_file_changed_along_is_not_missing() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["a.rs", "b.rs"], 4);
    write(&repo, "a.rs", "changed\n");
    write(&repo, "b.rs", "changed too\n");

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert_eq!(impact.diffusion.files, 2);
    assert!(impact.missing.is_empty(), "{:?}", missing_paths(&impact));
}

#[test]
fn commits_of_the_diff_are_not_evidence() {
    let (dir, repo) = create_test_repo();
    commit(
        &repo,
        &[("a.rs", "a\n"), ("b.rs", "b\n"), ("c.rs", "c\n")],
        &[],
    );
    let base = repo.head().unwrap().peel_to_commit().unwrap().id();
    // The branch itself changes a.rs and b.rs together, over and over.
    co_change(&repo, &["a.rs", "b.rs"], 5);
    // Restore b.rs so that only a.rs differs from the base.
    commit(&repo, &[("b.rs", "b\n")], &[]);

    let impact = analyze(dir.path(), &opts(&base.to_string())).unwrap();

    assert_eq!(impact.diffusion.files, 1);
    // Before the base a.rs has a single commit: too little to expect anything.
    assert!(impact.missing.is_empty(), "{:?}", missing_paths(&impact));
}

#[test]
fn diff_starts_at_the_merge_base_of_a_diverged_ref() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["a.rs", "b.rs"], 3);
    let base = repo.head().unwrap().peel_to_commit().unwrap();

    // The other line of history moves on after the branch point.
    commit(&repo, &[("upstream.rs", "new\n")], &[]);
    let upstream = repo.head().unwrap().peel_to_commit().unwrap().id();
    repo.reference("refs/heads/upstream", upstream, true, "test")
        .unwrap();

    // Back to the branch point, where the branch makes its own change.
    repo.reset(base.as_object(), git2::ResetType::Hard, None)
        .unwrap();
    commit(&repo, &[("a.rs", "branch work\n")], &[]);

    let impact = analyze(dir.path(), &opts("upstream")).unwrap();

    // upstream.rs is not on this branch; it is not a deletion made by it.
    assert_eq!(impact.diffusion.files, 1);
    assert_eq!(missing_paths(&impact), [Path::new("b.rs")]);
}

#[test]
fn new_files_are_listed_as_without_history() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["a.rs", "b.rs"], 3);
    write(&repo, "src/new.rs", "one\ntwo\nthree\n");

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();

    assert_eq!(impact.diffusion.files, 1);
    // The content of an untracked file counts as added lines.
    assert_eq!(impact.diffusion.lines_added, 3);
    assert_eq!(impact.without_history, [PathBuf::from("src/new.rs")]);
    assert!(impact.missing.is_empty());
}

#[test]
fn a_changed_file_outside_the_since_window_has_no_history() {
    let (dir, repo) = create_test_repo();
    // Every commit is dated 2023, far before a one-day window.
    co_change(&repo, &["a.rs", "b.rs"], 4);
    write(&repo, "a.rs", "changed\n");

    let o = ImpactOptions {
        since: Some("1d".to_string()),
        ..opts("HEAD")
    };
    let impact = analyze(dir.path(), &o).unwrap();

    assert!(impact.missing.is_empty());
    assert_eq!(impact.without_history, [PathBuf::from("a.rs")]);
}

#[test]
fn a_file_that_no_longer_exists_is_not_missing() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["a.rs", "gone.rs", "kept.rs"], 4);
    commit(&repo, &[], &["gone.rs"]);
    write(&repo, "a.rs", "changed\n");

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert_eq!(missing_paths(&impact), [Path::new("kept.rs")]);
}

#[test]
fn a_deleted_file_counts_and_still_predicts() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["a.rs", "b.rs"], 4);
    fs::remove_file(dir.path().join("a.rs")).unwrap();

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();

    assert_eq!(impact.diffusion.files, 1);
    assert_eq!(impact.diffusion.lines_added, 0);
    assert!(impact.diffusion.lines_deleted > 0);
    assert_eq!(missing_paths(&impact), [Path::new("b.rs")]);
}

#[test]
fn a_renamed_file_keeps_the_history_of_its_old_path() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["old.rs", "b.rs"], 4);
    fs::rename(dir.path().join("old.rs"), dir.path().join("new.rs")).unwrap();

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();

    assert_eq!(impact.diffusion.files, 1);
    assert_eq!(missing_paths(&impact), [Path::new("b.rs")]);
    assert_eq!(impact.missing[0].best().path, PathBuf::from("new.rs"));
    assert!(impact.without_history.is_empty());
}

#[test]
fn generated_files_are_left_out_of_every_measure() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["mix.exs", "mix.lock", "lib/app.ex"], 4);
    write(&repo, "mix.exs", "changed\n");

    // mix.lock always changed with mix.exs, and is not reported.
    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert_eq!(missing_paths(&impact), [Path::new("lib/app.ex")]);
    assert_eq!(impact.generated_skipped, 0);

    // Once in the diff, it is counted apart from the changed files.
    write(&repo, "mix.lock", "changed\n");
    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert_eq!(impact.diffusion.files, 1);
    assert_eq!(impact.generated_skipped, 1);
}

#[test]
fn test_files_are_expected_like_any_other() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["lib/parser.ex", "test/parser_test.exs"], 4);
    write(&repo, "lib/parser.ex", "changed\n");

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert_eq!(missing_paths(&impact), [Path::new("test/parser_test.exs")]);
}

#[test]
fn diffusion_spans_directories_and_subsystems() {
    let (dir, repo) = create_test_repo();
    commit(
        &repo,
        &[
            ("lib/a/one.ex", "1\n"),
            ("lib/b/two.ex", "2\n"),
            ("test/one_test.exs", "3\n"),
            ("README.md", "4\n"),
        ],
        &[],
    );
    write(&repo, "lib/a/one.ex", "1\nx\n");
    write(&repo, "lib/b/two.ex", "2\nx\n");
    write(&repo, "test/one_test.exs", "3\nx\n");
    write(&repo, "README.md", "4\nx\n");

    let d = analyze(dir.path(), &opts("HEAD")).unwrap().diffusion;
    assert_eq!((d.files, d.directories, d.subsystems), (4, 4, 3));
    assert_eq!((d.lines_added, d.lines_deleted), (4, 0));
    assert!((d.entropy - 1.0).abs() < 1e-9, "got {}", d.entropy);
}

#[test]
fn runs_in_every_supported_format() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["a.rs", "b.rs"], 4);
    write(&repo, "a.rs", "changed\n");

    for output in [
        OutputMode::Table,
        OutputMode::Json,
        OutputMode::Short,
        OutputMode::Terse,
    ] {
        let o = ImpactOptions {
            output,
            ..opts("HEAD")
        };
        run(dir.path(), &o).unwrap();
    }
}

#[test]
fn ci_formats_are_rejected() {
    let (dir, repo) = create_test_repo();
    commit(&repo, &[("a.rs", "a\n")], &[]);

    for output in [OutputMode::Github, OutputMode::Codeclimate] {
        let o = ImpactOptions {
            output,
            ..opts("HEAD")
        };
        let err = run(dir.path(), &o).unwrap_err();
        assert_eq!(err.to_string(), crate::cli::ERR_CI_FORMAT_ONLY);
    }
}

#[test]
fn sweeping_commits_are_not_evidence() {
    let (dir, repo) = create_test_repo();
    // a.rs and b.rs only ever met in commits that touch the whole project.
    let content: Vec<String> = (0..4).map(|i| format!("v{i}\n")).collect();
    for text in &content {
        commit(
            &repo,
            &[
                ("a.rs", text),
                ("b.rs", text),
                ("c.rs", text),
                ("d.rs", text),
            ],
            &[],
        );
    }
    write(&repo, "a.rs", "changed\n");

    let narrow = ImpactOptions {
        max_changeset: 3,
        ..opts("HEAD")
    };
    let impact = analyze(dir.path(), &narrow).unwrap();
    assert!(impact.missing.is_empty(), "{:?}", missing_paths(&impact));
    assert_eq!(impact.skipped_commits, 4);
    // Every commit of a.rs was set aside, so it has no history left.
    assert_eq!(impact.without_history, [PathBuf::from("a.rs")]);

    // At the limit the commits count.
    let wide = ImpactOptions {
        max_changeset: 4,
        ..opts("HEAD")
    };
    let impact = analyze(dir.path(), &wide).unwrap();
    assert_eq!(impact.missing.len(), 3);
    assert_eq!(impact.skipped_commits, 0);
}

#[test]
fn rejects_a_zero_max_changeset() {
    let (dir, repo) = create_test_repo();
    commit(&repo, &[("a.rs", "a\n")], &[]);
    let o = ImpactOptions {
        max_changeset: 0,
        ..opts("HEAD")
    };
    let err = run(dir.path(), &o).unwrap_err();
    assert!(err.to_string().contains("--max-changeset"), "got: {err}");
}

#[test]
fn full_confidence_is_a_valid_threshold() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["a.rs", "always.rs"], 4);
    commit(&repo, &[("a.rs", "alone\n")], &[]);
    co_change(&repo, &["b.rs", "always.rs"], 3);
    write(&repo, "a.rs", "changed\n");
    write(&repo, "b.rs", "changed\n");

    let o = ImpactOptions {
        min_confidence: 1.0,
        ..opts("HEAD")
    };
    let impact = analyze(dir.path(), &o).unwrap();

    // always.rs followed b.rs in 3 of 3 commits and a.rs in 4 of 5.
    assert_eq!(missing_paths(&impact), [Path::new("always.rs")]);
    let triggers: Vec<&Path> = impact.missing[0]
        .triggers
        .iter()
        .map(|t| t.path.as_path())
        .collect();
    assert_eq!(triggers, [Path::new("b.rs")]);
}

#[test]
fn projects_reached_by_the_diff() {
    let (dir, repo) = create_test_repo();
    commit(
        &repo,
        &[
            ("libs/core/Cargo.toml", "[package]\nname = \"core\"\n"),
            ("libs/core/src/lib.rs", "pub fn f() {}\n"),
            (
                "apps/api/Cargo.toml",
                "[package]\nname = \"api\"\n\n[dependencies]\ncore = { path = \"../../libs/core\" }\n",
            ),
            ("apps/api/src/main.rs", "fn main() {}\n"),
            ("apps/site/Cargo.toml", "[package]\nname = \"site\"\n"),
            ("README.md", "readme\n"),
        ],
        &[],
    );
    write(&repo, "libs/core/src/lib.rs", "pub fn f() -> u8 { 1 }\n");
    write(&repo, "README.md", "changed\n");

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();

    assert_eq!(impact.projects.total(), 3);
    assert_eq!(impact.projects.changed, ["libs/core"]);
    let reached: Vec<&str> = impact
        .projects
        .reached
        .iter()
        .map(|r| r.project.as_str())
        .collect();
    assert_eq!(reached, ["apps/api"]);
    // Documentation reaches nothing: it does not make the reach unknown.
    assert!(impact.projects.outside.is_empty());
    assert_eq!(impact.projects.inert, [PathBuf::from("README.md")]);
    assert_eq!(impact.projects.affected(), ["apps/api", "libs/core"]);

    // A file that is not documentation and belongs to no project does.
    write(&repo, "Makefile", "all:\n");
    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert_eq!(impact.projects.outside, [PathBuf::from("Makefile")]);
    assert_eq!(
        impact.projects.affected(),
        ["apps/api", "apps/site", "libs/core"]
    );
    fs::remove_file(dir.path().join("Makefile")).unwrap();

    // The repository is found from a directory inside it, and measured whole.
    let from_inside = analyze(&dir.path().join("apps/site"), &opts("HEAD")).unwrap();
    assert_eq!(from_inside.projects.changed, ["libs/core"]);
    assert_eq!(from_inside.projects.reached.len(), 1);

    let o = ImpactOptions {
        affected: true,
        ..opts("HEAD")
    };
    run(dir.path(), &o).unwrap();
}

#[test]
fn a_renamed_file_changes_the_project_it_left() {
    let (dir, repo) = create_test_repo();
    commit(
        &repo,
        &[
            ("a/Cargo.toml", "[package]\nname = \"a\"\n"),
            ("a/src/moved.rs", "one\ntwo\nthree\nfour\nfive\n"),
            ("b/Cargo.toml", "[package]\nname = \"b\"\n"),
        ],
        &[],
    );
    fs::create_dir_all(dir.path().join("b/src")).unwrap();
    fs::rename(
        dir.path().join("a/src/moved.rs"),
        dir.path().join("b/src/moved.rs"),
    )
    .unwrap();

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert_eq!(impact.projects.changed, ["a", "b"]);
}

#[test]
fn between_two_refs_the_tree_of_the_second_decides_what_exists() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, &["a.rs", "kept.rs", "dropped.rs"], 4);
    let base = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    // The change edits a.rs and deletes dropped.rs along the way... in a
    // later commit that is not part of what is measured.
    commit(&repo, &[("a.rs", "changed\n")], &[]);
    let until = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    commit(&repo, &[], &["kept.rs"]);

    let o = ImpactOptions {
        source: DiffSource::Refs {
            since: base,
            until: Some(until),
        },
        ..opts("HEAD")
    };
    let impact = analyze(dir.path(), &o).unwrap();

    assert_eq!(impact.diffusion.files, 1);
    // kept.rs is gone from the working tree, but it is in the measured tree.
    assert_eq!(
        missing_paths(&impact),
        [Path::new("dropped.rs"), Path::new("kept.rs")]
    );
    assert!(impact.source.starts_with("diff "));
    assert!(impact.source.contains("..."));
}

#[test]
fn a_patch_reaches_projects_like_any_other_change() {
    let (dir, repo) = create_test_repo();
    commit(
        &repo,
        &[
            ("libs/core/Cargo.toml", "[package]\nname = \"core\"\n"),
            ("libs/core/src/lib.rs", "pub fn f() {}\n"),
            (
                "apps/api/Cargo.toml",
                "[package]\nname = \"api\"\n\n[dependencies]\ncore = { path = \"../../libs/core\" }\n",
            ),
        ],
        &[],
    );
    let patch = dir.path().join("change.patch");
    fs::write(
        &patch,
        "diff --git a/libs/core/src/lib.rs b/libs/core/src/lib.rs\n\
         index 1111111..2222222 100644\n\
         --- a/libs/core/src/lib.rs\n\
         +++ b/libs/core/src/lib.rs\n\
         @@ -1 +1 @@\n\
         -pub fn f() {}\n\
         +pub fn f() -> u8 { 1 }\n",
    )
    .unwrap();

    let o = ImpactOptions {
        source: DiffSource::Patch {
            file: patch,
            base: None,
        },
        ..opts("HEAD")
    };
    let impact = analyze(dir.path(), &o).unwrap();

    assert_eq!(impact.projects.changed, ["libs/core"]);
    assert_eq!(impact.projects.affected(), ["apps/api", "libs/core"]);
    assert_eq!(
        (impact.diffusion.lines_added, impact.diffusion.lines_deleted),
        (1, 1)
    );

    // The same source serves --affected.
    let affected = ImpactOptions {
        affected: true,
        ..o
    };
    run(dir.path(), &affected).unwrap();
}

mod command_line {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn parse(args: &[&str]) -> Result<ImpactOptions, String> {
        let argv = ["km", "impact"].iter().chain(args).copied();
        match Cli::try_parse_from(argv) {
            Ok(Cli {
                command: Commands::Impact(args),
            }) => Ok(options(&args)),
            Ok(_) => Err("not the impact command".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn source(args: &[&str]) -> DiffSource {
        parse(args).unwrap().source
    }

    #[test]
    fn a_ref_alone_measures_up_to_the_working_tree() {
        assert_eq!(
            source(&["--since-ref", "main"]),
            DiffSource::Refs {
                since: "main".to_string(),
                until: None
            }
        );
    }

    #[test]
    fn two_refs_measure_between_them() {
        assert_eq!(
            source(&["--since-ref", "main", "--until-ref", "feature"]),
            DiffSource::Refs {
                since: "main".to_string(),
                until: Some("feature".to_string())
            }
        );
    }

    #[test]
    fn a_patch_with_or_without_a_base() {
        assert_eq!(
            source(&["--diff", "-"]),
            DiffSource::Patch {
                file: PathBuf::from("-"),
                base: None
            }
        );
        assert_eq!(
            source(&["--diff", "change.patch", "--since-ref", "main"]),
            DiffSource::Patch {
                file: PathBuf::from("change.patch"),
                base: Some("main".to_string())
            }
        );
    }

    #[test]
    fn a_pull_request_by_number() {
        assert_eq!(source(&["--pr", "75"]), DiffSource::PullRequest(75));
    }

    #[test]
    fn the_other_options_are_carried_over() {
        let o = parse(&[
            "--since-ref",
            "main",
            "--since",
            "6m",
            "--min-confidence",
            "0.8",
            "--min-shared",
            "5",
            "--max-changeset",
            "50",
            "--top",
            "7",
            "--affected",
            "--format",
            "json",
        ])
        .unwrap();
        assert_eq!(o.since.as_deref(), Some("6m"));
        assert_eq!(o.min_confidence, 0.8);
        assert_eq!((o.min_shared, o.max_changeset, o.top), (5, 50, 7));
        assert!(o.affected);
        assert_eq!(o.output, OutputMode::Json);
    }

    #[test]
    fn a_change_must_be_named() {
        let err = parse(&[]).unwrap_err();
        assert!(err.contains("--since-ref"), "{err}");
    }

    #[test]
    fn sources_that_do_not_mix_are_refused() {
        for args in [
            &["--pr", "1", "--since-ref", "main"][..],
            &["--pr", "1", "--diff", "x.patch"],
            &["--pr", "1", "--since-ref", "main", "--until-ref", "f"],
            &[
                "--diff",
                "x.patch",
                "--since-ref",
                "main",
                "--until-ref",
                "f",
            ],
        ] {
            let err = parse(args).unwrap_err();
            assert!(err.contains("cannot be used with"), "{args:?}: {err}");
        }
        // Without a starting ref there is nothing for --until-ref to bound.
        let err = parse(&["--until-ref", "feature"]).unwrap_err();
        assert!(err.contains("--since-ref"), "{err}");
        assert!(parse(&["--pr", "abc"]).is_err());
    }
}

/// Two crates, `api` depending on `core`, with documentation inside each.
fn two_crates(repo: &Repository) {
    commit(
        repo,
        &[
            ("libs/core/Cargo.toml", "[package]\nname = \"core\"\n"),
            ("libs/core/README.md", "core\n"),
            ("libs/core/src/lib.rs", "pub fn f() {}\n"),
            (
                "apps/api/Cargo.toml",
                "[package]\nname = \"api\"\n\n[dependencies]\ncore = { path = \"../../libs/core\" }\n",
            ),
            ("scripts/release.sh", "echo release\n"),
        ],
        &[],
    );
}

#[test]
fn documentation_inside_a_project_does_not_change_it() {
    let (dir, repo) = create_test_repo();
    two_crates(&repo);
    write(&repo, "libs/core/README.md", "core, explained\n");

    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();

    assert!(impact.projects.changed.is_empty());
    assert!(impact.projects.reached.is_empty());
    assert_eq!(
        impact.projects.inert,
        [PathBuf::from("libs/core/README.md")]
    );
    assert!(impact.projects.affected().is_empty());
    // It is still part of the change.
    assert_eq!(impact.diffusion.files, 1);
}

#[test]
fn the_configuration_declares_what_else_is_inert() {
    let (dir, repo) = create_test_repo();
    two_crates(&repo);
    write(&repo, "scripts/release.sh", "echo released\n");

    // By default a script outside every project has unknown reach.
    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert_eq!(
        impact.projects.outside,
        [PathBuf::from("scripts/release.sh")]
    );
    assert_eq!(impact.projects.affected(), ["apps/api", "libs/core"]);

    write(
        &repo,
        ".kimun.toml",
        "[impact]\ninert = [\"scripts/**\", \".kimun.toml\"]\n",
    );
    let impact = analyze(dir.path(), &opts("HEAD")).unwrap();
    assert!(impact.projects.outside.is_empty());
    assert_eq!(
        impact.projects.inert,
        [
            PathBuf::from(".kimun.toml"),
            PathBuf::from("scripts/release.sh")
        ]
    );
    assert!(impact.projects.affected().is_empty());
}

#[test]
fn an_invalid_inert_pattern_stops_the_analysis() {
    let (dir, repo) = create_test_repo();
    two_crates(&repo);
    write(&repo, ".kimun.toml", "[impact]\ninert = [\"a/[\"]\n");

    let err = analyze(dir.path(), &opts("HEAD"))
        .err()
        .unwrap()
        .to_string();
    assert!(err.contains("[impact] inert"), "{err}");
}
