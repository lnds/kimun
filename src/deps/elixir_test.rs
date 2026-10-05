use super::*;

fn call(module: &str, function: &str) -> Reference {
    Reference {
        module: module.to_string(),
        function: Some(function.to_string()),
    }
}

fn module(name: &str) -> Reference {
    Reference {
        module: name.to_string(),
        function: None,
    }
}

/// The modules a file refers to, without the functions.
fn modules(source: &str) -> Vec<String> {
    let mut names: Vec<String> = parse(source).refs.into_iter().map(|r| r.module).collect();
    names.dedup();
    names
}

#[test]
fn a_module_and_the_calls_it_makes() {
    let file = parse(
        r#"
defmodule Billing.Invoice do
  def total(invoice) do
    Billing.Tax.apply(invoice.amount) + Billing.Fees.fixed()
  end
end
"#,
    );

    assert_eq!(file.defines, ["Billing.Invoice"]);
    assert_eq!(
        file.refs,
        [
            call("Billing.Fees", "fixed"),
            module("Billing.Invoice"),
            call("Billing.Tax", "apply"),
        ]
    );
}

#[test]
fn aliases_are_expanded() {
    let file = parse(
        r#"
defmodule Billing.Report do
  alias Billing.Invoice
  alias Billing.Tax, as: T
  alias Billing.Ledger.Entry, warn: false

  def run(id) do
    id |> Invoice.get!() |> T.apply()
    %Entry{}
  end
end
"#,
    );

    assert!(file.refs.contains(&call("Billing.Invoice", "get!")));
    assert!(file.refs.contains(&call("Billing.Tax", "apply")));
    assert!(file.refs.contains(&module("Billing.Ledger.Entry")));
    // The short names are not modules of their own.
    assert!(!modules("alias Billing.Invoice\nInvoice.get(1)").contains(&"Invoice".to_string()));
}

#[test]
fn a_grouped_alias_may_span_lines() {
    let source = r#"
defmodule Web.Controller do
  alias Billing.{
    Invoice,
    Ledger.Entry,
    Tax
  }
  alias Web.{View, Router}

  def show, do: {Invoice.get(1), Entry.new(), Tax.rate(), View.render(), Router.path()}
end
"#;
    let refs = parse(source).refs;
    for expected in [
        call("Billing.Invoice", "get"),
        call("Billing.Ledger.Entry", "new"),
        call("Billing.Tax", "rate"),
        call("Web.View", "render"),
        call("Web.Router", "path"),
    ] {
        assert!(refs.contains(&expected), "missing {expected:?} in {refs:?}");
    }
}

#[test]
fn an_alias_may_build_on_another() {
    let refs = parse("alias Billing.Ledger\nalias Ledger.Entry\nEntry.new()").refs;
    assert!(
        refs.contains(&call("Billing.Ledger.Entry", "new")),
        "{refs:?}"
    );
}

#[test]
fn current_module_stands_for_its_name() {
    let file = parse(
        r#"
defmodule Billing.Invoice do
  alias __MODULE__.Line
  alias __MODULE__, as: Self

  def lines(%__MODULE__{} = invoice), do: Line.of(invoice) ++ Self.extra()
end
"#,
    );
    assert!(file.refs.contains(&call("Billing.Invoice.Line", "of")));
    assert!(file.refs.contains(&call("Billing.Invoice", "extra")));
}

#[test]
fn nested_modules_take_the_name_of_the_outer_one() {
    let file = parse(
        r#"
defmodule Billing.Invoice do
  defmodule Line do
    defstruct [:amount]

    defmodule Tax do
      def of(_), do: __MODULE__
    end

    def new, do: %__MODULE__{}
  end

  defmodule Note do
  end

  def me, do: __MODULE__
end

defprotocol Billing.Printable do
  def print(data)
end
"#,
    );
    assert_eq!(
        file.defines,
        [
            "Billing.Invoice",
            "Billing.Invoice.Line",
            "Billing.Invoice.Line.Tax",
            "Billing.Invoice.Note",
            "Billing.Printable",
        ]
    );
    // Each __MODULE__ is the module its line is in.
    let names = modules(
        "defmodule A do\n  defmodule B do\n    def x, do: __MODULE__.Inner\n  end\n\n  def y, do: __MODULE__.Outer\nend\n",
    );
    assert!(names.contains(&"A.B.Inner".to_string()), "{names:?}");
    assert!(names.contains(&"A.Outer".to_string()), "{names:?}");
}

