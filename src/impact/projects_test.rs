use super::*;
use std::fs;

fn graph(files: &[(&str, &str)]) -> (tempfile::TempDir, ProjectGraph) {
    let dir = tempfile::tempdir().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, content).unwrap();
    }
    let graph = ProjectGraph::discover(dir.path());
    (dir, graph)
}

fn package(name: &str, deps: &[&str], dev: &[&str]) -> String {
    let list = |names: &[&str]| {
        names
            .iter()
            .map(|n| format!("\"{n}\": \"*\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{{ \"name\": \"{name}\", \"dependencies\": {{ {} }}, \"devDependencies\": {{ {} }} }}",
        list(deps),
        list(dev)
    )
}

/// core ← ledger ← payouts, core ← invoicing, helpers ←(dev) invoicing.
fn monorepo() -> (tempfile::TempDir, ProjectGraph) {
    graph(&[
        ("libs/core/package.json", &package("core", &[], &[])),
        ("libs/helpers/package.json", &package("helpers", &[], &[])),
        (
            "libs/ledger/package.json",
            &package("ledger", &["core"], &[]),
        ),
        (
            "apps/invoicing/package.json",
            &package("invoicing", &["core"], &["helpers"]),
        ),
        (
            "apps/payouts/package.json",
            &package("payouts", &["ledger"], &[]),
        ),
        ("apps/site/package.json", &package("site", &[], &[])),
    ])
}

fn radius_of(graph: &ProjectGraph, files: &[&str]) -> ProjectRadius {
    compute(graph, files.iter().map(|f| Path::new(*f)))
}

fn row(
    project: &str,
    origin: &str,
    distance: usize,
    via: Option<&str>,
    scope: &'static str,
) -> ReachRow {
    ReachRow {
        project: project.to_string(),
        origin: origin.to_string(),
        distance,
        via: via.map(str::to_string),
        scope,
    }
}

#[test]
fn reach_of_a_change_to_a_shared_library() {
    let (_dir, graph) = monorepo();
    let radius = radius_of(
        &graph,
        &["libs/core/src/index.js", "libs/core/package.json"],
    );

    assert_eq!(radius.total(), 6);
    assert_eq!(radius.changed, ["libs/core"]);
    assert_eq!(
        radius.reached,
        [
            row("apps/invoicing", "libs/core", 1, None, ""),
            row("libs/ledger", "libs/core", 1, None, ""),
            row("apps/payouts", "libs/core", 2, Some("libs/ledger"), ""),
        ]
    );
    assert_eq!(radius.direct(), 2);
    assert_eq!(
        radius.affected(),
        ["apps/invoicing", "apps/payouts", "libs/core", "libs/ledger"]
    );
    assert!(radius.outside.is_empty());
}

#[test]
fn files_outside_every_project_are_set_apart() {
    let (_dir, graph) = monorepo();
    let radius = radius_of(
        &graph,
        &[
            ".github/workflows/ci.yml",
            "README.md",
            "README.md",
            "apps/site/index.js",
        ],
    );

    assert_eq!(radius.changed, ["apps/site"]);
    assert!(radius.reached.is_empty());
    assert_eq!(
        radius.outside,
        [
            PathBuf::from(".github/workflows/ci.yml"),
            PathBuf::from("README.md")
        ]
    );
    // Their reach is unknown, so every project is affected.
    assert_eq!(
        radius.affected(),
        radius.all.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(radius.all.len(), 6);
}

#[test]
fn render_lists_each_reach_with_its_path() {
    let (_dir, graph) = monorepo();
    let radius = radius_of(
        &graph,
        &["libs/core/index.js", "libs/helpers/index.js", "Makefile"],
    );
    let sep = "─".repeat(78);

    let expected = [
        "Blast radius — projects reached through their manifests",
        sep.as_str(),
        " 3 of 6 projects reached (50%), 2 direct",
        " Changed: libs/core, libs/helpers",
        "",
        " Changed    Reaches         Distance  Scope  Via",
        " libs/core  apps/invoicing         1",
        " libs/core  libs/ledger            1",
        " libs/core  apps/payouts           2         libs/ledger",
        sep.as_str(),
        "Changed files outside every project (reach unknown): Makefile",
    ];
    assert_eq!(render(&radius), expected);
}

#[test]
fn render_marks_a_reach_through_a_dev_dependency() {
    let (_dir, graph) = monorepo();
    let radius = radius_of(&graph, &["libs/helpers/index.js"]);

    let lines = render(&radius);
    assert_eq!(lines[2], " 1 of 6 projects reached (17%), 1 direct");
    assert_eq!(lines[3], " Changed: libs/helpers");
    assert_eq!(
        lines[5],
        " Changed       Reaches         Distance  Scope  Via"
    );
    assert_eq!(lines[6], " libs/helpers  apps/invoicing         1  dev");
}

#[test]
fn render_when_nothing_depends_on_the_change() {
    let (_dir, graph) = monorepo();
    let radius = radius_of(&graph, &["apps/site/index.js", "apps/payouts/index.js"]);
    assert_eq!(
        render(&radius)[2],
        " 0 of 6 projects reached: nothing depends on apps/payouts, apps/site."
    );
}

#[test]
fn render_when_no_project_changed() {
    let (_dir, graph) = monorepo();
    let lines = render(&radius_of(&graph, &["Makefile"]));
    assert_eq!(lines[2], " No project changed.");
    assert_eq!(
        lines[4],
        "Changed files outside every project (reach unknown): Makefile"
    );
}

#[test]
fn render_says_when_the_level_does_not_apply() {
    let (_dir, single) = graph(&[("package.json", &package("app", &[], &[]))]);
    let lines = render(&radius_of(&single, &["index.js"]));
    assert_eq!(
        lines[2],
        " Single project (.): no other project to reach; this level does not apply."
    );
    assert_eq!(lines[1].chars().count(), 78);

    let (_dir, none) = graph(&[("main.py", "")]);
    let lines = render(&radius_of(&none, &["main.py"]));
    assert_eq!(
        lines[2],
        " No project manifests found (Cargo.toml, package.json, mix.exs)."
    );
    assert_eq!(lines[1].chars().count(), 78);
}

#[test]
fn render_names_the_manifests_it_could_not_read_in_full() {
    let (_dir, graph) = graph(&[
        ("a/package.json", &package("a", &[], &[])),
        (
            "b/package.json",
            r#"{ "name": "b", "dependencies": { "gone": "file:../gone" } }"#,
        ),
    ]);
    let lines = render(&radius_of(&graph, &["a/index.js"]));
    assert_eq!(
        lines.last().unwrap(),
        "Local dependencies that could not be read: b/package.json (1)"
    );
}

#[test]
fn json_block() {
    let (_dir, graph) = monorepo();
    let radius = radius_of(
        &graph,
        &["libs/core/index.js", "libs/helpers/index.js", "Makefile"],
    );
    let json = serde_json::to_value(JsonProjects::from(&radius)).unwrap();

    assert_eq!(json["total"], 6);
    assert_eq!(json["direct"], 2);
    assert_eq!(
        json["changed"],
        serde_json::json!(["libs/core", "libs/helpers"])
    );
    assert_eq!(json["outside"], serde_json::json!(["Makefile"]));
    assert_eq!(json["reached"].as_array().unwrap().len(), 3);
    assert_eq!(
        json["reached"][2],
        serde_json::json!({
            "project": "apps/payouts",
            "origin": "libs/core",
            "distance": 2,
            "via": "libs/ledger",
            "scope": "runtime"
        })
    );
    assert_eq!(json["reached"][0]["via"], serde_json::Value::Null);
    // `Makefile` belongs to no project: all six are affected.
    assert_eq!(json["affected"].as_array().unwrap().len(), 6);
    assert_eq!(json["unread"], serde_json::json!([]));
}

#[test]
fn json_scope_of_a_dev_reach() {
    let (_dir, graph) = monorepo();
    let radius = radius_of(&graph, &["libs/helpers/index.js"]);
    let json = serde_json::to_value(JsonProjects::from(&radius)).unwrap();
    assert_eq!(json["reached"][0]["scope"], "dev");
}

/// An npm workspace: ui ← web, and a tool that stands alone.
fn npm_workspace() -> (tempfile::TempDir, ProjectGraph) {
    graph(&[
        (
            "package.json",
            r#"{ "private": true, "workspaces": ["packages/*"] }"#,
        ),
        ("packages/ui/package.json", &package("ui", &[], &[])),
        ("packages/web/package.json", &package("web", &["ui"], &[])),
        ("packages/tool/package.json", &package("tool", &[], &[])),
    ])
}

#[test]
fn a_lock_file_bump_reaches_every_member_of_the_workspace() {
    let (_dir, graph) = npm_workspace();
    let radius = radius_of(&graph, &["package-lock.json"]);

    assert!(radius.changed.is_empty());
    assert!(radius.outside.is_empty());
    assert_eq!(
        radius.reached,
        [
            row("packages/tool", ".", 1, None, "workspace"),
            row("packages/ui", ".", 1, None, "workspace"),
            row("packages/web", ".", 1, None, "workspace"),
        ]
    );
    assert_eq!(
        radius.affected(),
        ["packages/tool", "packages/ui", "packages/web"]
    );

    let lines = render(&radius);
    assert_eq!(lines[2], " 3 of 3 projects reached (100%), 3 direct");
    assert_eq!(lines[3], " Changed: no project (the workspace root)");
    assert_eq!(lines[6], " .        packages/tool         1  workspace");
}

#[test]
fn other_root_files_of_a_workspace_have_unknown_reach() {
    let (_dir, graph) = npm_workspace();
    let radius = radius_of(&graph, &["README.md"]);

    assert!(radius.reached.is_empty());
    assert_eq!(radius.outside, [PathBuf::from("README.md")]);
    let lines = render(&radius);
    assert_eq!(lines[2], " No project changed.");
    assert_eq!(
        lines[4],
        "Changed files outside every project (reach unknown): README.md"
    );
    assert_eq!(radius.affected().len(), 3);
}

#[test]
fn a_changed_member_is_not_reached_by_its_workspace() {
    let (_dir, graph) = npm_workspace();
    let radius = radius_of(&graph, &["yarn.lock", "packages/ui/index.js"]);

    assert_eq!(radius.changed, ["packages/ui"]);
    assert_eq!(
        radius.reached,
        [
            row("packages/tool", ".", 1, None, "workspace"),
            row("packages/web", ".", 1, None, "workspace"),
        ]
    );
}

#[test]
fn a_workspace_change_carries_on_to_projects_outside_it() {
    let (_dir, graph) = graph(&[
        ("engine/Cargo.toml", "[workspace]\nmembers = [\"core\"]\n"),
        ("engine/core/Cargo.toml", "[package]\nname = \"core\"\n"),
        (
            "apps/cli/Cargo.toml",
            "[package]\nname = \"cli\"\n\n[dependencies]\ncore = { path = \"../../engine/core\" }\n",
        ),
        (
            "apps/gui/Cargo.toml",
            "[package]\nname = \"gui\"\n\n[dev-dependencies]\ncli = { path = \"../cli\" }\n",
        ),
    ]);
    let radius = radius_of(&graph, &["engine/Cargo.toml"]);

    assert_eq!(
        radius.reached,
        [
            row("engine/core", "engine", 1, None, "workspace"),
            row("apps/cli", "engine", 2, Some("engine/core"), ""),
            row("apps/gui", "engine", 3, Some("apps/cli"), "dev"),
        ]
    );
}

#[test]
fn the_lock_file_of_an_umbrella_reaches_its_apps() {
    let (_dir, graph) = graph(&[
        (
            "mix.exs",
            "def project, do: [apps_path: \"apps\", deps: []]\n",
        ),
        ("apps/store/mix.exs", "def project, do: [app: :store]\n"),
        ("apps/web/mix.exs", "def project, do: [app: :web]\n"),
    ]);
    let radius = radius_of(&graph, &["mix.lock"]);

    assert_eq!(
        radius.reached,
        [
            row("apps/store", ".", 1, None, "workspace"),
            row("apps/web", ".", 1, None, "workspace"),
        ]
    );
    // A shared `config/` is no manifest: its reach is unknown.
    let radius = radius_of(&graph, &["config/config.exs"]);
    assert_eq!(radius.outside, [PathBuf::from("config/config.exs")]);
    assert_eq!(radius.affected(), ["apps/store", "apps/web"]);
}

#[test]
fn a_root_package_that_is_a_workspace_too_changes_and_governs() {
    let (_dir, graph) = graph(&[
        (
            "Cargo.toml",
            "[package]\nname = \"root\"\n\n[workspace]\nmembers = [\"sub\"]\n",
        ),
        ("sub/Cargo.toml", "[package]\nname = \"sub\"\n"),
    ]);
    let radius = radius_of(&graph, &["Cargo.toml"]);

    assert_eq!(radius.changed, ["."]);
    assert_eq!(radius.reached, [row("sub", ".", 1, None, "workspace")]);

    // Its sources change the root project alone.
    let radius = radius_of(&graph, &["src/main.rs"]);
    assert_eq!(radius.changed, ["."]);
    assert!(radius.reached.is_empty());
}

#[test]
fn the_files_of_a_deleted_project_have_unknown_reach() {
    // The graph is read from the working tree, where `libs/gone` no longer is.
    let (_dir, graph) = monorepo();
    let radius = radius_of(&graph, &["libs/gone/package.json", "libs/gone/index.js"]);

    assert!(radius.changed.is_empty());
    assert_eq!(radius.outside.len(), 2);
    assert_eq!(radius.affected().len(), 6);
}
