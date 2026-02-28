use super::*;

#[test]
fn rust_markers_exist() {
    assert!(cognitive_markers_for("Rust").is_some());
}

#[test]
fn python_markers_exist() {
    assert!(cognitive_markers_for("Python").is_some());
}

#[test]
fn javascript_markers_exist() {
    assert!(cognitive_markers_for("JavaScript").is_some());
}

#[test]
fn typescript_shares_javascript() {
    let js = cognitive_markers_for("JavaScript").unwrap();
    let ts = cognitive_markers_for("TypeScript").unwrap();
    assert_eq!(js.structural_keywords.len(), ts.structural_keywords.len());
}

#[test]
fn c_family_languages_share_markers() {
    let langs = ["Java", "C#", "C", "C++", "Objective-C", "PHP", "Dart"];
    for lang in &langs {
        assert!(
            cognitive_markers_for(lang).is_some(),
            "{lang} should have cognitive markers"
        );
    }
}

#[test]
fn go_markers_exist() {
    assert!(cognitive_markers_for("Go").is_some());
}

#[test]
fn ruby_markers_exist() {
    assert!(cognitive_markers_for("Ruby").is_some());
}

#[test]
fn shell_variants_share_markers() {
    for lang in &["Bourne Shell", "Bourne Again Shell", "Zsh"] {
        assert!(
            cognitive_markers_for(lang).is_some(),
            "{lang} should have markers"
        );
    }
}

#[test]
fn unknown_language_returns_none() {
    assert!(cognitive_markers_for("UnknownLang").is_none());
}

#[test]
fn all_supported_languages_have_markers() {
    let languages = [
        "Rust",
        "Python",
        "JavaScript",
        "TypeScript",
        "Java",
        "C#",
        "C",
        "C++",
        "Objective-C",
        "PHP",
        "Dart",
        "Go",
        "Ruby",
        "Kotlin",
        "Swift",
        "Scala",
        "Bourne Shell",
        "Bourne Again Shell",
        "Zsh",
        "Haskell",
        "Elixir",
        "Elixir Script",
        "Lua",
        "Perl",
        "Erlang",
        "OCaml",
        "F#",
        "R",
        "Julia",
        "Nim",
        "Zig",
        "Clojure",
    ];
    for lang in &languages {
        assert!(
            cognitive_markers_for(lang).is_some(),
            "{lang} should have cognitive markers"
        );
    }
}

#[test]
fn rust_has_structural_keywords() {
    let m = cognitive_markers_for("Rust").unwrap();
    assert!(m.structural_keywords.contains(&"if"));
    assert!(m.structural_keywords.contains(&"for"));
    assert!(m.structural_keywords.contains(&"while"));
    assert!(m.structural_keywords.contains(&"match"));
    assert!(m.structural_keywords.contains(&"loop"));
}

#[test]
fn rust_has_hybrid_keywords() {
    let m = cognitive_markers_for("Rust").unwrap();
    assert!(m.hybrid_keywords.contains(&"else if"));
}

#[test]
fn rust_has_fundamental_keywords() {
    let m = cognitive_markers_for("Rust").unwrap();
    assert!(m.fundamental_keywords.contains(&"else"));
}

#[test]
fn python_has_elif_as_hybrid() {
    let m = cognitive_markers_for("Python").unwrap();
    assert!(m.hybrid_keywords.contains(&"elif"));
}

#[test]
fn python_uses_word_boolean_operators() {
    let m = cognitive_markers_for("Python").unwrap();
    assert!(m.boolean_operators.contains(&"and"));
    assert!(m.boolean_operators.contains(&"or"));
}
