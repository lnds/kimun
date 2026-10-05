use super::*;

fn dep(target: DepTarget, scope: Scope) -> RawDep {
    RawDep { target, scope }
}

fn name(n: &str) -> DepTarget {
    DepTarget::Name(n.to_string())
}

#[test]
fn every_section_with_its_scope() {
    let m = read(
        r#"{
  "name": "@acme/web",
  "dependencies": { "@acme/ui": "workspace:*", "react": "^18.0.0" },
  "peerDependencies": { "@acme/theme": "^1.0.0" },
  "optionalDependencies": { "@acme/native": "1.2.3" },
  "devDependencies": { "@acme/test-utils": "*" }
}"#,
    );

    assert_eq!(m.name.as_deref(), Some("@acme/web"));
    assert_eq!(
        m.deps,
        [
            dep(name("@acme/ui"), Scope::Runtime),
            dep(name("react"), Scope::Runtime),
            dep(name("@acme/theme"), Scope::Runtime),
            dep(name("@acme/native"), Scope::Optional),
            dep(name("@acme/test-utils"), Scope::Dev),
        ]
    );
}

#[test]
fn file_and_link_specs_are_paths() {
    let m =
        read(r#"{ "name": "app", "dependencies": { "a": "file:../a", "b": "link:../libs/b" } }"#);
    assert_eq!(
        m.deps,
        [
            dep(DepTarget::Path(PathBuf::from("../a")), Scope::Runtime),
            dep(DepTarget::Path(PathBuf::from("../libs/b")), Scope::Runtime),
        ]
    );
}

#[test]
fn a_package_without_name_or_dependencies() {
    let m = read(r#"{ "private": true, "workspaces": ["packages/*"] }"#);
    assert_eq!(m.name, None);
    assert!(m.deps.is_empty());
}

#[test]
fn non_string_specs_are_ignored() {
    let m = read(r#"{ "name": "app", "dependencies": { "a": 1, "b": "^1" } }"#);
    assert_eq!(m.deps, [dep(name("b"), Scope::Runtime)]);
}

#[test]
fn unparsable_manifest_declares_nothing() {
    assert_eq!(read("{ not json"), Manifest::default());
}
