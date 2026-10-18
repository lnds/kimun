use super::*;

fn read(code: &str) -> Directives {
    let (aliases, used) = Aliases::of(code);
    directives(&used, &aliases)
}

fn call(module: &str, function: &str) -> Reference {
    Reference {
        module: module.to_string(),
        function: Some(function.to_string()),
    }
}

#[test]
fn an_import_with_only_calls_the_functions_it_names() {
    let found = read(
        "defmodule A do\n  import Web.Helpers, only: [link: 2, button?: 1]\n  import Web.Forms,\n    only: [\n      input: 1,\n      {:label, 2}\n    ]\nend\n",
    );
    assert_eq!(
        found.calls,
        [
            call("Web.Forms", "input"),
            call("Web.Forms", "label"),
            call("Web.Helpers", "button?"),
            call("Web.Helpers", "link"),
        ]
    );
    assert!(found.whole.is_empty(), "{:?}", found.whole);
}

#[test]
fn what_may_reach_any_function_is_whole_and_uses_are_kept() {
    let found = read(
        "defmodule A do\n  alias Web.Helpers, as: H\n  import H\n  import Web.Forms, except: [input: 1]\n  import Web.Kinds, only: :functions\n  use Web.Component, layout: false\n  use Web, :live_view\nend\n\ndefimpl Web.Printable, for: A do\n  def print(_), do: :ok\nend\n",
    );
    assert!(found.calls.is_empty(), "{:?}", found.calls);
    assert_eq!(
        found.whole,
        ["Web.Forms", "Web.Helpers", "Web.Kinds", "Web.Printable"]
    );
    let uses: Vec<(&str, Option<&str>)> = found
        .uses
        .iter()
        .map(|u| (u.module.as_str(), u.option.as_deref()))
        .collect();
    assert_eq!(uses, [("Web", Some("live_view")), ("Web.Component", None)]);
}

#[test]
fn a_keyword_must_open_the_line_and_name_a_module() {
    let found = read(
        "defmodule A do\n  def f(use, imports), do: {use, imports}\n  import :lists\n  usefully = Web.X\nend\n",
    );
    assert_eq!(found, Directives::default());
}

#[test]
fn bare_calls_leave_out_qualified_and_local_ones() {
    let source = "defmodule A do\n  def run(x), do: x |> button() |> Enum.map(&label/1) |> own()\n  defp own(x), do: x.field(1)\nend\n";
    assert_eq!(bare_calls(source, source), ["button", "label"]);
}
