use super::*;

/// The functions of a source, as names and their first and last lines
/// (1-based).
fn units(
    detect: for<'a> fn(&[(usize, &'a str)]) -> Vec<FunctionBody<'a>>,
    src: &str,
) -> Vec<(String, usize, usize)> {
    let lines: Vec<(usize, &str)> = src.lines().enumerate().collect();
    detect(&lines)
        .into_iter()
        .map(|f| {
            let last = f.code_lines.last().unwrap().0 + 1;
            (f.name, f.start_line, last)
        })
        .collect()
}

fn named(name: &str, first: usize, last: usize) -> (String, usize, usize) {
    (name.to_string(), first, last)
}

#[test]
fn fortran_units_end_at_their_end() {
    let src = "module m\ncontains\n  pure function mean(xs) result(m)\n    m = 1\n  end function mean\n  recursive subroutine walk(n)\n    if (n > 0) then\n      call walk(n - 1)\n    endif\n  end subroutine\nend module m\n";
    assert_eq!(
        units(fortran, src),
        [named("mean", 3, 5), named("walk", 6, 10)]
    );
}

#[test]
fn fortran_fixed_form_units_end_at_a_bare_end() {
    let src = "      program main\n      call work(1)\n      end\n      subroutine work(n)\n   10 continue\n      end\n";
    assert_eq!(
        units(fortran, src),
        [named("main", 1, 3), named("work", 4, 6)]
    );
}

#[test]
fn fortran_typed_functions_and_contained_units() {
    let src = "real(kind=8) function f(x)\n  f = g(x)\ncontains\n  integer function g(y)\n    g = y\n  end function g\nend function f\ninteger*4 function h()\nend\n";
    assert_eq!(units(fortran, src), [named("f", 1, 7), named("h", 8, 9)]);
}

#[test]
fn fortran_interface_blocks_declare_no_functions() {
    let src = "interface\n  subroutine ext(x)\n  end subroutine\nend interface\nsubroutine s()\nend subroutine\n";
    assert_eq!(units(fortran, src), [named("s", 5, 6)]);
}

#[test]
fn fortran_words_in_strings_and_statements_declare_nothing() {
    let src = "print *, 'function f'\nend function f\ncall subroutine_x()\n";
    assert!(units(fortran, src).is_empty());
}

#[test]
fn pascal_routines_of_the_implementation() {
    let src = "unit u;\ninterface\nprocedure helper(x: integer);\nimplementation\nprocedure helper(x: integer);\nbegin\n  writeln(x);\nend;\nend.\n";
    assert_eq!(units(pascal, src), [named("helper", 5, 8)]);
}

#[test]
fn pascal_methods_declared_in_a_class_have_no_body_there() {
    let src = "type\n  tcounter = class(tobject)\n    procedure inc;\n    function value: integer;\n  end;\n  tpoint = record x, y: integer; end;\nprocedure tcounter.inc;\nbegin\nend;\n";
    assert_eq!(units(pascal, src), [named("tcounter.inc", 7, 9)]);
}

#[test]
fn pascal_local_routines_and_blocks_belong_to_the_routine() {
    let src = "procedure outer;\nvar\n  r: record a: integer; end;\n  procedure inner;\n  begin\n  end;\nbegin\n  case r.a of\n    1: try inner; finally end;\n  end;\nend;\nfunction next: integer;\nbegin\nend;\n";
    assert_eq!(
        units(pascal, src),
        [named("outer", 1, 11), named("next", 12, 14)]
    );
}

#[test]
fn pascal_forward_and_external_declarations_have_no_body() {
    let src =
        "procedure a; forward;\nfunction b: integer; external 'lib';\nprocedure a;\nbegin\nend;\n";
    assert_eq!(units(pascal, src), [named("a", 3, 5)]);
}

#[test]
fn pascal_forward_class_and_metaclass_open_no_type() {
    let src = "type\n  tfoo = class;\n  tfooclass = class of tfoo;\n  tbar = class(tfoo);\nprocedure p;\nbegin\nend;\n";
    assert_eq!(units(pascal, src), [named("p", 5, 7)]);
}

#[test]
fn pascal_sections_close_what_conditional_compilation_left_open() {
    // Each branch opens a record; only one `end` closes them
    let src = "interface\ntype\n{$ifdef fpc}\n  tfoo = record\n{$else}\n  tfoo = object\n{$endif}\n    a: integer;\n  end;\nimplementation\nprocedure p;\nbegin\nend;\n";
    assert_eq!(units(pascal, src), [named("p", 11, 13)]);
}

#[test]
fn pascal_an_unindented_routine_ends_an_unbalanced_one() {
    let src = "procedure a;\nbegin\n{$ifdef x}\n  if b then begin\n{$else}\n  if c then begin\n{$endif}\n  end;\nend;\nprocedure d;\nbegin\nend;\n";
    assert_eq!(units(pascal, src), [named("a", 1, 9), named("d", 10, 12)]);
}

#[test]
fn pascal_headers_under_conditional_compilation_are_one_routine() {
    let src = "{$ifdef fpc}\nprocedure t.call(a: integer);\n{$else}\nprocedure t.call(a: int64);\n{$endif}\nbegin\nend;\nprocedure next;\nbegin\nend;\n";
    assert_eq!(
        units(pascal, src),
        [named("t.call", 2, 7), named("next", 8, 10)]
    );
}
