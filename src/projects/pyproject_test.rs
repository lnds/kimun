use super::*;

fn dep(target: DepTarget, scope: Scope) -> RawDep {
    RawDep { target, scope }
}

fn name(n: &str) -> DepTarget {
    DepTarget::Name(n.to_string())
}

fn path(p: &str) -> DepTarget {
    DepTarget::Path(PathBuf::from(p))
}

#[test]
fn every_list_of_requirements_with_its_scope() {
    let m = read(
        r#"
[build-system]
requires = ["hatchling", "acme-build>=1"]

[project]
name = "acme-app"
dependencies = [
    "acme-core>=1.0",
    "requests[socks]~=2.31; python_version > '3.8'",
    "acme_log @ file:///srv/acme/log",
]

[project.optional-dependencies]
pdf = ["acme-pdf"]

[dependency-groups]
test = ["acme-fixtures", { include-group = "lint" }]
lint = ["ruff"]
"#,
    );

    assert_eq!(m.name.as_deref(), Some("acme-app"));
    assert_eq!(
        m.deps,
        [
            dep(name("acme-core"), Scope::Runtime),
            dep(name("requests"), Scope::Runtime),
            dep(name("acme_log"), Scope::Runtime),
            dep(name("acme-pdf"), Scope::Optional),
            dep(name("ruff"), Scope::Dev),
            dep(name("acme-fixtures"), Scope::Dev),
            dep(name("hatchling"), Scope::Build),
            dep(name("acme-build"), Scope::Build),
        ]
    );
    assert!(!m.is_workspace_root);
    assert_eq!(m.unread, 0);
}

#[test]
fn uv_sources_say_where_a_requirement_lives() {
    let m = read(
        r#"
[project]
name = "app"
dependencies = ["Acme_Core", "acme-log", "wheels", "requests"]

[tool.uv]
dev-dependencies = ["acme-fixtures"]

[tool.uv.sources]
acme-core = { path = "../core", editable = true }
acme-log = { workspace = true }
acme-fixtures = [{ path = "../fixtures", marker = "sys_platform == 'linux'" }]
wheels = { path = "../dist/wheels-1.0-py3-none-any.whl" }
unused = { path = "../unused" }
"#,
    );

    assert_eq!(
        m.deps,
        [
            // The source is found whatever the spelling of the name.
            dep(path("../core"), Scope::Runtime),
            // A member of the workspace is found by its name.
            dep(name("acme-log"), Scope::Runtime),
            // A built package is not a project.
            dep(name("wheels"), Scope::Runtime),
            dep(name("requests"), Scope::Runtime),
            dep(path("../fixtures"), Scope::Dev),
        ]
    );
}

#[test]
fn poetry_tables_are_requirement_and_source_at_once() {
    let m = read(
        r#"
[tool.poetry]
name = "app"

[tool.poetry.dependencies]
python = "^3.11"
core = { path = "../core", develop = true }
extra = { path = "../extra", optional = true }
requests = "^2.31"

[tool.poetry.group.test.dependencies]
fixtures = { path = "../fixtures" }

[tool.poetry.dev-dependencies]
lint = { path = "../lint" }
"#,
    );

    assert_eq!(m.name.as_deref(), Some("app"));
    assert_eq!(
        m.deps,
        [
            dep(path("../core"), Scope::Runtime),
            dep(path("../extra"), Scope::Optional),
            dep(name("requests"), Scope::Runtime),
            dep(path("../lint"), Scope::Dev),
            dep(path("../fixtures"), Scope::Dev),
        ]
    );
}

#[test]
fn a_poetry_path_places_a_requirement_of_the_project_table() {
    let m = read(
        r#"
[project]
name = "app"
dependencies = ["core>=1"]

[tool.poetry.dependencies]
core = { path = "../core" }
"#,
    );
    assert!(
        m.deps
            .iter()
            .all(|d| d == &dep(path("../core"), Scope::Runtime))
    );
    assert!(!m.deps.is_empty());
}

