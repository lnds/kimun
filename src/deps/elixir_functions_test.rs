use super::*;

const CONTEXT: &str = r#"defmodule Billing.Invoices do
  @moduledoc """
  Invoices.
  """
  alias Billing.Repo

  @default_rate 19

  @doc """
  Approve an invoice.
  """
  @spec approve(map(), map()) :: {:ok, map()}
  def approve(invoice, user) do
    invoice
    |> check(user)
    |> record(:approved)
  end

  def reject(invoice, user), do: record(check(invoice, user), :rejected)

  def total(invoice), do: Enum.sum(invoice.lines) * rate()

  def list, do: Repo.all(Billing.Invoice)

  defp check(invoice, user) do
    if allowed?(user), do: invoice, else: raise("no")
  end

  defp allowed?(%{role: :admin}), do: true
  defp allowed?(_), do: false

  defp record(invoice, state) do
    Repo.insert!(%{invoice | state: state})
  end

  defp rate, do: @default_rate
end
"#;

/// The 1-based number of the first line of `CONTEXT` holding `needle`.
fn line_of(needle: &str) -> usize {
    CONTEXT
        .lines()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line with {needle:?}"))
        + 1
}

fn changed(needles: &[&str]) -> Result<Vec<String>, usize> {
    let lines: Vec<usize> = needles.iter().map(|n| line_of(n)).collect();
    changed_functions(CONTEXT, &lines).map(|names| names.into_iter().collect())
}

#[test]
fn functions_with_their_lines() {
    let found = functions(CONTEXT);
    let span = |name: &str| {
        let f = found.iter().find(|f| f.name == name).unwrap();
        (f.public, f.first_line, f.last_line)
    };

    // The doc and the spec above a function are part of it.
    assert_eq!(
        span("approve"),
        (true, line_of("@doc"), line_of("|> record(:approved)") + 1)
    );
    assert_eq!(span("reject").2, line_of("def reject"));
    assert_eq!(
        span("check"),
        (false, line_of("defp check"), line_of("defp check") + 2)
    );
    // Each clause is listed.
    assert_eq!(found.iter().filter(|f| f.name == "allowed?").count(), 2);
    let names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "approve", "reject", "total", "list", "check", "allowed?", "allowed?", "record", "rate"
        ]
    );
}

#[test]
fn a_change_inside_a_public_function_affects_it_alone() {
    assert_eq!(changed(&["|> check(user)"]).unwrap(), ["approve"]);
    assert_eq!(changed(&["def list"]).unwrap(), ["list"]);
    // Its documentation counts as part of it.
    assert_eq!(changed(&["Approve an invoice."]).unwrap(), ["approve"]);
    assert_eq!(changed(&["@spec approve"]).unwrap(), ["approve"]);
}

#[test]
fn a_change_to_a_private_function_affects_the_public_ones_that_reach_it() {
    // record is called by approve and reject.
    assert_eq!(changed(&["Repo.insert!"]).unwrap(), ["approve", "reject"]);
    // allowed? is called by check, which approve and reject call.
    assert_eq!(changed(&["role: :admin"]).unwrap(), ["approve", "reject"]);
    assert_eq!(changed(&["defp rate"]).unwrap(), ["total"]);
}

#[test]
fn a_change_that_may_concern_every_function_cannot_be_narrowed() {
    // The line is told, to say why.
    assert_eq!(changed(&["defmodule Billing.Invoices"]), Err(1));

    let with_use = "defmodule A do\n  use GenServer\n  import Ecto.Query\n  defstruct [:id]\n\n  def f, do: 1\nend\n";
    assert_eq!(changed_functions(with_use, &[2]), Err(2));
    assert_eq!(changed_functions(with_use, &[3]), Err(3));
    assert_eq!(changed_functions(with_use, &[4]), Err(4));
    // Even next to a change inside a function: the first such line wins.
    assert_eq!(changed_functions(with_use, &[6, 4, 3]), Err(4));
}

