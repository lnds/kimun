use super::*;

fn types(declared: &[(&str, usize)]) -> Types {
    let mut types = Types::default();
    for (name, file) in declared {
        types.declare(name.to_string(), *file);
    }
    types
}

#[test]
fn a_simple_name_is_found_in_the_namespaces_seen_whole() {
    let types = types(&[("app.models.User", 1), ("app.web.User", 2)]);
    let scope = Scope {
        whole: vec!["app.web".to_string()],
        ..Scope::default()
    };
    assert_eq!(types.resolve(&["User", "find"], &scope), [2]);
    assert!(types.resolve(&["Order"], &scope).is_empty());
}

#[test]
fn an_alias_names_a_type_or_a_namespace() {
    let types = types(&[("app.models.User", 1), ("app.models.Order", 3)]);
    let mut scope = Scope::default();
    scope
        .aliases
        .insert("U".to_string(), "app.models.User".to_string());
    scope
        .aliases
        .insert("M".to_string(), "app.models".to_string());
    assert_eq!(types.resolve(&["U"], &scope), [1]);
    assert_eq!(types.resolve(&["M", "Order", "Of"], &scope), [3]);
}

#[test]
fn a_qualified_name_is_read_whole_or_from_an_enclosing_namespace() {
    let types = types(&[("app.models.User", 1)]);
    let scope = Scope {
        enclosing: vec!["app.web".to_string(), "app".to_string()],
        ..Scope::default()
    };
    assert_eq!(types.resolve(&["app", "models", "User"], &scope), [1]);
    assert_eq!(types.resolve(&["models", "User"], &scope), [1]);
}

#[test]
fn a_nested_type_or_a_member_is_held_by_its_outer_type() {
    let types = types(&[("a.Outer", 4)]);
    assert_eq!(types.holding("a.Outer.Inner.Deeper"), [4]);
    assert!(types.holding("b.Other").is_empty());
}

#[test]
fn the_unnamed_namespace_qualifies_nothing() {
    assert_eq!(qualified("", "Main"), "Main");
    assert_eq!(qualified("a.b", "C"), "a.b.C");
}
