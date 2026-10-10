use super::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// A directory holding `files`, each given as a path and its content.
fn tree(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, content).unwrap();
    }
    dir
}

fn discover(files: &[(&str, &str)]) -> ProjectGraph {
    ProjectGraph::discover(tree(files).path())
}

fn cargo(name: &str, deps: &str) -> String {
    format!("[package]\nname = \"{name}\"\n\n{deps}\n")
}

fn mix(app: &str, deps: &str) -> String {
    format!("def project, do: [app: :{app}, deps: deps()]\ndefp deps, do: [{deps}]\n")
}

fn roots(graph: &ProjectGraph) -> Vec<String> {
    graph.projects.iter().map(Project::display_root).collect()
}

/// Edges as `(dependent, dependency, scope)`, by project root.
fn edges(graph: &ProjectGraph) -> Vec<(String, String, Scope)> {
    let root = |i: usize| graph.projects[i].display_root();
    let mut edges: Vec<_> = graph
        .edges
        .iter()
        .map(|e| (root(e.from), root(e.to), e.scope))
        .collect();
    edges.sort();
    edges
}

fn edge(from: &str, to: &str, scope: Scope) -> (String, String, Scope) {
    (from.to_string(), to.to_string(), scope)
}

fn index(graph: &ProjectGraph, root: &str) -> usize {
    graph
        .projects
        .iter()
        .position(|p| p.display_root() == root)
        .unwrap_or_else(|| panic!("no project at {root}"))
}

/// The radius of a change to `changed`, as `(project, origin, distance, via, scope)`.
fn radius(graph: &ProjectGraph, changed: &[&str]) -> Vec<(String, String, usize, String, Scope)> {
    let changed: BTreeSet<usize> = changed.iter().map(|r| index(graph, r)).collect();
    let root = |i: usize| graph.projects[i].display_root();
    graph
        .radius(&changed)
        .into_iter()
        .map(|r| {
            (
                root(r.project),
                root(r.origin),
                r.distance,
                root(r.via),
                r.scope,
            )
        })
        .collect()
}

fn reach(
    project: &str,
    origin: &str,
    distance: usize,
    via: &str,
    scope: Scope,
) -> (String, String, usize, String, Scope) {
    (
        project.to_string(),
        origin.to_string(),
        distance,
        via.to_string(),
        scope,
    )
}

#[test]
fn cargo_workspace_with_inherited_dependencies() {
    let graph = discover(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.dependencies]\ncore = { path = \"crates/core\" }\nserde = \"1\"\n",
        ),
        ("crates/core/Cargo.toml", &cargo("core", "")),
        (
            "crates/api/Cargo.toml",
            &cargo(
                "api",
                "[dependencies]\ncore = { workspace = true }\nserde = { workspace = true }",
            ),
        ),
        (
            "crates/cli/Cargo.toml",
            &cargo(
                "cli",
                "[dependencies]\napi = { path = \"../api\" }\n\n[dev-dependencies]\ncore.workspace = true",
            ),
        ),
    ]);

    // The virtual root is not a project.
    assert_eq!(roots(&graph), ["crates/api", "crates/cli", "crates/core"]);
    assert_eq!(
        edges(&graph),
        [
            edge("crates/api", "crates/core", Scope::Runtime),
            edge("crates/cli", "crates/api", Scope::Runtime),
            edge("crates/cli", "crates/core", Scope::Dev),
        ]
    );
    assert!(graph.unread.is_empty());
}

