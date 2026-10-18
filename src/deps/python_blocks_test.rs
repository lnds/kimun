use super::super::graph::EdgeKind::{self, Deferred, Load, Type};
use super::super::python::read;

/// The kind of each binding of `source`, by module.
fn kinds(source: &str) -> Vec<(String, EdgeKind)> {
    read(source)
        .bindings
        .into_iter()
        .map(|b| (b.module, b.kind))
        .collect()
}

fn pairs(expected: &[(&str, EdgeKind)]) -> Vec<(String, EdgeKind)> {
    expected.iter().map(|(m, k)| (m.to_string(), *k)).collect()
}

#[test]
fn top_level_imports_load() {
    assert_eq!(
        kinds("import a\nfrom b import c\n"),
        pairs(&[("a", Load), ("b", Load)])
    );
}

#[test]
fn imports_under_type_checking_take_only_types() {
    let source = "\
from typing import TYPE_CHECKING
import typing

if TYPE_CHECKING:
    from pkg.ctx import Context
if typing.TYPE_CHECKING:
    import pkg.other
else:
    import pkg.runtime
import pkg.after
";
    assert_eq!(
        kinds(source),
        pairs(&[
            ("typing", Load),
            ("typing", Load),
            ("pkg.ctx", Type),
            ("pkg.other", Type),
            ("pkg.runtime", Load),
            ("pkg.after", Load),
        ])
    );
}

#[test]
fn imports_inside_functions_are_deferred() {
    let source = "\
def current():
    from pkg.views import render
    return render()


class C:
    import pkg.on_load

    async def go(self):
        import pkg.later

import pkg.top
";
    assert_eq!(
        kinds(source),
        pairs(&[
            ("pkg.views", Deferred),
            ("pkg.on_load", Load),
            ("pkg.later", Deferred),
            ("pkg.top", Load),
        ])
    );
}

#[test]
fn a_signature_over_several_lines_keeps_the_function_open() {
    let source = "\
def f(
    a,
    b,
):
    import pkg.inner
x = g(
    1,
)
import pkg.outer
";
    assert_eq!(
        kinds(source),
        pairs(&[("pkg.inner", Deferred), ("pkg.outer", Load)])
    );
}

#[test]
fn a_bracket_inside_a_string_does_not_count() {
    let source = "def f():\n    s = \"(\"\nimport pkg.top\n";
    assert_eq!(kinds(source), pairs(&[("pkg.top", Load)]));
}

#[test]
fn not_type_checking_runs() {
    let source = "if not TYPE_CHECKING:\n    import pkg.a\n";
    assert_eq!(kinds(source), pairs(&[("pkg.a", Load)]));
}
