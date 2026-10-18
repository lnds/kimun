use crate::loc::counter::LineKind;

use super::*;

fn lines(s: &str) -> Vec<String> {
    s.lines().map(String::from).collect()
}

#[test]
fn commented_out_code_detected() {
    let ls = lines("// let x = 1;\n// if (y > 0) {\n// return z;\n// }");
    let kinds = vec![
        LineKind::Comment,
        LineKind::Comment,
        LineKind::Comment,
        LineKind::Comment,
    ];
    let smells = detect_commented_out_code(&ls, &kinds);
    assert_eq!(smells.len(), 1);
    assert_eq!(smells[0].kind, SmellKind::CommentedOutCode);
}

#[test]
fn regular_comments_not_detected() {
    let ls = lines("// This is a description\n// of the module behavior");
    let kinds = vec![LineKind::Comment, LineKind::Comment];
    let smells = detect_commented_out_code(&ls, &kinds);
    assert!(smells.is_empty());
}

#[test]
fn single_commented_code_line_not_detected() {
    let ls = lines("// let x = 1;");
    let kinds = vec![LineKind::Comment];
    let smells = detect_commented_out_code(&ls, &kinds);
    assert!(smells.is_empty());
}

#[test]
fn doc_comment_with_use_not_false_positive() {
    let ls = lines("/// use std::io::Read;\n/// use tokio::runtime::Runtime;");
    let kinds = vec![LineKind::Comment, LineKind::Comment];
    let smells = detect_commented_out_code(&ls, &kinds);
    assert!(
        smells.is_empty(),
        "doc comments with `use` should not trigger"
    );
}

fn kinds_of(src: &str) -> Vec<LineKind> {
    src.lines()
        .map(|l| {
            let t = l.trim_start();
            let comment = ["//", "/*", "*", "#"].iter().any(|m| t.starts_with(m));
            if t.is_empty() {
                LineKind::Blank
            } else if comment {
                LineKind::Comment
            } else {
                LineKind::Code
            }
        })
        .collect()
}

fn found(src: &str) -> Vec<usize> {
    detect_commented_out_code(&lines(src), &kinds_of(src))
        .into_iter()
        .map(|s| s.line)
        .collect()
}

#[test]
fn jsdoc_and_javadoc_blocks_are_documentation() {
    let src = "\
/**
 * @param {String|Buffer} body
 * @param {String} [encoding]
 * @return {String}
 * see {@link #nextName()} and {@link #skipValue()};
 */
exports.etag = function (body, encoding) {";
    assert!(found(src).is_empty());
}

#[test]
fn rust_doctests_and_csharp_xml_doc_are_documentation() {
    let src = "\
/*!
let matcher = RegexMatcher::new(r\"Sherlock\")?;
let mut printer = Standard::new_no_color(vec![]);
*/
/// let m = Match::new(2, 5);
/// let bytes = b\"abcdefghi\";
/// <returns>An <see cref=\"IEnumerable{T}\"/> of the ancestors.</returns>
pub fn value() -> u32 {";
    assert!(found(src).is_empty());
}

#[test]
fn go_doc_example_above_a_declaration_is_documentation() {
    let src = "\
// ShouldBindJSON is a shortcut.
//
//\tif err := c.ShouldBindJSON(&user); err != nil {
//\t\treturn
//\t}
func ShouldBindJSON() bool {";
    assert!(found(src).is_empty());
}

#[test]
fn tab_indented_code_inside_a_function_is_still_commented_out() {
    let src = "\
	x := 1
//\tif err := c.ShouldBindJSON(&user); err != nil {
//\t\treturn x;
//\t}
	return x";
    assert_eq!(found(src), [2]);
}

#[test]
fn plain_comment_with_code_is_still_reported() {
    let src = "\
// let x = compute();
// if (x > 0) { return x; }
fn f() {";
    assert_eq!(found(src), [1]);
}
