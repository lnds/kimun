use super::*;
use crate::loc::counter::LineKind;

fn rust_markers() -> &'static CognitiveMarkers {
    super::super::markers::cognitive_markers_for("Rust").unwrap()
}

fn python_markers() -> &'static CognitiveMarkers {
    super::super::markers::cognitive_markers_for("Python").unwrap()
}

#[test]
fn extract_rust_function_name() {
    let m = rust_markers();
    assert_eq!(extract_function_name("fn foo() {", m), "foo");
    assert_eq!(extract_function_name("fn bar_baz() {", m), "bar_baz");
    assert_eq!(
        extract_function_name("pub fn my_func(x: i32) {", m),
        "my_func"
    );
}

#[test]
fn extract_python_function_name() {
    let m = python_markers();
    assert_eq!(extract_function_name("def foo():", m), "foo");
    assert_eq!(extract_function_name("async def bar():", m), "bar");
}

#[test]
fn extract_c_family_name() {
    let m = super::super::markers::cognitive_markers_for("C").unwrap();
    assert_eq!(extract_function_name("int main(int argc) {", m), "main");
}

#[test]
fn detect_rust_functions() {
    let lines: Vec<String> = "fn foo() {\n    bar();\n}\nfn baz() {\n    qux();\n}\n"
        .lines()
        .map(String::from)
        .collect();
    let kinds = vec![LineKind::Code; lines.len()];
    let code_lines: Vec<(usize, &str)> = lines
        .iter()
        .zip(kinds.iter())
        .enumerate()
        .filter(|(_, (_, k))| **k == LineKind::Code)
        .map(|(i, (l, _))| (i, l.as_str()))
        .collect();

    let functions = detect_functions(&lines, &code_lines, rust_markers());
    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "foo");
    assert_eq!(functions[1].name, "baz");
}

#[test]
fn function_like_define_macro_not_treated_as_function() {
    // Regression for issue #36: a function-like `#define NAME(args) body`
    // resembles a C function declaration and was absorbing the cognitive
    // complexity of the code that followed it (here: `classify`), hiding the
    // real function from the per-function breakdown.
    let src = "#define TRACE_ALLOC(name) trace_record(name)\n\
               #define TRACE_FREE(name)  trace_record(name)\n\
               static int classify(int x) {\n\
               if (x < 0) return -1;\n\
               else if (x == 0) return 0;\n\
               return 1;\n\
               }\n";
    let lines: Vec<String> = src.lines().map(String::from).collect();
    let kinds = vec![LineKind::Code; lines.len()];
    let code_lines: Vec<(usize, &str)> = lines
        .iter()
        .zip(kinds.iter())
        .enumerate()
        .filter(|(_, (_, k))| **k == LineKind::Code)
        .map(|(i, (l, _))| (i, l.as_str()))
        .collect();

    let m = super::super::markers::cognitive_markers_for("C").unwrap();
    let functions = detect_functions(&lines, &code_lines, m);
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0].name, "classify");
    assert!(functions[0].complexity > 0);
}

#[test]
fn detect_python_functions() {
    let lines: Vec<String> = "def foo():\n    bar()\n\ndef baz():\n    qux()\n"
        .lines()
        .map(String::from)
        .collect();
    let mut kinds = vec![LineKind::Code; lines.len()];
    kinds[2] = LineKind::Blank;
    let code_lines: Vec<(usize, &str)> = lines
        .iter()
        .zip(kinds.iter())
        .enumerate()
        .filter(|(_, (_, k))| **k == LineKind::Code)
        .map(|(i, (l, _))| (i, l.as_str()))
        .collect();

    let functions = detect_functions(&lines, &code_lines, python_markers());
    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "foo");
    assert_eq!(functions[1].name, "baz");
}
