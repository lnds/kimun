use super::*;

#[test]
fn go_block_import() {
    let src = r#"
import (
    "fmt"
    "github.com/user/project/pkg/foo"
    alias "github.com/user/project/internal/bar"
)
"#;
    let result = extract_go(src);
    assert_eq!(
        result,
        vec![
            "fmt",
            "github.com/user/project/pkg/foo",
            "github.com/user/project/internal/bar",
        ]
    );
}

#[test]
fn go_single_line_imports() {
    let src = "package main\n\nimport \"fmt\"\nimport bar \"github.com/user/project/bar\"\n";
    assert_eq!(extract_go(src), vec!["fmt", "github.com/user/project/bar"]);
}

#[test]
fn go_strings_outside_imports_are_not_imports() {
    let src = "package main\nconst name = \"before\"\nimport (\n\t\"fmt\"\n\n\t\"os\"\n)\nvar after = \"after\"\n";
    assert_eq!(extract_go(src), vec!["fmt", "os"]);
}

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
