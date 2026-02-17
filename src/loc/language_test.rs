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
