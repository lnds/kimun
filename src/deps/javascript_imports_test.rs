use super::super::graph::EdgeKind::{self, Deferred, Load, Type};
use super::super::javascript::imports;

fn kinds(source: &str) -> Vec<(String, EdgeKind)> {
    imports(source)
}

fn one(source: &str) -> EdgeKind {
    let found = kinds(source);
    assert_eq!(found.len(), 1, "{source}: {found:?}");
    found[0].1
}

#[test]
fn import_type_takes_only_types() {
    assert_eq!(one("import type { B } from './b'"), Type);
    assert_eq!(one("import type B from './b'"), Type);
    assert_eq!(one("import type * as b from './b'"), Type);
}

#[test]
fn every_name_marked_type_takes_only_types() {
    assert_eq!(one("import { type A, type B } from './b'"), Type);
    assert_eq!(one("import { type A as X } from './b'"), Type);
}

#[test]
fn one_value_among_types_loads() {
    assert_eq!(one("import { type A, b } from './b'"), Load);
    assert_eq!(one("import X, { type A } from './b'"), Load);
    assert_eq!(one("import { type as t } from './b'"), Load);
}

#[test]
fn a_binding_named_type_loads() {
    assert_eq!(one("import type from './b'"), Load);
    assert_eq!(one("import type, { a } from './b'"), Load);
}

#[test]
fn reexports_of_types_take_only_types() {
    assert_eq!(one("export type { A } from './a'"), Type);
    assert_eq!(one("export type * from './a'"), Type);
    assert_eq!(one("export { type A } from './a'"), Type);
    assert_eq!(one("export { A } from './a'"), Load);
    assert_eq!(one("export * as a from './a'"), Load);
}

#[test]
fn plain_imports_load() {
    assert_eq!(one("import './side'"), Load);
    assert_eq!(one("import a from './a'"), Load);
    assert_eq!(one("import * as a from './a'"), Load);
    assert_eq!(one("const a = require('./a')"), Load);
    assert_eq!(one("import a = require('./a')"), Load);
}

#[test]
fn dynamic_import_is_deferred() {
    assert_eq!(one("const a = await import('./a')"), Deferred);
}

#[test]
fn a_local_export_list_is_no_import() {
    let found = kinds("export { a, b }\nimport type { C } from './c'\n");
    assert_eq!(found, vec![("./c".to_string(), Type)]);
}

#[test]
fn imports_over_several_lines() {
    let source = "import {\n  type A,\n  type B,\n} from './ab'\nimport {\n  c,\n} from './c'\n";
    assert_eq!(
        kinds(source),
        vec![("./ab".to_string(), Type), ("./c".to_string(), Load)]
    );
}
