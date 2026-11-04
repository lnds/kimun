use super::*;

/// The public functions a change to `lines` affects, in order.
fn told(source: &str, lines: &[usize]) -> Result<Vec<String>, Unnarrowed> {
    changed_functions(source, lines).map(|n| n.functions.into_iter().collect())
}

const CONTEXT: &str = r#"defmodule Booking.Insights do
  @moduledoc """
  Insights.
  """
  alias Booking.Repo

  @default_rate 19

  @doc """
  Arrange an insight.
  """
  @spec arrange(map(), map()) :: {:ok, map()}
  def arrange(insight, user) do
    insight
    |> check(user)
    |> record(:arranged)
  end

  def revoke(insight, user), do: record(check(insight, user), :revoked)

  def total(insight), do: Enum.sum(insight.lines) * rate()

  def list, do: Repo.all(Booking.Insight)

  defp check(insight, user) do
    if allowed?(user), do: insight, else: raise("no")
  end

  defp allowed?(%{role: :admin}), do: true
  defp allowed?(_), do: false

  defp record(insight, state) do
    Repo.insert!(%{insight | state: state})
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

fn changed(needles: &[&str]) -> Result<Vec<String>, Unnarrowed> {
    let lines: Vec<usize> = needles.iter().map(|n| line_of(n)).collect();
    told(CONTEXT, &lines)
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
        span("arrange"),
        (true, line_of("@doc"), line_of("|> record(:arranged)") + 1)
    );
    assert_eq!(span("revoke").2, line_of("def revoke"));
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
            "arrange", "revoke", "total", "list", "check", "allowed?", "allowed?", "record", "rate"
        ]
    );
}

#[test]
fn a_change_inside_a_public_function_affects_it_alone() {
    assert_eq!(changed(&["|> check(user)"]).unwrap(), ["arrange"]);
    assert_eq!(changed(&["def list"]).unwrap(), ["list"]);
    // Its documentation counts as part of it.
    assert_eq!(changed(&["Arrange an insight."]).unwrap(), ["arrange"]);
    assert_eq!(changed(&["@spec arrange"]).unwrap(), ["arrange"]);
}

#[test]
fn a_change_to_a_private_function_affects_the_public_ones_that_reach_it() {
    // record is called by arrange and revoke.
    assert_eq!(changed(&["Repo.insert!"]).unwrap(), ["arrange", "revoke"]);
    // allowed? is called by check, which arrange and revoke call.
    assert_eq!(changed(&["role: :admin"]).unwrap(), ["arrange", "revoke"]);
    assert_eq!(changed(&["defp rate"]).unwrap(), ["total"]);
}

#[test]
fn a_change_that_may_concern_every_function_cannot_be_narrowed() {
    // The line is told, to say why.
    assert_eq!(
        changed(&["defmodule Booking.Insights"]),
        Err(Unnarrowed::Outside(1))
    );

    let with_use = "defmodule A do\n  use GenServer\n  import Ecto.Query\n  defstruct [:id]\n\n  def f, do: 1\nend\n";
    assert_eq!(told(with_use, &[2]), Err(Unnarrowed::Outside(2)));
    assert_eq!(told(with_use, &[3]), Err(Unnarrowed::Outside(3)));
    assert_eq!(told(with_use, &[4]), Err(Unnarrowed::Outside(4)));
    // Even next to a change inside a function: the first such line wins.
    assert_eq!(told(with_use, &[6, 4, 3]), Err(Unnarrowed::Outside(4)));
}

