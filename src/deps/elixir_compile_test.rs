use super::*;

fn read(code: &str) -> CompileTime {
    let (aliases, used) = Aliases::of(&code_only(code));
    CompileTime::of(&used, code, &aliases)
}

#[test]
fn uses_and_calls_outside_quote_blocks_are_read() {
    let found = read(
        "defmodule A do\n  use Web.Component\n  require Logger\n  import Web.Forms, only: [input: 1]\n  alias Web.Macros, as: M\n  def f(x), do: M.check(x) |> input() |> label()\n\n  defmacro __using__(_) do\n    quote location: :keep do\n      use Web.Pipeline\n      Web.Inner.run()\n    end\n  end\nend\n",
    );
    assert_eq!(found.uses, ["Web.Component"]);
    let calls: Vec<(&str, &str)> = found
        .calls
        .iter()
        .map(|c| (c.module.as_str(), c.function.as_deref().unwrap()))
        .collect();
    assert_eq!(calls, [("Web.Forms", "input"), ("Web.Macros", "check")]);
    assert_eq!(found.bare, ["input", "label"]);
}

#[test]
fn a_quote_bound_to_a_name_or_inline_is_told() {
    assert!(opens_quote("quote do"));
    assert!(opens_quote("ast = quote bind_quoted: [x: x] do"));
    assert!(!opens_quote("quote(do: x)"));
    assert!(!opens_quote("misquote do"));
    let code = "  ast = quote do\n    use X\n  end\n  use Y\n";
    assert_eq!(outside_quotes(code), "  ast = quote do\n\n  end\n  use Y\n");
}

#[test]
fn macros_are_the_public_macros_and_guards() {
    let source = "defmodule M do\n  defmacro check(x), do: x\n  defmacrop hidden(x), do: x\n  defguard is_even(n) when rem(n, 2) == 0\n  def run(x), do: x\n  # defmacro commented(x), do: x\nend\n";
    assert_eq!(macros(source), ["check", "is_even"]);
}

#[test]
fn needed_files_are_those_used_and_those_whose_macros_are_called() {
    let found = read(
        "defmodule A do\n  use Web.Component\n  import Web.Guards\n  import Web.Helpers\n  def f(x) when is_even(x), do: Web.Macros.check(x) |> Web.Plain.run()\nend\n",
    );
    let files = [
        "Web.Component",
        "Web.Guards",
        "Web.Helpers",
        "Web.Macros",
        "Web.Plain",
    ];
    let resolve = |m: &String| files.iter().position(|f| f == m);
    let macros: [&[String]; 5] = [
        &[],
        &["is_even".to_string()],
        &["unused".to_string()],
        &["check".to_string()],
        &["run2".to_string()],
    ];
    let whole = ["Web.Guards".to_string(), "Web.Helpers".to_string()];
    let mut needed = found.needed(&whole, &resolve, &|to| macros[to]);
    needed.sort();
    assert_eq!(needed, [0, 1, 3]);
}
