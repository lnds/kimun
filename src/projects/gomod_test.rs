use super::*;

fn name(module: &str) -> RawDep {
    dep(DepTarget::Name(module.to_string()))
}

fn path(dir: &str) -> RawDep {
    dep(DepTarget::Path(PathBuf::from(dir)))
}

#[test]
fn module_requirements_and_local_replacements() {
    let m = read(
        r#"
module example.com/acme/tools/cli // the command line

go 1.22

require (
	example.com/acme/libs/core v0.0.0
	github.com/spf13/cobra v1.8.0 // indirect

	golang.org/x/sys v0.20.0
)

require example.com/acme/libs/log v0.0.0

replace example.com/acme/libs/core => ../../libs/core

replace (
	example.com/acme/libs/log => ./vendored/log
	github.com/old/dep v1.0.0 => github.com/new/dep v1.2.3
)
"#,
    );

    assert_eq!(m.name.as_deref(), Some("example.com/acme/tools/cli"));
    assert_eq!(
        m.deps,
        [
            name("example.com/acme/libs/core"),
            name("github.com/spf13/cobra"),
            name("golang.org/x/sys"),
            name("example.com/acme/libs/log"),
            path("../../libs/core"),
            path("./vendored/log"),
        ]
    );
    assert!(!m.is_workspace_root);
}

#[test]
fn a_module_with_nothing_else() {
    let m = read("module \"example.com/solo\"\n\ngo 1.22\n");
    assert_eq!(m.name.as_deref(), Some("example.com/solo"));
    assert!(m.deps.is_empty());
}

#[test]
fn words_that_only_start_like_a_directive_are_not_one() {
    let m = read("module example.com/a\n\nrequirements v1\nreplaced => ../x\n");
    assert!(m.deps.is_empty());
}

#[test]
fn a_file_that_is_not_a_module_declares_nothing() {
    assert_eq!(read(""), Manifest::default());
    assert_eq!(read("// just a comment\n").name, None);
}

#[test]
fn a_work_file_is_a_workspace_root() {
    let m = read_work("go 1.22\n\nuse (\n\t./cli\n\t./libs/core\n)\n");
    assert!(m.is_workspace_root);
    assert_eq!(m.name, None);
    assert!(m.deps.is_empty());
}