#[test]
fn naming_what_the_functions_use_changes_no_function() {
    // An alias or a require says nothing by itself: the functions that use
    // the new name are touched too, and those are what counts.
    assert_eq!(
        changed(&["alias Booking.Repo"]).unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(
        changed(&["def list", "alias Booking.Repo"]).unwrap(),
        ["list"]
    );

    let grouped = "defmodule A do\n  alias B.{\n    C,\n    D\n  }\n  require Logger\n\n  def f, do: C.x()\nend\n";
    for line in 2..=6 {
        assert_eq!(
            told(grouped, &[line]).unwrap().len(),
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
    assert_eq!(told(source, &[2, 3]).unwrap().len(), 0);
    // A longer name is another attribute.
    let names = |lines: &[usize]| told(source, lines);
    assert_eq!(names(&[4]).unwrap(), ["wait"]);
    assert_eq!(names(&[5]).unwrap(), ["all"]);
    // One that no function reads may be read by the compiler or a macro.
    assert_eq!(names(&[6]), Err(Unnarrowed::Unread(6, "@unused".into())));
    assert_eq!(
        names(&[7]),
        Err(Unnarrowed::CompileTime(7, "@behaviour".into()))
    );
}

#[test]
fn a_change_to_comments_or_the_module_doc_affects_nothing() {
    assert_eq!(changed(&["Insights."]).unwrap(), Vec::<String>::new());
    assert_eq!(told(CONTEXT, &[]).unwrap().len(), 0);
    // A line past the end of the file is ignored.
    assert_eq!(told(CONTEXT, &[9_999]).unwrap().len(), 0);
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

fn changed_component(needle: &str) -> Result<Vec<String>, Unnarrowed> {
    told(COMPONENTS, &[component_line(needle)])
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
    assert_eq!(
        changed_component("use Phoenix.Component"),
        Err(Unnarrowed::Outside(2))
    );
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

const COUNTRIES: &str = r#"defmodule Geo do
  @valid_countries [
    "CL",
    # Peru joined later.
    "PE"
  ]
  @codes ~w(
    CL
    PE
  )
  @all_codes @codes ++ ["AR"]
  @pattern ~r/^[A-Z]{2}$/

  def valid_countries, do: @valid_countries

  def valid?(code), do: code in valid_countries()

  def known?(code), do: code in @all_codes

  defp shape?(code), do: code =~ @pattern

  def check(code), do: shape?(code) and known?(code)
end
"#;

fn country_line(needle: &str) -> usize {
    COUNTRIES.lines().position(|l| l.contains(needle)).unwrap() + 1
}

#[test]
fn every_line_of_an_attribute_value_changes_its_readers() {
    for needle in ["\"PE\"", "# Peru", "  ]", "@valid_countries ["] {
        assert_eq!(
            told(COUNTRIES, &[country_line(needle)]).unwrap(),
            ["valid?", "valid_countries"],
            "{needle}"
        );
    }
    // Inside a sigil, the words are blank once literals are set aside.
    assert_eq!(
        told(COUNTRIES, &[country_line("    PE")]).unwrap(),
        ["check", "known?"]
    );
}

#[test]
fn an_attribute_that_reads_a_changed_one_changes_too() {
    let narrowed = changed_functions(COUNTRIES, &[country_line("    CL")]).unwrap();
    assert_eq!(
        narrowed.functions.into_iter().collect::<Vec<_>>(),
        ["check", "known?"]
    );
    // What reads each attribute, private functions too.
    let read: Vec<(String, Vec<String>)> = narrowed
        .attributes
        .into_iter()
        .map(|(name, by)| (name, by.into_iter().collect()))
        .collect();
    assert_eq!(
        read,
        [("@all_codes".to_string(), vec!["known?".to_string()])]
    );

    let pattern = changed_functions(COUNTRIES, &[country_line("@pattern")]).unwrap();
    assert_eq!(pattern.functions.into_iter().collect::<Vec<_>>(), ["check"]);
    assert_eq!(pattern.attributes.keys().collect::<Vec<_>>(), ["@pattern"]);
}

#[test]
fn an_attribute_the_compiler_reads_concerns_every_function() {
    let schema = "defmodule A do\n  use Ecto.Schema\n  @derive {Jason.Encoder, only: [\n    :id\n  ]}\n  @primary_key {:id, :binary_id, autogenerate: true}\n\n  def f, do: 1\nend\n";
    let derive = Err(Unnarrowed::CompileTime(3, "@derive".into()));
    assert_eq!(told(schema, &[3]), derive);
    assert_eq!(
        told(schema, &[4]),
        Err(Unnarrowed::CompileTime(4, "@derive".into()))
    );
    assert_eq!(
        told(schema, &[6]),
        Err(Unnarrowed::CompileTime(6, "@primary_key".into()))
    );
}

#[test]
fn the_routes_of_a_router_are_told_apart() {
    let router = "defmodule Web.Router do\n  use Web, :router\n\n  scope \"/api\", Web do\n    pipe_through :api\n    get \"/items\", ItemController, :index\n  end\n\n  def helper, do: :ok\nend\n";
    assert_eq!(told(router, &[6]), Err(Unnarrowed::Routes(6)));
    assert_eq!(told(router, &[5]), Err(Unnarrowed::Routes(5)));
    assert_eq!(told(router, &[9]).unwrap(), ["helper"]);
    // The same line elsewhere is just outside every function.
    let plain = router.replace("use Web, :router", "use Web, :controller");
    assert_eq!(told(&plain, &[6]), Err(Unnarrowed::Outside(6)));
}
