use super::*;

#[test]
fn dispatcher_routes_each_language_to_its_extractor() {
    let cases = [
        ("pkg/a.py", "Python", "from .foo import bar\n", ".foo:bar"),
        (
            "src/a.js",
            "JavaScript",
            "import a from './foo';\n",
            "./foo",
        ),
        (
            "src/a.ts",
            "TypeScript",
            "import a from './foo';\n",
            "./foo",
        ),
        ("src/a.jsx", "JSX", "import a from './foo';\n", "./foo"),
        ("main.go", "Go", "import \"fmt\"\n", "fmt"),
    ];
    for (_, language, source, expected) in cases {
        let result = extract_imports(language, source);
        assert_eq!(result, vec![expected], "{language}");
    }
}

#[test]
fn kaikai_is_extracted_through_the_dispatcher() {
    let result = extract_imports("Kaikai", "import app.b\n");
    assert_eq!(result, vec!["app.b"]);
}

#[test]
fn supported_languages() {
    for lang in [
        "Rust",
        "Python",
        "TypeScript",
        "JSX",
        "Go",
        "Kaikai",
        "C",
        "C++",
        "Java",
    ] {
        assert!(is_supported(lang), "{lang} should be supported");
    }
    for lang in ["Bash", "TOML", "Kotlin"] {
        assert!(!is_supported(lang), "{lang} should not be supported");
    }
}
