use super::*;

fn inert(patterns: &[&str]) -> Inert {
    let patterns: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
    Inert::new(&patterns).unwrap()
}

#[test]
fn documentation_is_inert_without_any_configuration() {
    let inert = inert(&[]);
    for path in [
        "README.md",
        "docs/guide/intro.mdx",
        "kb/notes.RST",
        "CHANGELOG.txt",
        "a/b.adoc",
    ] {
        assert!(inert.matches(Path::new(path)), "{path}");
    }
    for path in [
        "src/main.rs",
        "lib/app.ex",
        ".github/workflows/ci.yml",
        "Makefile",
        "scripts/run.sh",
    ] {
        assert!(!inert.matches(Path::new(path)), "{path}");
    }
}

#[test]
fn configured_paths_are_inert_too() {
    let inert = inert(&["scripts/**", "*.png", "tools/bench/*.exs"]);
    assert!(inert.matches(Path::new("scripts/deploy/run.sh")));
    assert!(inert.matches(Path::new("assets/logo.png")));
    assert!(inert.matches(Path::new("tools/bench/load.exs")));
    assert!(!inert.matches(Path::new("tools/other/load.exs")));
    assert!(!inert.matches(Path::new("lib/scripts.ex")));
    // Documentation stays inert next to them.
    assert!(inert.matches(Path::new("README.md")));
}

#[test]
fn an_invalid_pattern_is_reported() {
    let err = Inert::new(&["a/[".to_string()]).err().unwrap().to_string();
    assert!(err.contains("[impact] inert"), "{err}");
}
