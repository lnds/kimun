use super::*;

fn read(files: &[(&str, &str)]) -> Vec<Config> {
    configs(files.iter().map(|(p, t)| (Path::new(*p), *t)))
}

#[test]
fn paths_are_taken_from_the_config_without_base_url() {
    let found = read(&[(
        "web/tsconfig.json",
        r#"{
          // comments and trailing commas are allowed
          "compilerOptions": { "paths": { "@/*": ["./src/*"], }, },
        }"#,
    )]);
    assert_eq!(
        found,
        vec![Config {
            dir: PathBuf::from("web"),
            base_url: None,
            paths: vec![("@/*".to_string(), vec!["web/src/*".to_string()])],
        }]
    );
}

#[test]
fn paths_are_taken_from_base_url() {
    let found = read(&[(
        "tsconfig.json",
        r#"{"compilerOptions": {"baseUrl": "src", "paths": {"~/*": ["lib/*"]}}}"#,
    )]);
    assert_eq!(found[0].base_url, Some(PathBuf::from("src")));
    assert_eq!(
        found[0].paths,
        vec![("~/*".to_string(), vec!["src/lib/*".to_string()])]
    );
}

#[test]
fn extends_is_followed_and_keeps_where_paths_were_declared() {
    let found = read(&[
        (
            "tsconfig.base.json",
            r#"{"compilerOptions": {"paths": {"@acme/*": ["packages/*/src"]}}}"#,
        ),
        (
            "apps/web/tsconfig.json",
            r#"{"extends": "../../tsconfig.base"}"#,
        ),
    ]);
    let web = found
        .iter()
        .find(|c| c.dir == Path::new("apps/web"))
        .unwrap();
    assert_eq!(
        web.paths,
        vec![("@acme/*".to_string(), vec!["packages/*/src".to_string()])]
    );
}

#[test]
fn a_package_extends_is_external() {
    let found = read(&[(
        "tsconfig.json",
        r#"{"extends": "@tsconfig/node20/tsconfig.json"}"#,
    )]);
    assert!(found.is_empty());
}

#[test]
fn the_app_config_of_a_directory_counts_when_the_main_one_has_no_aliases() {
    let found = read(&[
        (
            "tsconfig.json",
            r#"{"files": [], "references": [{"path": "./tsconfig.app.json"}]}"#,
        ),
        (
            "tsconfig.app.json",
            r#"{"compilerOptions": {"paths": {"@/*": ["./src/*"]}}}"#,
        ),
    ]);
    assert_eq!(
        found[0].paths,
        vec![("@/*".to_string(), vec!["src/*".to_string()])]
    );
}

#[test]
fn an_extends_cycle_ends() {
    let found = read(&[
        (
            "a/tsconfig.json",
            r#"{"extends": "../b/tsconfig.json", "compilerOptions": {"baseUrl": "."}}"#,
        ),
        ("b/tsconfig.json", r#"{"extends": "../a/tsconfig.json"}"#),
    ]);
    assert_eq!(found.len(), 2);
}

#[test]
fn a_package_declares_its_name_and_entries() {
    let found = package(
        Path::new("packages/ui/package.json"),
        r#"{"name": "@acme/ui", "main": "./dist/index.js", "exports": {".": {"import": "./dist/index.mjs"}}}"#,
    );
    assert_eq!(
        found,
        Some(Package {
            dir: PathBuf::from("packages/ui"),
            name: "@acme/ui".to_string(),
            entries: vec![
                "./dist/index.js".to_string(),
                "./dist/index.mjs".to_string()
            ],
        })
    );
    assert_eq!(
        package(Path::new("package.json"), r#"{"private": true}"#),
        None
    );
}

#[test]
fn strict_json_keeps_strings() {
    let text = r#"{"a": "http://x//y", "b": "/* no */",}"#;
    assert_eq!(
        strict_json(text),
        r#"{"a": "http://x//y", "b": "/* no */"}"#
    );
}
