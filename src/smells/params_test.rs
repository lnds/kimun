use crate::detection::{FunctionDetectionMarkers, FunctionSyntax};
use crate::loc::counter::LineKind;

use super::*;

/// Minimal markers for testing (Rust-like).
struct TestMarkers;

impl FunctionDetectionMarkers for TestMarkers {
    fn function_markers(&self) -> &[&str] {
        &["fn "]
    }
    fn brace_scoped(&self) -> bool {
        true
    }
    fn function_syntax(&self) -> FunctionSyntax {
        FunctionSyntax::KeywordWithLifetimes
    }
    fn line_comments(&self) -> &[&str] {
        &["//"]
    }
}

fn lines(s: &str) -> Vec<String> {
    s.lines().map(String::from).collect()
}

fn all_code(n: usize) -> Vec<LineKind> {
    vec![LineKind::Code; n]
}

// ── Long function ──

#[test]
fn long_params_detected() {
    let src = "fn baz(a: i32, b: i32, c: i32, d: i32, e: i32) {\n}\n";
    let ls = lines(src);
    let kinds = all_code(ls.len());
    let smells = detect_long_params(&ls, &kinds, &TestMarkers, 4);
    assert_eq!(smells.len(), 1);
    assert_eq!(smells[0].kind, SmellKind::LongParameterList);
    assert!(smells[0].detail.contains("5 params"));
}

#[test]
fn long_params_multiline_detected() {
    let src =
        "fn process(\n    a: i32,\n    b: i32,\n    c: i32,\n    d: i32,\n    e: i32,\n) {\n}\n";
    let ls = lines(src);
    let kinds = all_code(ls.len());
    let smells = detect_long_params(&ls, &kinds, &TestMarkers, 4);
    assert_eq!(smells.len(), 1, "multi-line signature should be detected");
    assert!(smells[0].detail.contains("5 params"));
}

#[test]
fn few_params_not_detected() {
    let src = "fn qux(a: i32, b: i32) {\n}\n";
    let ls = lines(src);
    let kinds = all_code(ls.len());
    let smells = detect_long_params(&ls, &kinds, &TestMarkers, 4);
    assert!(smells.is_empty());
}

#[test]
fn no_params_not_detected() {
    let src = "fn empty() {\n}\n";
    let ls = lines(src);
    let kinds = all_code(ls.len());
    let smells = detect_long_params(&ls, &kinds, &TestMarkers, 4);
    assert!(smells.is_empty());
}

// ── TODO/FIXME debt ──

/// The parameter count reported for each function of `src` in `language`.
fn reported(language: &str, src: &str) -> Vec<String> {
    let markers = crate::cycom::markers::markers_for(language).unwrap();
    let ls = lines(src);
    let kinds = all_code(ls.len());
    detect_long_params(&ls, &kinds, markers, 0)
        .into_iter()
        .map(|s| s.detail)
        .collect()
}

#[test]
fn commas_inside_type_arguments_separate_no_parameters() {
    let src = "\
class Node<K, V> {
  Node(boolean allowNull, Node<K, V> parent, K key, Node<K, V> next, Node<K, V> prev) {
    this.parent = parent;
  }
}";
    assert_eq!(
        reported("Java", src),
        ["function `Node` has 5 params (max 0)"]
    );
}

#[test]
fn commas_inside_patterns_separate_no_parameters() {
    let src = "\
defmodule S do
  def handle_in(%{\"topic\" => topic, \"event\" => event, \"ref\" => ref}, state) do
    {topic, event, ref, state}
  end
  defp decode_binary(<<
         push::size(8),
         rest::binary
       >>) do
    {push, rest}
  end
end";
    assert_eq!(
        reported("Elixir", src),
        [
            "function `handle_in` has 2 params (max 0)",
            "function `decode_binary` has 1 params (max 0)"
        ]
    );
}

#[test]
fn parameters_are_those_after_the_name() {
    let src = "\
defineGetter(req, 'protocol', function protocol(){
  return this.socket.encrypted ? 'https' : 'http';
});";
    assert!(reported("JavaScript", src).is_empty());
}

#[test]
fn go_receiver_is_not_the_parameter_list() {
    let src = "\
func (c *Context) SetCookie(name, value string, maxAge int, path string) {
	c.name = name
}";
    assert_eq!(
        reported("Go", src),
        ["function `SetCookie` has 4 params (max 0)"]
    );
}

#[test]
fn arrows_in_parameter_types_do_not_close_type_arguments() {
    let src = "\
function apply(f: (x: number) => number, xs: Array<number>, n: number) {
  return xs.map(f)
}";
    assert_eq!(
        reported("TypeScript", src),
        ["function `apply` has 3 params (max 0)"]
    );
}