#[test]
fn a_uv_workspace_root_with_and_without_a_project() {
    let virtual_root = read("[tool.uv.workspace]\nmembers = [\"packages/*\"]\n");
    assert!(virtual_root.is_workspace_root);
    assert_eq!(virtual_root.name, None);

    let root = read("[project]\nname = \"acme\"\n\n[tool.uv.workspace]\nmembers = [\"libs/*\"]\n");
    assert!(root.is_workspace_root);
    assert_eq!(root.name.as_deref(), Some("acme"));
}

#[test]
fn a_file_that_only_configures_tools_declares_no_project() {
    let m = read("[tool.ruff]\nline-length = 100\n\n[tool.pytest.ini_options]\naddopts = \"-q\"\n");
    assert_eq!(m, Manifest::default());
}

#[test]
fn what_is_not_a_requirement_is_left_out() {
    let m = read(
        r#"
[project]
name = "app"
dependencies = ["-e ../core", "", "  spaced >=1"]
"#,
    );
    assert_eq!(m.deps, [dep(name("spaced"), Scope::Runtime)]);
}

#[test]
fn invalid_toml_declares_nothing() {
    assert_eq!(read("[project\nname ="), Manifest::default());
}

fn roots(source: &str) -> Vec<String> {
    import_roots(source)
        .iter()
        .map(|root| root.display().to_string())
        .collect()
}

#[test]
fn import_roots_of_each_build_backend() {
    assert_eq!(
        roots("[tool.setuptools.packages.find]\nwhere = [\"python\", \"plugins\"]\n"),
        ["python", "plugins"]
    );
    assert_eq!(
        roots("[tool.setuptools.package-dir]\n\"\" = \"lib\"\n"),
        ["lib"]
    );
    assert_eq!(
        roots(
            "[tool.poetry]\npackages = [{ include = \"acme\", from = \"python\" }, { include = \"flat\" }]\n"
        ),
        ["python"]
    );
    assert_eq!(
        roots("[tool.maturin]\npython-source = \"python\"\n"),
        ["python"]
    );
    assert_eq!(roots("[tool.pdm.build]\npackage-dir = \"lib\"\n"), ["lib"]);
    assert_eq!(
        roots("[tool.uv.build-backend]\nmodule-root = \"lib\"\n"),
        ["lib"]
    );
}

#[test]
fn hatch_lists_roots_or_the_packages_under_them() {
    assert_eq!(
        roots("[tool.hatch.build.targets.wheel]\nsources = [\"python\"]\n"),
        ["python"]
    );
    assert_eq!(
        roots("[tool.hatch.build.targets.wheel.sources]\n\"python\" = \"\"\n"),
        ["python"]
    );
    assert_eq!(
        roots("[tool.hatch.build.targets.wheel]\npackages = [\"python/acme\", \"python/tools\"]\n"),
        ["python"]
    );
    assert_eq!(roots("[tool.hatch.build]\nsources = [\"lib\"]\n"), ["lib"]);
}

#[test]
fn what_pytest_adds_to_the_search_path_is_a_root() {
    assert_eq!(
        roots("[tool.pytest.ini_options]\npythonpath = [\".\", \"python\"]\n"),
        [".", "python"]
    );
    assert_eq!(
        roots("[tool.pytest.ini_options]\npythonpath = \"python\"\n"),
        ["python"]
    );
}

#[test]
fn a_root_declared_twice_is_one_and_none_is_none() {
    assert_eq!(
        roots(
            "[tool.setuptools.packages.find]\nwhere = [\"python\"]\n\n[tool.pytest.ini_options]\npythonpath = [\"python\"]\n"
        ),
        ["python"]
    );
    assert!(roots("[project]\nname = \"app\"\n").is_empty());
    assert!(roots("[tool").is_empty());
}
