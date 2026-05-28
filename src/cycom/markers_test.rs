use super::*;

#[test]
fn rust_markers_exist() {
    let m = markers_for("Rust").unwrap();
    assert!(m.brace_scoped);
    assert!(m.keywords.contains(&"if"));
    assert!(m.function_markers.contains(&"fn "));
}

#[test]
fn json_returns_none() {
    assert!(markers_for("JSON").is_none());
}

#[test]
fn html_returns_none() {
    assert!(markers_for("HTML").is_none());
}

#[test]
fn python_is_indent_scoped() {
    let m = markers_for("Python").unwrap();
    assert!(!m.brace_scoped);
    assert!(m.keywords.contains(&"elif"));
}

#[test]
fn c_family_shared() {
    let java = markers_for("Java").unwrap();
    let c = markers_for("C").unwrap();
    assert!(std::ptr::eq(java, c));
}

#[test]
fn shell_variants() {
    assert!(markers_for("Bourne Shell").is_some());
    assert!(markers_for("Bourne Again Shell").is_some());
    assert!(markers_for("Zsh").is_some());
}

#[test]
fn unknown_language_returns_none() {
    assert!(markers_for("Unknown").is_none());
    assert!(markers_for("Markdown").is_none());
    assert!(markers_for("TOML").is_none());
}

#[test]
fn kaikai_markers_present() {
    let m = markers_for("Kaikai").unwrap();
    assert_eq!(m.function_markers, &["fn "]);
    assert!(m.brace_scoped);
    assert!(m.keywords.contains(&"match"));
    assert!(m.keywords.contains(&"when"));
    assert!(m.keywords.contains(&"handle"));
    assert!(m.keywords.contains(&"and"));
    assert!(m.keywords.contains(&"or"));
}
