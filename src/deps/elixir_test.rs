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
defmodule Booking.Insight do
  def total(insight) do
    Booking.Tip.apply(insight.amount) + Booking.Fare.fixed()
  end
end
"#,
    );

    assert_eq!(file.defines, ["Booking.Insight"]);
    assert_eq!(
        file.refs,
        [
            call("Booking.Fare", "fixed"),
            module("Booking.Insight"),
            call("Booking.Tip", "apply"),
        ]
    );
}

#[test]
fn aliases_are_expanded() {
    let file = parse(
        r#"
defmodule Booking.Report do
  alias Booking.Insight
  alias Booking.Tip, as: T
  alias Booking.Locker.Entry, warn: false

  def run(id) do
    id |> Insight.get!() |> T.apply()
    %Entry{}
  end
end
"#,
    );

    assert!(file.refs.contains(&call("Booking.Insight", "get!")));
    assert!(file.refs.contains(&call("Booking.Tip", "apply")));
    assert!(file.refs.contains(&module("Booking.Locker.Entry")));
    // The short names are not modules of their own.
    assert!(!modules("alias Booking.Insight\nInsight.get(1)").contains(&"Insight".to_string()));
}

#[test]
fn a_grouped_alias_may_span_lines() {
    let source = r#"
defmodule Web.Controller do
  alias Booking.{
    Insight,
    Locker.Entry,
    Tip
  }
  alias Web.{View, Router}

  def show, do: {Insight.get(1), Entry.new(), Tip.rate(), View.render(), Router.path()}
end
"#;
    let refs = parse(source).refs;
    for expected in [
        call("Booking.Insight", "get"),
        call("Booking.Locker.Entry", "new"),
        call("Booking.Tip", "rate"),
        call("Web.View", "render"),
        call("Web.Router", "path"),
    ] {
        assert!(refs.contains(&expected), "missing {expected:?} in {refs:?}");
    }
}

#[test]
fn an_alias_may_build_on_another() {
    let refs = parse("alias Booking.Locker\nalias Locker.Entry\nEntry.new()").refs;
    assert!(
        refs.contains(&call("Booking.Locker.Entry", "new")),
        "{refs:?}"
    );
}

#[test]
fn current_module_stands_for_its_name() {
    let file = parse(
        r#"
defmodule Booking.Insight do
  alias __MODULE__.Line
  alias __MODULE__, as: Self

  def lines(%__MODULE__{} = insight), do: Line.of(insight) ++ Self.extra()
end
"#,
    );
    assert!(file.refs.contains(&call("Booking.Insight.Line", "of")));
    assert!(file.refs.contains(&call("Booking.Insight", "extra")));
}

#[test]
fn nested_modules_take_the_name_of_the_outer_one() {
    let file = parse(
        r#"
defmodule Booking.Insight do
  defmodule Line do
    defstruct [:amount]

    defmodule Tip do
      def of(_), do: __MODULE__
    end

    def new, do: %__MODULE__{}
  end

  defmodule Note do
  end

  def me, do: __MODULE__
end

defprotocol Booking.Printable do
  def print(data)
end
"#,
    );
    assert_eq!(
        file.defines,
        [
            "Booking.Insight",
            "Booking.Insight.Line",
            "Booking.Insight.Line.Tip",
            "Booking.Insight.Note",
            "Booking.Printable",
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

  def render(%Booking.Insight{} = i), do: Enum.map(i.lines, &Booking.Line.format/1)
end

defimpl Booking.Printable, for: Booking.Insight do
  def print(_), do: :ok
end
"#,
    );
    for expected in [
        "Web.Component",
        "Web.Helpers",
        "Logger",
        "Web.Renderable",
        "Booking.Insight",
        "Booking.Line",
        "Booking.Printable",
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
defmodule Booking.Docs do
  @moduledoc """
  Use it like this:

      iex> Booking.Ghost.call()
  """

  # Booking.Commented.out()
  @doc ~S"""
  See `Booking.Sigil.doc/0`.
  """
  def text do
    a = "Booking.InString.x() and #{Booking.Interpolated.y("q")} too"
    b = 'Booking.Charlist.z()'
    c = ~s(Booking.Sigil.paren())
    d = ~r/Booking\.Regex/
    e = ~w[Booking.Words.A Booking.Words.B]
    f = ?B
    g = :"Booking.QuotedAtom"
    {a, b, c, d, e, f, g, Booking.Real.call()}
  end
end
"##,
    );
    assert_eq!(
        file.refs,
        [module("Booking.Docs"), call("Booking.Real", "call")]
    );
}

#[test]
fn a_question_mark_in_a_name_is_not_a_character_literal() {
    let refs = parse(
        "if Booking.Insight.paid?(i), do: Booking.Locker.close!(i)\nx = ?\\n\nBooking.After.run()",
    )
    .refs;
    assert_eq!(
        refs,
        [
            call("Booking.After", "run"),
            call("Booking.Insight", "paid?"),
            call("Booking.Locker", "close!"),
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
    let refs = parse("x = a ~> Booking.Next.step()\ny = ~~~Booking.Bits.mask()").refs;
    assert!(refs.contains(&call("Booking.Next", "step")), "{refs:?}");
    assert!(refs.contains(&call("Booking.Bits", "mask")), "{refs:?}");
}

#[test]
fn naming_a_module_in_an_alias_is_not_using_it() {
    // The base of a group and an alias nobody uses are no references.
    let names = modules(
        "alias Booking.Locker.{Entry, Account}\nalias Booking.Unused\nalias Booking.Tip, as: T\nEntry.new()\n",
    );
    assert_eq!(names, ["Booking.Locker.Entry"]);

    // What follows the statement on later lines is still read.
    let names = modules("alias Booking.{\n  Tip,\n  Fare\n}\n\nTip.rate() + Booking.Other.x()\n");
    assert_eq!(names, ["Booking.Other", "Booking.Tip"]);
}
