use super::*;

const REPORT: &str = "defmodule Booking.Report do
  alias Booking.Insight
  import Booking.Format, only: [money: 1]

  def summary(id) do
    id |> Insight.total() |> render()
  end

  def export(id), do: Booking.Repo.get(id)

  defp render(total) do
    money(total) <> Booking.Style.wrap(total)
  end
end
";

#[test]
fn each_public_function_calls_what_it_and_its_private_helpers_call() {
    let calls = calls(REPORT);
    let summary: Vec<&str> = calls["summary"].iter().map(String::as_str).collect();
    // Through `render`: the imported `money` and `Booking.Style.wrap`.
    assert_eq!(
        summary,
        vec![
            "Booking.Format.money",
            "Booking.Insight.total",
            "Booking.Style.wrap"
        ]
    );
    let export: Vec<&str> = calls["export"].iter().map(String::as_str).collect();
    assert_eq!(export, vec!["Booking.Repo.get"]);
    assert!(!calls.contains_key("render"));
}

#[test]
fn clauses_of_one_function_are_one() {
    let source = "defmodule M do
  def f(0), do: A.zero()
  def f(n), do: B.more(n)
end
";
    let calls = calls(source);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls["f"].len(), 2);
}
