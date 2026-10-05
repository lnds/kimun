use super::*;

fn dep(target: DepTarget, scope: Scope) -> RawDep {
    RawDep { target, scope }
}

fn path(p: &str) -> DepTarget {
    DepTarget::Path(PathBuf::from(p))
}

#[test]
fn package_with_path_dependencies_by_section() {
    let m = read(
        r#"
[package]
name = "app"

[dependencies]
serde = "1"
core = { path = "../core" }
extra = { path = "../extra", optional = true }
remote = { git = "https://example.com/x" }

[dev-dependencies]
fixtures = { path = "../fixtures" }

[build-dependencies]
codegen = { path = "../codegen" }
"#,
    );

    assert_eq!(m.name.as_deref(), Some("app"));
    assert_eq!(
        m.deps,
        [
            dep(path("../core"), Scope::Runtime),
            dep(path("../extra"), Scope::Optional),
            dep(path("../fixtures"), Scope::Dev),
            dep(path("../codegen"), Scope::Build),
        ]
    );
    assert!(!m.is_workspace_root);
    assert_eq!(m.unread, 0);
}

#[test]
fn dependencies_inherited_from_the_workspace() {
    let m = read(
        r#"
[package]
name = "app"

[dependencies]
core = { workspace = true }
serde.workspace = true
plain = { workspace = false, version = "1" }
"#,
    );
    assert_eq!(
        m.deps,
        [
            dep(DepTarget::Workspace("core".into()), Scope::Runtime),
            dep(DepTarget::Workspace("serde".into()), Scope::Runtime),
        ]
    );
}

#[test]
fn platform_specific_dependencies_are_read() {
    let m = read(
        r#"
[package]
name = "app"

[target.'cfg(unix)'.dependencies]
unix_core = { path = "../unix_core" }

[target.'cfg(windows)'.dev-dependencies]
win_fixtures = { path = "../win_fixtures" }
"#,
    );
    assert_eq!(
        m.deps,
        [
            dep(path("../unix_core"), Scope::Runtime),
            dep(path("../win_fixtures"), Scope::Dev),
        ]
    );
}

#[test]
fn virtual_workspace_root_is_not_a_project() {
    let m = read(
        r#"
[workspace]
members = ["crates/*"]

[workspace.dependencies]
core = { path = "crates/core" }
serde = "1"
"#,
    );
    assert_eq!(m.name, None);
    assert!(m.is_workspace_root);
    assert_eq!(
        m.workspace_paths,
        [("core".to_string(), PathBuf::from("crates/core"))]
    );
}

#[test]
fn workspace_root_can_be_a_package_too() {
    let m = read("[package]\nname = \"root\"\n\n[workspace]\nmembers = [\"sub\"]\n");
    assert_eq!(m.name.as_deref(), Some("root"));
    assert!(m.is_workspace_root);
    assert!(m.workspace_paths.is_empty());
}

#[test]
fn unparsable_manifest_declares_nothing() {
    assert_eq!(read("this is [not toml"), Manifest::default());
}