#[test]
fn npm_packages_are_linked_by_name_whatever_the_range() {
    let graph = discover(&[
        (
            "package.json",
            r#"{ "private": true, "workspaces": ["packages/*"] }"#,
        ),
        ("packages/ui/package.json", r#"{ "name": "@acme/ui" }"#),
        (
            "packages/web/package.json",
            r#"{ "name": "@acme/web", "dependencies": { "@acme/ui": "^1.0.0", "react": "^18" },
                 "devDependencies": { "@acme/lint": "workspace:*" } }"#,
        ),
        ("packages/lint/package.json", r#"{ "name": "@acme/lint" }"#),
    ]);

    // The root only declares the workspace: it is not a project.
    assert_eq!(
        roots(&graph),
        ["packages/lint", "packages/ui", "packages/web"]
    );
    assert_eq!(graph.workspaces, [PathBuf::from("")]);
    assert_eq!(
        edges(&graph),
        [
            edge("packages/web", "packages/lint", Scope::Dev),
            edge("packages/web", "packages/ui", Scope::Runtime),
        ]
    );
    // `react` is on a registry: neither an edge nor a dependency left unread.
    assert!(graph.unread.is_empty());
}

#[test]
fn mix_projects_linked_by_path_and_by_umbrella() {
    let graph = discover(&[
        ("libs/core/mix.exs", &mix("core", "")),
        ("libs/helpers/mix.exs", &mix("helpers", "")),
        (
            "apps/booking/mix.exs",
            &mix(
                "booking",
                r#"{:core, path: "../../libs/core"}, {:helpers, path: "../../libs/helpers", only: :test}"#,
            ),
        ),
        (
            "umbrella/mix.exs",
            "def project, do: [apps_path: \"apps\"]\n",
        ),
        ("umbrella/apps/store/mix.exs", &mix("store", "")),
        (
            "umbrella/apps/web/mix.exs",
            &mix("web", "{:store, in_umbrella: true}"),
        ),
    ]);

    assert_eq!(
        roots(&graph),
        [
            "apps/booking",
            "libs/core",
            "libs/helpers",
            "umbrella/apps/store",
            "umbrella/apps/web",
        ]
    );
    assert_eq!(
        edges(&graph),
        [
            edge("apps/booking", "libs/core", Scope::Runtime),
            edge("apps/booking", "libs/helpers", Scope::Dev),
            edge("umbrella/apps/web", "umbrella/apps/store", Scope::Runtime),
        ]
    );
}

#[test]
fn fetched_dependencies_and_build_output_are_not_projects() {
    let graph = discover(&[
        (
            "package.json",
            r#"{ "name": "app", "dependencies": { "left-pad": "1" } }"#,
        ),
        (
            "node_modules/left-pad/package.json",
            r#"{ "name": "left-pad" }"#,
        ),
        ("mix.exs", &mix("app", "")),
        ("deps/phoenix/mix.exs", &mix("phoenix", "")),
        ("_build/dev/lib/app/mix.exs", &mix("built", "")),
        ("target/package/app/Cargo.toml", &cargo("packaged", "")),
        ("vendor/gem/package.json", r#"{ "name": "vendored" }"#),
    ]);

    assert_eq!(roots(&graph), ["."]);
    assert!(graph.edges.is_empty());
}

#[test]
fn a_directory_with_two_manifests_is_one_project() {
    let graph = discover(&[
        (
            "app/Cargo.toml",
            &cargo("app", "[dependencies]\ncore = { path = \"../core\" }"),
        ),
        (
            "app/package.json",
            r#"{ "name": "app-js", "dependencies": { "ui": "*" } }"#,
        ),
        ("core/Cargo.toml", &cargo("core", "")),
        ("ui/package.json", r#"{ "name": "ui" }"#),
    ]);

    assert_eq!(roots(&graph), ["app", "core", "ui"]);
    assert_eq!(graph.projects[index(&graph, "app")].name, "app");
    assert_eq!(
        edges(&graph),
        [
            edge("app", "core", Scope::Runtime),
            edge("app", "ui", Scope::Runtime),
        ]
    );
}

#[test]
fn names_do_not_cross_ecosystems() {
    let graph = discover(&[
        (
            "js/package.json",
            r#"{ "name": "web", "dependencies": { "core": "*" } }"#,
        ),
        ("rs/Cargo.toml", &cargo("core", "")),
    ]);
    assert!(graph.edges.is_empty());
}

#[test]
fn the_strongest_scope_of_a_repeated_dependency_wins() {
    let graph = discover(&[
        (
            "app/Cargo.toml",
            &cargo(
                "app",
                "[dev-dependencies]\ncore = { path = \"../core\" }\n\n[build-dependencies]\ncore = { path = \"../core\" }",
            ),
        ),
        ("core/Cargo.toml", &cargo("core", "")),
    ]);
    assert_eq!(edges(&graph), [edge("app", "core", Scope::Build)]);
}

#[test]
fn local_dependencies_pointing_at_no_project_are_reported() {
    let graph = discover(&[
        (
            "app/Cargo.toml",
            &cargo(
                "app",
                "[dependencies]\ngone = { path = \"../gone\" }\noutside = { path = \"../../elsewhere\" }\nself_ref = { path = \".\" }",
            ),
        ),
        (
            "web/mix.exs",
            &mix("web", r#"{:dyn, path: Path.expand("../dyn", __DIR__)}"#),
        ),
    ]);

    assert!(graph.edges.is_empty());
    assert_eq!(
        graph.unread,
        [
            (PathBuf::from("app/Cargo.toml"), 2),
            (PathBuf::from("web/mix.exs"), 1),
        ]
    );
}

#[test]
fn a_file_belongs_to_the_nearest_project_above_it() {
    let graph = discover(&[
        ("apps/web/mix.exs", &mix("web", "")),
        ("apps/web/assets/package.json", r#"{ "name": "assets" }"#),
    ]);
    let owner = |file: &str| {
        graph
            .owner(Path::new(file))
            .map(|i| graph.projects[i].display_root())
    };

    assert_eq!(owner("apps/web/lib/web.ex").as_deref(), Some("apps/web"));
    assert_eq!(owner("apps/web/mix.exs").as_deref(), Some("apps/web"));
    assert_eq!(
        owner("apps/web/assets/js/app.js").as_deref(),
        Some("apps/web/assets")
    );
    assert_eq!(owner("apps/README.md"), None);
    assert_eq!(owner(".github/workflows/ci.yml"), None);
    // A nested project has no edge to the one that contains it.
    assert!(graph.edges.is_empty());
}

#[test]
fn a_project_at_the_root_owns_what_no_other_does() {
    let graph = discover(&[
        ("Cargo.toml", &cargo("root", "")),
        ("sub/Cargo.toml", &cargo("sub", "")),
    ]);
    let owner = |file: &str| {
        graph
            .owner(Path::new(file))
            .map(|i| graph.projects[i].display_root())
    };

    assert_eq!(owner("src/main.rs").as_deref(), Some("."));
    assert_eq!(owner("README.md").as_deref(), Some("."));
    assert_eq!(owner("sub/src/lib.rs").as_deref(), Some("sub"));
}

/// a ← b ← c ← d, each depending on the previous one.
fn chain() -> ProjectGraph {
    discover(&[
        ("a/Cargo.toml", &cargo("a", "")),
        (
            "b/Cargo.toml",
            &cargo("b", "[dependencies]\na = { path = \"../a\" }"),
        ),
        (
            "c/Cargo.toml",
            &cargo("c", "[dependencies]\nb = { path = \"../b\" }"),
        ),
        (
            "d/Cargo.toml",
            &cargo("d", "[dependencies]\nc = { path = \"../c\" }"),
        ),
        ("lone/Cargo.toml", &cargo("lone", "")),
    ])
}

#[test]
fn radius_follows_dependents_by_distance() {
    let graph = chain();
    assert_eq!(
        radius(&graph, &["a"]),
        [
            reach("b", "a", 1, "a", Scope::Runtime),
            reach("c", "a", 2, "b", Scope::Runtime),
            reach("d", "a", 3, "c", Scope::Runtime),
        ]
    );
    // What the changed project depends on is not reached.
    assert_eq!(radius(&graph, &["d"]), []);
    assert_eq!(radius(&graph, &["lone"]), []);
    assert_eq!(radius(&graph, &[]), []);
}

#[test]
fn a_changed_project_is_not_reached_by_another() {
    let graph = chain();
    assert_eq!(
        radius(&graph, &["a", "c"]),
        [
            reach("b", "a", 1, "a", Scope::Runtime),
            reach("d", "c", 1, "c", Scope::Runtime),
        ]
    );
}

#[test]
fn a_dev_dependency_reaches_the_dependent_and_stops() {
    let graph = discover(&[
        ("helpers/Cargo.toml", &cargo("helpers", "")),
        (
            "lib/Cargo.toml",
            &cargo(
                "lib",
                "[dev-dependencies]\nhelpers = { path = \"../helpers\" }",
            ),
        ),
        (
            "app/Cargo.toml",
            &cargo("app", "[dependencies]\nlib = { path = \"../lib\" }"),
        ),
    ]);

    // `lib` tests with `helpers`; `app` ships `lib`, not `helpers`.
    assert_eq!(
        radius(&graph, &["helpers"]),
        [reach("lib", "helpers", 1, "helpers", Scope::Dev)]
    );
}

#[test]
fn a_shipping_path_wins_over_a_shorter_dev_one() {
    let graph = discover(&[
        ("core/Cargo.toml", &cargo("core", "")),
        (
            "mid/Cargo.toml",
            &cargo("mid", "[dependencies]\ncore = { path = \"../core\" }"),
        ),
        (
            "app/Cargo.toml",
            &cargo(
                "app",
                "[dependencies]\nmid = { path = \"../mid\" }\n\n[dev-dependencies]\ncore = { path = \"../core\" }",
            ),
        ),
        (
            "top/Cargo.toml",
            &cargo("top", "[dependencies]\napp = { path = \"../app\" }"),
        ),
    ]);

    // `app` uses `core` directly only in its tests, but ships it through
    // `mid`, so the reach goes on to `top`.
    assert_eq!(
        radius(&graph, &["core"]),
        [
            reach("mid", "core", 1, "core", Scope::Runtime),
            reach("app", "core", 2, "mid", Scope::Runtime),
            reach("top", "core", 3, "app", Scope::Runtime),
        ]
    );
}

#[test]
fn radius_ends_on_a_cycle() {
    let graph = discover(&[
        (
            "a/package.json",
            r#"{ "name": "a", "dependencies": { "c": "*" } }"#,
        ),
        (
            "b/package.json",
            r#"{ "name": "b", "dependencies": { "a": "*" } }"#,
        ),
        (
            "c/package.json",
            r#"{ "name": "c", "dependencies": { "b": "*" } }"#,
        ),
    ]);
    assert_eq!(
        radius(&graph, &["a"]),
        [
            reach("b", "a", 1, "a", Scope::Runtime),
            reach("c", "a", 2, "b", Scope::Runtime),
        ]
    );
}

#[test]
fn a_repository_without_manifests_has_no_projects() {
    let graph = discover(&[("src/main.py", "print('hi')\n")]);
    assert!(graph.projects.is_empty());
    assert_eq!(graph.owner(Path::new("src/main.py")), None);
}

#[test]
fn scope_labels() {
    assert_eq!(Scope::Runtime.label(), "");
    assert_eq!(Scope::Build.label(), "build");
    assert_eq!(Scope::Optional.label(), "optional");
    assert_eq!(Scope::Dev.label(), "dev");
}

#[test]
fn a_pnpm_workspace_root_is_not_a_project() {
    let graph = discover(&[
        ("package.json", r#"{ "name": "monorepo", "private": true }"#),
        ("pnpm-workspace.yaml", "packages:\n  - 'packages/*'\n"),
        ("packages/a/package.json", r#"{ "name": "a" }"#),
        (
            "packages/b/package.json",
            r#"{ "name": "b", "dependencies": { "a": "workspace:^" } }"#,
        ),
    ]);

    assert_eq!(roots(&graph), ["packages/a", "packages/b"]);
    assert_eq!(graph.workspaces, [PathBuf::from("")]);
    assert_eq!(
        edges(&graph),
        [edge("packages/b", "packages/a", Scope::Runtime)]
    );
}

#[test]
fn workspace_roots_of_each_ecosystem() {
    let graph = discover(&[
        ("rust/Cargo.toml", "[workspace]\nmembers = [\"a\"]\n"),
        ("rust/a/Cargo.toml", &cargo("a", "")),
        (
            "rooted/Cargo.toml",
            "[package]\nname = \"rooted\"\n\n[workspace]\nmembers = [\"sub\"]\n",
        ),
        ("rooted/sub/Cargo.toml", &cargo("sub", "")),
        (
            "js/package.json",
            r#"{ "name": "js-root", "workspaces": ["p/*"] }"#,
        ),
        ("js/p/x/package.json", r#"{ "name": "x" }"#),
        ("ex/mix.exs", "def project, do: [apps_path: \"apps\"]\n"),
        ("ex/apps/y/mix.exs", &mix("y", "")),
        ("plain/Cargo.toml", &cargo("plain", "")),
    ]);

    assert_eq!(
        graph.workspaces,
        [
            PathBuf::from("ex"),
            PathBuf::from("js"),
            PathBuf::from("rooted"),
            PathBuf::from("rust"),
        ]
    );
    // A Cargo root that is a package too is a project; the others are not.
    assert_eq!(
        roots(&graph),
        [
            "ex/apps/y",
            "js/p/x",
            "plain",
            "rooted",
            "rooted/sub",
            "rust/a"
        ]
    );
}

#[test]
fn manifest_and_lock_file_of_a_workspace_root_govern_it() {
    let graph = discover(&[
        ("ws/Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
        ("ws/a/Cargo.toml", &cargo("a", "")),
        ("ws/b/Cargo.toml", &cargo("b", "")),
        ("other/Cargo.toml", &cargo("other", "")),
    ]);
    let workspace_of = |file: &str| graph.workspace_of(Path::new(file));

    assert_eq!(workspace_of("ws/Cargo.lock"), Some(Path::new("ws")));
    assert_eq!(workspace_of("ws/Cargo.toml"), Some(Path::new("ws")));
    // The same names elsewhere belong to their own project, or to none.
    assert_eq!(workspace_of("ws/a/Cargo.toml"), None);
    assert_eq!(workspace_of("other/Cargo.lock"), None);
    assert_eq!(workspace_of("Cargo.lock"), None);
    assert_eq!(workspace_of("ws/README.md"), None);
    assert_eq!(workspace_of("ws/rust-toolchain.toml"), None);

    let members: Vec<String> = graph
        .members(Path::new("ws"))
        .map(|i| graph.projects[i].display_root())
        .collect();
    assert_eq!(members, ["ws/a", "ws/b"]);
}

#[test]
fn every_lock_file_of_a_workspace_is_known() {
    let graph = discover(&[
        ("package.json", r#"{ "workspaces": ["p/*"] }"#),
        ("p/a/package.json", r#"{ "name": "a" }"#),
    ]);
    for file in [
        "package.json",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "pnpm-workspace.yaml",
        "bun.lock",
        "bun.lockb",
        "Cargo.toml",
        "Cargo.lock",
        "mix.exs",
        "mix.lock",
    ] {
        assert_eq!(
            graph.workspace_of(Path::new(file)),
            Some(Path::new("")),
            "{file} should govern the workspace"
        );
    }
}

#[test]
fn sample_projects_of_test_suites_are_not_projects() {
    let graph = discover(&[
        ("Cargo.toml", &cargo("tool", "")),
        ("tests/fixtures/sample/Cargo.toml", &cargo("sample", "")),
        (
            "test/__fixtures__/app/package.json",
            r#"{ "name": "fixture-app" }"#,
        ),
        ("testdata/mod/Cargo.toml", &cargo("testdata_mod", "")),
        // An end-to-end suite with its own manifest is a project.
        (
            "test/e2e/package.json",
            r#"{ "name": "e2e", "dependencies": { "widget": "*" } }"#,
        ),
        // `fixtures` outside a test directory is someone's package.
        ("packages/fixtures/package.json", r#"{ "name": "widget" }"#),
    ]);

    assert_eq!(roots(&graph), [".", "packages/fixtures", "test/e2e"]);
    assert_eq!(
        edges(&graph),
        [edge("test/e2e", "packages/fixtures", Scope::Runtime)]
    );
}

#[test]
fn go_modules_are_linked_by_requirement_and_by_replacement() {
    let graph = discover(&[
        ("go.work", "go 1.22\n\nuse (\n\t./cli\n\t./libs/core\n)\n"),
        (
            "libs/core/go.mod",
            "module example.com/acme/libs/core\n\ngo 1.22\n",
        ),
        ("libs/log/go.mod", "module example.com/acme/libs/log\n"),
        (
            "cli/go.mod",
            "module example.com/acme/cli\n\nrequire (\n\texample.com/acme/libs/core v0.0.0\n\tgithub.com/spf13/cobra v1.8.0\n)\n",
        ),
        (
            "tools/gen/go.mod",
            "module example.com/acme/tools/gen\n\nrequire example.com/elsewhere/log v1.0.0\n\nreplace example.com/elsewhere/log => ../../libs/log\n",
        ),
    ]);

    assert_eq!(roots(&graph), ["cli", "libs/core", "libs/log", "tools/gen"]);
    assert_eq!(
        edges(&graph),
        [
            edge("cli", "libs/core", Scope::Runtime),
            edge("tools/gen", "libs/log", Scope::Runtime),
        ]
    );
    // The work file makes its directory a workspace root, and governs it.
    assert_eq!(graph.workspaces, [PathBuf::from("")]);
    assert_eq!(
        graph.workspace_of(Path::new("go.work")),
        Some(Path::new(""))
    );
    assert_eq!(
        graph.workspace_of(Path::new("go.work.sum")),
        Some(Path::new(""))
    );
    assert!(graph.unread.is_empty());
}

#[test]
fn a_go_file_belongs_to_its_module() {
    let graph = discover(&[
        ("ops/go.mod", "module example.com/acme/ops\n"),
        ("apps/web/mix.exs", &mix("web", "")),
    ]);
    let owner = |file: &str| {
        graph
            .owner(Path::new(file))
            .map(|i| graph.projects[i].display_root())
    };
    assert_eq!(owner("ops/cmd/main.go").as_deref(), Some("ops"));
    assert_eq!(owner("ops/go.sum").as_deref(), Some("ops"));
}

#[test]
fn a_dependency_directory_is_skipped_at_a_project_root_only() {
    for skipped in [
        "deps",
        "apps/web/deps",
        "target",
        "_build",
        "vendor",
        "web/node_modules",
    ] {
        assert!(is_skipped(Path::new(skipped)), "{skipped}");
    }
    // Under the sources, the same name is a module of the project.
    for kept in [
        "src/deps",
        "lib/app/target",
        "crates/core/src/_build",
        "src",
        "lib",
    ] {
        assert!(!is_skipped(Path::new(kept)), "{kept}");
    }
    assert!(is_skipped(Path::new("src/node_modules")));
    assert!(is_skipped(Path::new("src/svc/vendor")));
}

#[test]
fn python_projects_are_linked_by_name_and_by_source() {
    let graph = discover(&[
        (
            "pyproject.toml",
            "[tool.uv.workspace]\nmembers = [\"apps/*\", \"libs/*\"]\n",
        ),
        (
            "libs/core/pyproject.toml",
            "[project]\nname = \"Acme_Core\"\n",
        ),
        (
            "libs/log/pyproject.toml",
            "[project]\nname = \"acme-log\"\n",
        ),
        (
            "libs/fixtures/pyproject.toml",
            "[tool.poetry]\nname = \"fixtures\"\n",
        ),
        (
            "apps/api/pyproject.toml",
            r#"
[project]
name = "api"
dependencies = ["acme-core>=1", "renamed", "requests"]

[dependency-groups]
test = ["fixtures", "pytest"]

[tool.uv.sources]
acme-core = { workspace = true }
renamed = { path = "../../libs/log" }
"#,
        ),
        // Only the configuration of a linter: nobody's project.
        ("scripts/pyproject.toml", "[tool.ruff]\nline-length = 100\n"),
    ]);

    assert_eq!(
        roots(&graph),
        ["apps/api", "libs/core", "libs/fixtures", "libs/log"]
    );
    assert_eq!(
        edges(&graph),
        [
            edge("apps/api", "libs/core", Scope::Runtime),
            edge("apps/api", "libs/fixtures", Scope::Dev),
            edge("apps/api", "libs/log", Scope::Runtime),
        ]
    );
    // `requests` and `pytest` are on an index: neither edges nor unread.
    assert!(graph.unread.is_empty());

    assert_eq!(graph.workspaces, [PathBuf::from("")]);
    for governing in ["pyproject.toml", "uv.lock"] {
        assert_eq!(
            graph.workspace_of(Path::new(governing)),
            Some(Path::new("")),
            "{governing}"
        );
    }
    assert_eq!(
        graph.workspace_of(Path::new("libs/core/pyproject.toml")),
        None
    );
}

#[test]
fn a_python_source_pointing_at_no_project_is_reported() {
    let graph = discover(&[(
        "app/pyproject.toml",
        "[project]\nname = \"app\"\ndependencies = [\"gone\"]\n\n[tool.uv.sources]\ngone = { path = \"../gone\" }\n",
    )]);
    assert!(graph.edges.is_empty());
    assert_eq!(graph.unread, [(PathBuf::from("app/pyproject.toml"), 1)]);
}

#[test]
fn only_python_names_are_compared_loosely() {
    let graph = discover(&[
        (
            "web/package.json",
            r#"{ "name": "web", "dependencies": { "acme_core": "*" } }"#,
        ),
        ("core/package.json", r#"{ "name": "acme-core" }"#),
    ]);
    assert!(graph.edges.is_empty());
}
