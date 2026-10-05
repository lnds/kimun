//! Long help of `km impact`.

/// What the command measures, from which sources, and how to read it.
pub const HELP: &str = "\
Measure the blast radius of a change: the part of the system that can behave
differently because of it, beyond the files it edits, and which part of that
no test protects.

The radius is measured at two levels, each as a count over a total: the
projects of the repository that depend on the changed ones, and the source
files that call the functions that changed. Two more measures describe the
change itself: how spread it is, and which files usually change with it.

The change to measure comes from one of four sources:
  --since-ref REF              from where REF and HEAD diverged to the working
                               tree: committed, uncommitted and untracked
  --since-ref A --until-ref B  what B brings since it diverged from A,
                               whatever is checked out
  --diff FILE                  a patch in git format; `-` reads stdin
  --pr NUMBER                  a GitHub pull request. Requires the GitHub CLI
                               (gh) installed and authenticated

It is always the change over the whole repository: PATH only locates it.
Generated files (lock files, minified assets) are ignored.

Between two refs, manifests and files are read from the tree of B. A patch is
measured against the working tree, where it is not applied: a project it
creates is not known, and its files have unknown reach. With --diff,
--since-ref only tells where the history consulted ends.

A pull request is measured from its own commits when this repository has
them, as between two refs; one merged by squash or rebase, up to the commit
that merged it. Otherwise its patch is fetched with `gh pr diff` and measured
like any other patch.

Blast radius -- projects reached through their manifests:
  A project is a directory with a manifest (Cargo.toml, package.json,
  mix.exs). The projects that depend on a changed one, directly or through
  others, are reached. A dev or test dependency reaches the dependent and
  stops there. The manifest or lock file of a workspace root reaches every
  project under it. Other changed files outside every project are listed
  apart: their reach is unknown. In a repository with a single project this
  level does not apply.

  --affected prints the projects to build and test, one per line: changed
  and reached. If a changed file is outside every project, all projects are
  printed. It reads the diff and the manifests only, not the history.

Blast radius -- source files that use what changed:
  Inside the projects the change affects, the dependency graph is read
  backwards from the changed files. In Elixir the change is narrowed to the
  functions it touches, and each file that uses the module either calls one
  of them, refers to the module without calling it, or is left out. A file
  that calls what changed and that no test protects is reported: an
  integration test is probably missing. A test protects a file when it
  refers to it or sits at the same place (lib/a/b.ex, test/a/b_test.exs).
  Measured for Elixir, JavaScript/TypeScript and Kaikai.

Diffusion -- how spread the change is (Kamei et al., 2013):


  files, directories and subsystems (top-level directories) touched,
  lines added and deleted, and entropy: 0 when one file holds every
  modified line, 1 when all the files hold the same amount.

Logical radius -- files that usually change with the diff and are not in it
(Zimmermann et al., 2005):
  confidence = shared_commits / commits of the changed file
  A file is reported when a changed file reaches --min-confidence (0.5) with
  at least --min-shared (3) shared commits. History ends at the merge base:
  the commits of the diff are never evidence for themselves. Commits touching
  more than --max-changeset (30) files are ignored: a reformat or a rename
  across the project relates its files to each other by accident.

A changed file with no history (new, or outside --since) predicts nothing
and is listed apart. Test files are part of the analysis: a test that usually
changes with the code is a change worth expecting.

Requires a git repository. File renames are not tracked across history.

Examples:
  km impact --since-ref origin/main          # impact of the branch
  km impact --since-ref HEAD                 # impact of uncommitted work
  km impact --since-ref main --since 1y      # history of the last year only
  km impact --since-ref main --min-confidence 0.8
  km impact --since-ref main --max-changeset 50
  km impact --since-ref origin/main --affected   # projects to build and test
  km impact --since-ref main --until-ref feature # a branch, without checkout
  git diff main... | km impact --diff -             # a patch from stdin
  km impact --pr 123                             # a pull request (needs gh)
  km impact --since-ref main --format json   # everything, for tools and LLMs";
