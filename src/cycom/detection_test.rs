use super::*;

#[test]
fn extract_name_rust_function() {
    let m = crate::cycom::markers::markers_for("Rust").unwrap();
    assert_eq!(extract_function_name("fn foo() {", m), "foo");
    assert_eq!(
        extract_function_name("pub fn bar(x: i32) -> bool {", m),
        "bar"
    );
    assert_eq!(extract_function_name("fn main() {", m), "main");
}

#[test]
fn extract_name_python_function() {
    let m = crate::cycom::markers::markers_for("Python").unwrap();
    assert_eq!(extract_function_name("def foo():", m), "foo");
    assert_eq!(extract_function_name("def bar(x, y):", m), "bar");
}

#[test]
fn extract_name_c_family_heuristic() {
    let m = crate::cycom::markers::markers_for("C").unwrap();
    assert_eq!(extract_function_name("int main(int argc) {", m), "main");
    assert_eq!(
        extract_function_name("void *process(void *arg) {", m),
        "process"
    );
}

#[test]
fn extract_name_pointer_return_type() {
    let m = crate::cycom::markers::markers_for("C").unwrap();
    assert_eq!(
        extract_function_name("char *get_name(int id) {", m),
        "get_name"
    );
}

#[test]
fn extract_name_anonymous_fallback() {
    let m = crate::cycom::markers::markers_for("C").unwrap();
    assert_eq!(extract_function_name("something_weird {", m), "<anonymous>");
}

#[test]
fn extract_name_macro_like() {
    let m = crate::cycom::markers::markers_for("C").unwrap();
    assert_eq!(
        extract_function_name("DEFINE_TEST(my_test) {", m),
        "DEFINE_TEST"
    );
}
