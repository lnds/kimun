use super::*;
use std::path::Path;

#[test]
fn detect_by_extension_rs() {
    let spec = detect(Path::new("main.rs")).unwrap();
    assert_eq!(spec.name, "Rust");
}

#[test]
fn detect_by_extension_py() {
    let spec = detect(Path::new("script.py")).unwrap();
    assert_eq!(spec.name, "Python");
}

#[test]
fn detect_by_filename_makefile() {
    let spec = detect(Path::new("Makefile")).unwrap();
    assert_eq!(spec.name, "Makefile");
}

#[test]
fn detect_by_filename_dockerfile() {
    let spec = detect(Path::new("Dockerfile")).unwrap();
    assert_eq!(spec.name, "Dockerfile");
}

#[test]
fn detect_unknown_extension() {
    assert!(detect(Path::new("file.xyz123")).is_none());
}

#[test]
fn detect_no_extension() {
    // A file with no extension and no matching filename
    assert!(detect(Path::new("randomfile")).is_none());
}

#[test]
fn shebang_python() {
    let spec = detect_by_shebang("#!/usr/bin/python3\n").unwrap();
    assert_eq!(spec.name, "Python");
}

#[test]
fn shebang_env_python() {
    let spec = detect_by_shebang("#!/usr/bin/env python3\n").unwrap();
    assert_eq!(spec.name, "Python");
}

#[test]
fn shebang_env_with_flags() {
    let spec = detect_by_shebang("#!/usr/bin/env -S python3 -u\n").unwrap();
    assert_eq!(spec.name, "Python");
}

#[test]
fn shebang_bash() {
    let spec = detect_by_shebang("#!/bin/bash\n").unwrap();
    assert_eq!(spec.name, "Bourne Again Shell");
}

#[test]
fn shebang_node() {
    let spec = detect_by_shebang("#!/usr/bin/env node\n").unwrap();
    assert_eq!(spec.name, "JavaScript");
}

#[test]
fn shebang_not_a_shebang() {
    assert!(detect_by_shebang("print('hello')\n").is_none());
}

#[test]
fn shebang_unknown_interpreter() {
    assert!(detect_by_shebang("#!/usr/bin/unknownlang\n").is_none());
}

#[test]
fn languages_not_empty() {
    assert!(!languages().is_empty());
}

#[test]
fn all_languages_have_names() {
    for spec in languages() {
        assert!(!spec.name.is_empty());
    }
}

#[test]
fn detect_heex() {
    let spec = detect(Path::new("component.heex")).unwrap();
    assert_eq!(spec.name, "HTML EEx");
}

#[test]
fn detect_po() {
    let spec = detect(Path::new("es.po")).unwrap();
    assert_eq!(spec.name, "PO File");
}

#[test]
fn detect_pot() {
    let spec = detect(Path::new("default.pot")).unwrap();
    assert_eq!(spec.name, "PO File");
}

#[test]
fn detect_kaikai() {
    let spec = detect(Path::new("main.kai")).unwrap();
    assert_eq!(spec.name, "Kaikai");
}

#[test]
fn react_components_are_detected() {
    assert_eq!(detect(Path::new("App.jsx")).unwrap().name, "JSX");
    assert_eq!(detect(Path::new("App.tsx")).unwrap().name, "TypeScript");
}

#[test]
fn new_languages_by_name_and_extension() {
    let name = |p: &str| detect(Path::new(p)).map(|s| s.name);
    assert_eq!(name("docs/index.rst"), Some("reStructuredText"));
    assert_eq!(name("src/parser.y"), Some("yacc"));
    assert_eq!(name("src/lexer.l"), Some("lex"));
    assert_eq!(name("lib/web/page.html.eex"), Some("EEx"));
    assert_eq!(name("CMakeLists.txt"), Some("CMake"));
    assert_eq!(name("cmake/fmt.cmake"), Some("CMake"));
    assert_eq!(name("src/builtin.jq"), Some("jq"));
    assert_eq!(name("include/a.hh"), Some("C++"));
    assert_eq!(name("notes.txt"), Some("Text"));
}

#[test]
fn header_is_cpp_when_its_text_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let cpp = dir.path().join("fmt.h");
    std::fs::write(&cpp, "#pragma once\nnamespace fmt {\nint f();\n}\n").unwrap();
    let c = dir.path().join("jv.h");
    std::fs::write(
        &c,
        "#ifndef JV_H\n#ifdef __cplusplus\nextern \"C\" {\n#endif\nint jv(void);\n",
    )
    .unwrap();
    assert_eq!(detect_file(&cpp).map(|s| s.name), Some("C++"));
    assert_eq!(detect_file(&c).map(|s| s.name), Some("C"));
    assert_eq!(detect(&cpp).map(|s| s.name), Some("C"));
}

#[test]
fn restructured_text_is_prose() {
    let spec = detect(Path::new("a.rst")).unwrap();
    assert!(!spec.is_code());
}