#[test]
fn directives_structs_and_captures_are_references() {
    let names = modules(
        r#"
defmodule Web.Page do
  use Web.Component
  import Web.Helpers, only: [link: 2]
  require Logger
  @behaviour Web.Renderable

  def render(%Billing.Invoice{} = i), do: Enum.map(i.lines, &Billing.Line.format/1)
end

defimpl Billing.Printable, for: Billing.Invoice do
  def print(_), do: :ok
end
"#,
    );
    for expected in [
        "Web.Component",
        "Web.Helpers",
        "Logger",
        "Web.Renderable",
        "Billing.Invoice",
        "Billing.Line",
        "Billing.Printable",
        "Enum",
    ] {
        assert!(
            names.contains(&expected.to_string()),
            "missing {expected} in {names:?}"
        );
    }
}

#[test]
fn comments_and_literals_hold_no_references() {
    let file = parse(
        r##"
defmodule Billing.Docs do
  @moduledoc """
  Use it like this:

      iex> Billing.Ghost.call()
  """

  # Billing.Commented.out()
  @doc ~S"""
  See `Billing.Sigil.doc/0`.
  """
  def text do
    a = "Billing.InString.x() and #{Billing.Interpolated.y("q")} too"
    b = 'Billing.Charlist.z()'
    c = ~s(Billing.Sigil.paren())
    d = ~r/Billing\.Regex/
    e = ~w[Billing.Words.A Billing.Words.B]
    f = ?B
    g = :"Billing.QuotedAtom"
    {a, b, c, d, e, f, g, Billing.Real.call()}
  end
end
"##,
    );
    assert_eq!(
        file.refs,
        [module("Billing.Docs"), call("Billing.Real", "call")]
    );
}

#[test]
fn a_question_mark_in_a_name_is_not_a_character_literal() {
    let refs = parse(
        "if Billing.Invoice.paid?(i), do: Billing.Ledger.close!(i)\nx = ?\\n\nBilling.After.run()",
    )
    .refs;
    assert_eq!(
        refs,
        [
            call("Billing.After", "run"),
            call("Billing.Invoice", "paid?"),
            call("Billing.Ledger", "close!"),
        ]
    );
}

#[test]
fn atoms_and_fields_are_not_modules() {
    let names = modules("x = :Atom\ny = conn.Assigns\nz = Real.Module\nw = %{Key: 1}\n");
    assert_eq!(names, ["Real.Module"]);
}

#[test]
fn unterminated_literals_do_not_panic() {
    for source in [
        "x = \"never closed",
        "~",
        "~s",
        "~s(",
        "?",
        "\"\"\"\nopen",
        "\"#{",
        "'",
    ] {
        parse(source);
    }
    assert_eq!(parse(""), ElixirFile::default());
}

#[test]
fn an_operator_that_looks_like_a_sigil_is_not_one() {
    let refs = parse("x = a ~> Billing.Next.step()\ny = ~~~Billing.Bits.mask()").refs;
    assert!(refs.contains(&call("Billing.Next", "step")), "{refs:?}");
    assert!(refs.contains(&call("Billing.Bits", "mask")), "{refs:?}");
}

#[test]
fn naming_a_module_in_an_alias_is_not_using_it() {
    // The base of a group and an alias nobody uses are no references.
    let names = modules(
        "alias Billing.Ledger.{Entry, Account}\nalias Billing.Unused\nalias Billing.Tax, as: T\nEntry.new()\n",
    );
    assert_eq!(names, ["Billing.Ledger.Entry"]);

    // What follows the statement on later lines is still read.
    let names = modules("alias Billing.{\n  Tax,\n  Fees\n}\n\nTax.rate() + Billing.Other.x()\n");
    assert_eq!(names, ["Billing.Other", "Billing.Tax"]);
}
