use super::*;

fn depths(code: &str) -> Vec<usize> {
    let lines: Vec<&str> = code.lines().collect();
    structural_depths(&lines, 4)
}

#[test]
fn aligned_arguments_are_one_level_deeper_than_their_call() {
    let code = "\
int load(const char *path) {
    if (!f) {
        return report(\"Could not open %s (flags %d)\",
                                          path,
                                          flags);
    }
    return 0;
}";
    assert_eq!(depths(code), [0, 1, 2, 3, 3, 1, 1, 0]);
}

#[test]
fn block_style_parameters_keep_their_depth() {
    let code = "\
fn f(
    a: u32,
    b: u32,
) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}";
    assert_eq!(depths(code), [0, 1, 1, 0, 1, 2, 1, 2, 1, 0]);
}

#[test]
fn a_closure_inside_a_call_nests_again() {
    let code = "\
describe('x', () => {
    it('y', () => {
        if (a) {
            b();
        }
    });
});";
    assert_eq!(depths(code), [0, 1, 2, 3, 2, 1, 0]);
}

#[test]
fn elixir_fn_inside_a_call_nests_again() {
    let code = "\
Enum.map(list, fn x ->
    case x do
        1 -> :one
        _ -> :other
    end
end)";
    assert_eq!(depths(code), [0, 1, 2, 2, 1, 0]);
}

#[test]
fn lines_after_or_starting_with_an_operator_continue_the_statement() {
    let code = "\
defmodule Serializer do
  @callback encode!(term()) ::
              {:push, :text, iodata()}
              | {:push, :binary, iodata()}
end";
    // Two spaces are no level with a width of 4; the continuation lines
    // are one level below the `@callback`, not three.
    assert_eq!(depths(code), [0, 0, 1, 1, 0]);
}

#[test]
fn method_chains_continue_the_statement() {
    let code = "\
let v = items
                .iter()
                .map(f)
                .collect();
let w = 1;";
    assert_eq!(depths(code), [0, 1, 1, 1, 0]);
}

#[test]
fn brackets_in_literals_and_comments_count_for_nothing() {
    let code = "\
fn f() {
    let s = \"(((\"; // ((
    let c = '(';
    g();
}";
    assert_eq!(depths(code), [0, 1, 1, 1, 0]);
}
