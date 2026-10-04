use super::*;

// ── extraction ───────────────────────────────────────────────────────────────

#[test]
fn import_forms() {
    let src = "\
import compiler.ast
import loop
import core.list as list
import mathlib.{add, mul}
import app.util.text.{shout}

fn main() : Unit / Stdout = Stdout.print(\"hi\")
";
    assert_eq!(
        extract(src),
        vec![
            "compiler.ast",
            "loop",
            "core.list",
            "mathlib",
            "app.util.text"
        ]
    );
}

#[test]
fn import_with_trailing_comment() {
    assert_eq!(
        extract("import compiler.ast # the tree\n"),
        vec!["compiler.ast"]
    );
}

#[test]
fn import_in_comment_is_skipped() {
    let src = "# import compiler.ast\n  # import compiler.lexer\nimport compiler.parser\n";
    assert_eq!(extract(src), vec!["compiler.parser"]);
}

#[test]
fn import_in_doc_block_is_skipped() {
    let src = "\
import compiler.ast

#[doc(\"\"\"
Usage:

import compiler.parser
\"\"\")]
pub fn parse() : Unit = ()
";
    assert_eq!(extract(src), vec!["compiler.ast"]);
}

#[test]
fn import_in_multi_line_string_is_skipped() {
    let src = "\
import compiler.ast

fn sample() : String = \"\"\"
import compiler.parser
\"\"\"
";
    assert_eq!(extract(src), vec!["compiler.ast"]);
}

#[test]
fn import_inside_single_line_string_is_skipped() {
    let src = "let s = \"import compiler.ast\"\n";
    assert!(extract(src).is_empty());
}

#[test]
fn names_starting_with_import_are_not_imports() {
    let src = "importer.run()\nimport_all(xs)\nimport ?\n";
    assert!(extract(src).is_empty());
}

#[test]
fn import_with_an_empty_segment_is_not_an_import() {
    let src = "import a..b\nimport .a\nimport ..\n";
    assert!(extract(src).is_empty());
}

#[test]
fn crlf_line_endings_keep_one_import_per_line() {
    let src = "import a.b\r\nimport c\r\n\r\nfn main() : Unit = ()\r\n";
    assert_eq!(extract(src), vec!["a.b", "c"]);
}

// ── resolution ───────────────────────────────────────────────────────────────

fn kai_files(items: &[&str]) -> HashSet<PathBuf> {
    items.iter().map(PathBuf::from).collect()
}

fn resolve_kai(importer: &str, module: &str, file_set: &HashSet<PathBuf>) -> Vec<PathBuf> {
    resolve(Path::new(importer), module, file_set)
}

#[test]
fn resolve_dotted_path_is_relative_to_the_package_root() {
    // A sibling is reached through the root (`stage2/`), not through the
    // importing file's own directory.
    let file_set = kai_files(&["stage2/compiler/infer.kai", "stage2/compiler/ast.kai"]);
    let result = resolve_kai("stage2/compiler/infer.kai", "compiler.ast", &file_set);
    assert_eq!(result, vec![PathBuf::from("stage2/compiler/ast.kai")]);
}

#[test]
fn resolve_root_is_the_analysed_directory() {
    let file_set = kai_files(&["app/main.kai", "app/util/text.kai"]);
    let result = resolve_kai("app/main.kai", "app.util.text", &file_set);
    assert_eq!(result, vec![PathBuf::from("app/util/text.kai")]);
}

#[test]
fn resolve_single_segment_sibling() {
    let file_set = kai_files(&["pkg/main.kai", "pkg/shapes.kai"]);
    let result = resolve_kai("pkg/main.kai", "shapes", &file_set);
    assert_eq!(result, vec![PathBuf::from("pkg/shapes.kai")]);
}

#[test]
fn resolve_nearest_root_wins() {
    let file_set = kai_files(&["a/main.kai", "a/util.kai", "util.kai"]);
    let result = resolve_kai("a/main.kai", "util", &file_set);
    assert_eq!(result, vec![PathBuf::from("a/util.kai")]);
}

#[test]
fn resolve_external_import_resolves_to_nothing() {
    let file_set = kai_files(&["stage2/compiler/infer.kai", "stage2/compiler/ast.kai"]);
    assert!(resolve_kai("stage2/compiler/infer.kai", "core.list", &file_set).is_empty());
    assert!(resolve_kai("stage2/compiler/infer.kai", "loop", &file_set).is_empty());
}

#[test]
fn resolve_package_directory() {
    let file_set = kai_files(&[
        "consumer/main.kai",
        "mid/kai.toml",
        "mid/mid.kai",
        "mid/helpers.kai",
        "mid/notes.md",
        "mid/internal/deep.kai",
    ]);
    let result = resolve_kai("consumer/main.kai", "mid", &file_set);
    assert_eq!(
        result,
        vec![
            PathBuf::from("mid/helpers.kai"),
            PathBuf::from("mid/mid.kai")
        ]
    );
}

#[test]
fn resolve_file_wins_over_package_directory() {
    let file_set = kai_files(&["main.kai", "mid.kai", "mid/kai.toml", "mid/mid.kai"]);
    let result = resolve_kai("main.kai", "mid", &file_set);
    assert_eq!(result, vec![PathBuf::from("mid.kai")]);
}

#[test]
fn resolve_directory_without_manifest_is_not_a_package() {
    // `import loop` names an external module; the files that merely live
    // in a directory called `loop/` do not depend on each other.
    let file_set = kai_files(&["sugars/loop/while.kai", "sugars/loop/until.kai"]);
    assert!(resolve_kai("sugars/loop/while.kai", "loop", &file_set).is_empty());
}

#[test]
fn resolve_package_import_skips_the_importer() {
    let file_set = kai_files(&["notes/kai.toml", "notes/main.kai", "notes/store.kai"]);
    let result = resolve_kai("notes/main.kai", "notes", &file_set);
    assert_eq!(result, vec![PathBuf::from("notes/store.kai")]);
}
