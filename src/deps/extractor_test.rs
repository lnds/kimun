use super::*;
use std::path::PathBuf;

#[test]
fn rust_mod_declarations() {
    let src = r#"
pub mod analyzer;
mod report;
pub(crate) mod utils;
mod inline { fn foo() {} }  // should be skipped (has {)
// mod commented_out;
"#;
    let result = extract_rust(&PathBuf::from("src/lib.rs"), src);
    assert_eq!(result, vec!["analyzer", "report", "utils"]);
}

#[test]
fn python_relative_imports() {
    let src = "from .foo import bar\nfrom . import baz\nfrom ..utils import helper\nimport os\n";
    let result = extract_python(src);
    assert_eq!(result, vec![".foo", "..utils"]);
}

#[test]
fn js_relative_imports() {
    let src = r#"
import foo from './foo';
import { bar } from '../bar';
import external from 'lodash';
const x = require('./utils');
"#;
    let result = extract_js(src);
    assert_eq!(result, vec!["./foo", "../bar", "./utils"]);
}

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
fn rust_mod_without_semicolon_is_skipped() {
    assert!(extract_rust(&PathBuf::from("src/lib.rs"), "mod foo\n").is_empty());
}

#[test]
fn rust_pub_super_mod_is_extracted() {
    let result = extract_rust(&PathBuf::from("src/lib.rs"), "pub(super) mod helpers;\n");
    assert_eq!(result, vec!["helpers"]);
}

#[test]
fn rust_pub_in_path_mod_is_extracted() {
    let src = "pub(in crate::deps) mod scoped;\npub(in  mod broken;\npublic mod other;\n";
    let result = extract_rust(&PathBuf::from("src/lib.rs"), src);
    assert_eq!(result, vec!["scoped"]);
}

#[test]
fn rust_mod_name_may_contain_underscores() {
    let result = extract_rust(&PathBuf::from("src/lib.rs"), "mod string_mask;\n");
    assert_eq!(result, vec!["string_mask"]);
}

#[test]
fn rust_mod_with_invalid_name_is_skipped() {
    let src = "mod foo-bar;\nmod ;\n";
    assert!(extract_rust(&PathBuf::from("src/lib.rs"), src).is_empty());
}

#[test]
fn js_comment_lines_are_skipped() {
    let src =
        "// import a from './line';\n/* import b from './block'; */\n * import c from './doc';\n";
    assert!(extract_js(src).is_empty());
}

#[test]
fn js_imports_without_semicolons() {
    let src = "import external from 'lodash'\nimport bar from './bar'\n";
    assert_eq!(extract_js(src), vec!["./bar"]);
}

#[test]
fn js_relative_string_after_a_bare_one_on_the_same_line() {
    let src = "export { a } from 'pkg'; export { b } from './local';\n";
    assert_eq!(extract_js(src), vec!["./local"]);
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
        ("src/lib.rs", "Rust", "mod foo;\n", "foo"),
        ("pkg/a.py", "Python", "from .foo import bar\n", ".foo"),
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
        ("src/a.tsx", "TSX", "import a from './foo';\n", "./foo"),
        ("main.go", "Go", "import \"fmt\"\n", "fmt"),
    ];
    for (path, language, source, expected) in cases {
        let result = extract_imports(&PathBuf::from(path), language, source);
        assert_eq!(result, vec![expected], "{language}");
    }
}

#[test]
fn kaikai_is_extracted_through_the_dispatcher() {
    let result = extract_imports(&PathBuf::from("app/main.kai"), "Kaikai", "import app.b\n");
    assert_eq!(result, vec!["app.b"]);
}

#[test]
fn supported_languages() {
    for lang in ["Rust", "Python", "TypeScript", "TSX", "Go", "Kaikai"] {
        assert!(is_supported(lang), "{lang} should be supported");
    }
    for lang in ["Bash", "TOML", "Java"] {
        assert!(!is_supported(lang), "{lang} should not be supported");
    }
}