#[test]
fn naming_what_the_functions_use_changes_no_function() {
    // An alias or a require says nothing by itself: the functions that use
    // the new name are touched too, and those are what counts.
    assert_eq!(
        changed(&["alias Billing.Repo"]).unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(
        changed(&["def list", "alias Billing.Repo"]).unwrap(),
        ["list"]
    );

    let grouped = "defmodule A do\n  alias B.{\n    C,\n    D\n  }\n  require Logger\n\n  def f, do: C.x()\nend\n";
    for line in 2..=6 {
        assert_eq!(
            changed_functions(grouped, &[line]).unwrap().len(),
            0,
            "line {line} should change no function"
        );
    }
}

#[test]
fn a_module_attribute_changes_the_functions_that_read_it() {
    // @default_rate is read by rate, which total calls.
    assert_eq!(changed(&["@default_rate 19"]).unwrap(), ["total"]);

    let source = "defmodule A do\n  @moduledoc false\n  @type t :: map()\n  @timeout 5_000\n  @timeouts %{a: 1}\n  @unused :x\n  @behaviour Runner\n\n  def wait, do: sleep(@timeout)\n  def all, do: @timeouts\n  def other, do: :ok\nend\n";
    // What describes the module or its types changes no function.
    assert_eq!(changed_functions(source, &[2, 3]).unwrap().len(), 0);
    // A longer name is another attribute.
    let names = |lines: &[usize]| {
        changed_functions(source, lines).map(|n| n.into_iter().collect::<Vec<_>>())
    };
    assert_eq!(names(&[4]).unwrap(), ["wait"]);
    assert_eq!(names(&[5]).unwrap(), ["all"]);
    // One that no function reads may be read by the compiler or a macro.
    assert_eq!(names(&[6]), Err(6));
    assert_eq!(names(&[7]), Err(7));
}

#[test]
fn a_change_to_comments_or_the_module_doc_affects_nothing() {
    assert_eq!(changed(&["Invoices."]).unwrap(), Vec::<String>::new());
    assert_eq!(changed_functions(CONTEXT, &[]).unwrap().len(), 0);
    // A line past the end of the file is ignored.
    assert_eq!(changed_functions(CONTEXT, &[9_999]).unwrap().len(), 0);
}

#[test]
fn a_local_call_is_told_from_a_remote_one_and_from_a_longer_name() {
    assert!(calls_local("x |> record(:a)", "record"));
    assert!(calls_local("Enum.map(xs, &record/1)", "record"));
    assert!(!calls_local("Other.record(x)", "record"));
    assert!(!calls_local("prerecord(x)", "record"));
    assert!(!calls_local("recorded(x)", "record"));
    assert!(!calls_local("record = 1", "record"));
    assert!(!calls_local("%{record: 1}", "record"));
}

const COMPONENTS: &str = r#"defmodule Web.Components do
  use Phoenix.Component

  attr :label, :string, required: true
  attr :rest, :global,
    include: ~w(disabled form)

  slot :inner_block
  slot :col do
    attr :title, :string
  end

  def button(assigns) do
    render(assigns)
  end

  attr(:kind, :atom, default: :info)

  def badge(assigns), do: render(assigns)

  def fetch(id) do
    load(id)
  rescue
    error -> handle(error)
  after
    cleanup()
  end

  def after_fetch, do: :ok
end
"#;

fn component_line(needle: &str) -> usize {
    COMPONENTS.lines().position(|l| l.contains(needle)).unwrap() + 1
}

fn changed_component(needle: &str) -> Result<Vec<String>, usize> {
    changed_functions(COMPONENTS, &[component_line(needle)]).map(|n| n.into_iter().collect())
}

#[test]
fn the_assigns_a_component_declares_are_part_of_it() {
    for needle in [
        "attr :label",
        "include: ~w(disabled form)",
        "slot :inner_block",
        "slot :col do",
        "attr :title",
    ] {
        assert_eq!(changed_component(needle).unwrap(), ["button"], "{needle}");
    }
    assert_eq!(changed_component("attr(:kind").unwrap(), ["badge"]);
    // What the module uses is still the module's.
    assert_eq!(changed_component("use Phoenix.Component"), Err(2));
}

#[test]
fn a_rescue_at_the_indentation_of_a_function_is_part_of_it() {
    for needle in ["load(id)", "rescue", "handle(error)", "cleanup()"] {
        assert_eq!(changed_component(needle).unwrap(), ["fetch"], "{needle}");
    }
    assert_eq!(
        changed_component("def after_fetch").unwrap(),
        ["after_fetch"]
    );

    let fetch = functions(COMPONENTS)
        .into_iter()
        .find(|f| f.name == "fetch")
        .unwrap();
    assert_eq!(fetch.first_line, component_line("def fetch"));
    assert_eq!(fetch.last_line, component_line("cleanup()") + 1);
}

#[test]
fn a_declaration_above_does_not_reach_past_another_function() {
    let source = "defmodule A do\n  attr :x, :string\n\n  def one(assigns),\n    do: assigns\n\n  def two(assigns), do: assigns\nend\n";
    let spans: Vec<(String, usize, usize)> = functions(source)
        .into_iter()
        .map(|f| (f.name, f.first_line, f.last_line))
        .collect();
    assert_eq!(
        spans,
        [("one".to_string(), 2, 5), ("two".to_string(), 7, 7)]
    );
}
