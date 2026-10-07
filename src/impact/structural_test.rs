use super::*;
use crate::git::ChangeKind;
use crate::impact::protection::Protection;
use crate::impact::structural_json::JsonStructural;
use crate::impact::structural_report::{reach_line, render, summary};

const INSIGHTS: &str = "\
defmodule Booking.Insights do
  alias Booking.Repo

  defstruct [:id]

  def arrange(insight, user) do
    record(insight, user, :arranged)
  end

  def list, do: Repo.all(__MODULE__)

  defp record(insight, _user, state) do
    Repo.insert!(%{insight | state: state})
  end
end
";

/// The 1-based number of the line of `INSIGHTS` holding `needle`.
fn line_of(needle: &str) -> usize {
    INSIGHTS.lines().position(|l| l.contains(needle)).unwrap() + 1
}

fn project(extra: &[(&str, &str)]) -> Vec<(PathBuf, String)> {
    let base = [
        ("lib/booking/insights.ex", INSIGHTS),
        (
            "lib/booking/repo.ex",
            "defmodule Booking.Repo do\n  def all(q), do: q\n  def insert!(x), do: x\nend\n",
        ),
        (
            "lib/booking_web/controllers/insight_controller.ex",
            "defmodule BookingWeb.InsightController do\n  alias Booking.Insights\n  def create(conn, p), do: Insights.arrange(p, conn.assigns.user)\nend\n",
        ),
        (
            "lib/booking/report.ex",
            "defmodule Booking.Report do\n  def run, do: Booking.Insights.list()\nend\n",
        ),
        (
            "lib/booking/export.ex",
            "defmodule Booking.Export do\n  def header(%Booking.Insights{} = i), do: i.id\nend\n",
        ),
        (
            "lib/booking/mailer.ex",
            "defmodule Booking.Mailer do\n  def send, do: Booking.Report.run()\nend\n",
        ),
        (
            "test/booking/report_test.exs",
            "defmodule Booking.ReportTest do\n  test \"run\" do\n    assert Booking.Report.run()\n  end\nend\n",
        ),
        (
            "test/support/factory.ex",
            "defmodule Booking.Factory do\n  def build, do: {Booking.Export.header(nil), BookingWeb.InsightController}\nend\n",
        ),
        (
            "config/config.exs",
            "import Config\nconfig :booking, Booking.Export, []\n",
        ),
        ("mix.exs", "defmodule Booking.MixProject do\nend\n"),
    ];
    base.iter()
        .chain(extra)
        .map(|(path, text)| (PathBuf::from(path), text.to_string()))
        .collect()
}

fn change(path: &str, lines: &[usize]) -> FileDiffStat {
    FileDiffStat {
        path: PathBuf::from(path),
        old_path: Some(PathBuf::from(path)),
        kind: ChangeKind::Modified,
        added: lines.len(),
        deleted: 0,
        lines: lines.to_vec(),
        old_lines: lines.to_vec(),
        probe: Vec::new(),
    }
}

fn radius(sources: Vec<(PathBuf, String)>, changes: &[FileDiffStat]) -> Structural {
    let changed: Vec<Changed> = changes.iter().collect();
    compute(sources, &changed, &|_, _| false)
}

/// The direct dependents as `(file name, exposure, tests, calls)`.
fn direct(radius: &Structural) -> Vec<(&str, Exposure, usize, Vec<&str>)> {
    radius
        .direct
        .iter()
        .map(|d| {
            (
                d.file.file_name().unwrap().to_str().unwrap(),
                d.exposure,
                d.tests,
                d.calls.iter().map(String::as_str).collect(),
            )
        })
        .collect()
}

const CONTROLLER_TEST: (&str, &str) = (
    "test/booking_web/controllers/insight_controller_test.exs",
    "defmodule BookingWeb.InsightControllerTest do\n  test \"create\", %{conn: conn} do\n    assert post(conn, \"/insights\")\n  end\nend\n",
);

#[test]
fn a_change_inside_a_function_exposes_those_who_call_it() {
    let changes = [change(
        "lib/booking/insights.ex",
        &[line_of("record(insight, user, :arranged)")],
    )];
    let radius = radius(project(&[CONTROLLER_TEST]), &changes);

    assert_eq!(radius.origins, [PathBuf::from("lib/booking/insights.ex")]);
    assert_eq!(
        radius.functions.as_deref(),
        Some(&["arrange".to_string()][..])
    );
    assert_eq!(
        direct(&radius),
        [
            // The controller test names no module: it is at the same place.
            (
                "insight_controller.ex",
                Exposure::Calls,
                1,
                vec!["Insights.arrange"]
            ),
            ("export.ex", Exposure::Refers, 0, vec![]),
            ("report.ex", Exposure::Elsewhere, 1, vec![]),
        ]
    );
    assert_eq!(radius.unprotected().count(), 0);
    assert_eq!(radius.exposed().count(), 2);
    // Sources only: tests, support, configuration and mix.exs are not.
    assert_eq!(radius.source_files, 6);
    // The radius goes through the file that calls what changed. The export
    // only names the struct, and the report calls elsewhere, so neither
    // carries it on: the mailer that uses the report is not reached.
    assert_eq!(radius.by_distance, [1]);
    assert_eq!(radius.reached(), 1);
    // Whatever the function: three direct, then the mailer.
    assert_eq!(radius.upper_bound, 4);
}

#[test]
fn a_change_to_a_private_function_exposes_the_callers_of_what_reaches_it() {
    let changes = [change(
        "lib/booking/insights.ex",
        &[line_of("Repo.insert!")],
    )];
    let radius = radius(project(&[]), &changes);

    assert_eq!(
        radius.functions.as_deref(),
        Some(&["arrange".to_string()][..])
    );
    // Without its test the controller is unprotected.
    assert_eq!(
        direct(&radius)[0],
        (
            "insight_controller.ex",
            Exposure::Calls,
            0,
            vec!["Insights.arrange"]
        )
    );
    let unprotected: Vec<&Path> = radius.unprotected().map(|d| d.file.as_path()).collect();
    assert_eq!(
        unprotected,
        [Path::new(
            "lib/booking_web/controllers/insight_controller.ex"
        )]
    );
}

#[test]
fn a_change_outside_functions_exposes_every_user_of_the_file() {
    let changes = [change("lib/booking/insights.ex", &[line_of("defstruct")])];
    let radius = radius(project(&[]), &changes);

    assert_eq!(radius.functions, None);
    assert_eq!(
        direct(&radius),
        [
            ("export.ex", Exposure::Calls, 0, vec![]),
            (
                "insight_controller.ex",
                Exposure::Calls,
                0,
                vec!["Insights.arrange"]
            ),
            ("report.ex", Exposure::Calls, 1, vec!["Insights.list"]),
        ]
    );
    assert_eq!(radius.unprotected().count(), 2);
}

#[test]
fn tests_of_the_change_are_told_from_the_others() {
    let changes = [
        change("lib/booking/insights.ex", &[line_of("def list")]),
        change("test/booking/report_test.exs", &[1]),
    ];
    let radius = radius(project(&[]), &changes);

    // The changed test is no origin; it protects the report, in the change.
    assert_eq!(radius.origins, [PathBuf::from("lib/booking/insights.ex")]);
    let report = radius
        .direct
        .iter()
        .find(|d| d.file.ends_with("report.ex"))
        .unwrap();
    assert_eq!(
        (report.exposure, report.tests, report.tests_in_diff),
        (Exposure::Calls, 1, 1)
    );
}

#[test]
fn test_support_is_neither_a_test_nor_a_dependent() {
    let changes = [change("lib/booking/export.ex", &[2])];
    let radius = radius(project(&[]), &changes);

    // The factory uses the export; it does not protect it, nor is it reached.
    assert!(radius.direct.is_empty(), "{:?}", radius.direct);
    assert_eq!(radius.reached(), 0);
}

#[test]
fn nothing_to_measure_without_a_changed_source() {
    let changes = [
        change("README.md", &[1]),
        change("src/main.rs", &[1]),
        change("src/lib.rs", &[2]),
    ];
    let radius = radius(project(&[]), &changes);

    assert!(radius.origins.is_empty());
    assert_eq!(radius.functions, None);
    assert_eq!(radius.unavailable, [("Rust".to_string(), 2)]);
    let lines = render(&radius, 20);
    assert_eq!(
        lines[2],
        " No changed source file in a language with a reliable graph"
    );
    assert_eq!(
        lines.last().unwrap(),
        "Not measured for Rust (2 changed): its graph does not reflect usage yet."
    );
}

#[test]
fn render_when_the_change_is_narrowed_to_functions() {
    let changes = [change(
        "lib/booking/insights.ex",
        &[line_of("Repo.insert!")],
    )];
    let lines = render(&radius(project(&[]), &changes), 20);
    let sep = "─".repeat(78);

    assert_eq!(
        lines,
        [
            "Structural radius — source files that use what changed",
            sep.as_str(),
            " 1 file calls what changed, 1 of them with no test",
            " Changed: lib/booking/insights.ex",
            " Functions: arrange",
            " Radius: 1 of 6 source files (17%): 1 at distance 1",
            " Not in the radius: 1 that refer to the module without calling it",
            " Upper bound, whatever the function: 4 files (67%)",
            "",
            " Tests  Dependent",
            "  none  lib/booking_web/controllers/insight_controller.ex",
            "            calls Insights.arrange",
            "  none  lib/booking/export.ex",
            "            refers to the module without calling it",
            sep.as_str(),
            "No test reaches 1 of the files that call what changed; an integration test is probably missing:",
            "  lib/booking_web/controllers/insight_controller.ex",
            "No test in the change exercises a file that uses what changed.",
            "1 more with no test use the module without a call that tells whether the change concerns them.",
        ]
    );
}

#[test]
fn render_when_the_change_cannot_be_narrowed() {
    let changes = [change("lib/booking/insights.ex", &[line_of("defstruct")])];
    let lines = render(&radius(project(&[CONTROLLER_TEST]), &changes), 2);

    assert_eq!(
        lines[2..6],
        [
            " 4 of 6 source files reached (67%), 3 direct, 1 of them with no test",
            " Changed: lib/booking/insights.ex",
            " Every use counts: lib/booking/insights.ex (line 4 is outside every function)",
            " Reach: 3 at distance 1, 1 at distance 2",
        ]
    );
    // Unprotected first, and no more than `top`.
    assert_eq!(lines[8], "  none  lib/booking/export.ex");
    assert!(
        lines.contains(&" 3 files (2 shown).".to_string()),
        "{lines:?}"
    );
}

#[test]
fn render_when_nobody_calls_what_changed() {
    let sources = project(&[]);
    let changes = [change("lib/booking/mailer.ex", &[2])];
    let lines = render(&radius(sources, &changes), 20);
    assert_eq!(lines[2], " 0 of 6 source files reached.");
    assert_eq!(lines[4], " No source file measured uses what changed.");

    // The repo is used, but not through the function that changed.
    let unused = (
        "lib/booking/repo.ex",
        "defmodule Booking.Repo do\n  def all(q), do: q\n  def insert!(x), do: x\n  def never(x), do: x\nend\n",
    );
    let mut sources = project(&[]);
    sources.retain(|(path, _)| !path.ends_with("repo.ex"));
    sources.push((PathBuf::from(unused.0), unused.1.to_string()));
    let lines = render(&radius(sources, &[change("lib/booking/repo.ex", &[4])]), 20);
    assert_eq!(
        lines[2],
        " 0 files call what changed, 0 of them with no test"
    );
    assert_eq!(lines[4], " Functions: never");
    assert_eq!(lines[7], " No other file calls the functions that changed.");
}

#[test]
fn a_long_reach_is_summed_beyond_the_third_distance() {
    let radius = Structural {
        by_distance: vec![5, 4, 3, 2, 1],
        ..Structural::default()
    };
    assert_eq!(
        reach_line(&radius),
        "5 at distance 1, 4 at distance 2, 3 at distance 3, 3 further (up to distance 5)"
    );
}

#[test]
fn what_each_file_is_to_the_radius() {
    assert_eq!(role(Path::new("lib/app/user.ex")), Role::Source);
    assert_eq!(role(Path::new("lib/mix/tasks/seed.ex")), Role::Source);
    assert_eq!(role(Path::new("test/app/user_test.exs")), Role::Test);
    assert_eq!(role(Path::new("src/ui/button.test.ts")), Role::Test);
    assert_eq!(role(Path::new("test/support/factory.ex")), Role::Other);
    assert_eq!(role(Path::new("test/test_helper.exs")), Role::Other);
    assert_eq!(role(Path::new("config/runtime.exs")), Role::Other);
    assert_eq!(role(Path::new("mix.exs")), Role::Other);

    assert!(is_reliable(Path::new("lib/a.ex")));
    assert!(is_reliable(Path::new("src/a.ts")));
    assert!(!is_reliable(Path::new("src/a.rs")));
    assert!(!is_reliable(Path::new("README.md")));
}

#[test]
fn json_block() {
    let changes = [change(
        "lib/booking/insights.ex",
        &[line_of("Repo.insert!")],
    )];
    let radius = radius(project(&[]), &changes);
    let json = serde_json::to_value(JsonStructural::from(&radius)).unwrap();

    assert_eq!(json["source_files"], 6);
    assert_eq!(
        json["origins"],
        serde_json::json!(["lib/booking/insights.ex"])
    );
    assert_eq!(json["functions"], serde_json::json!(["arrange"]));
    assert_eq!(json["reached"], 1);
    assert_eq!(json["by_distance"], serde_json::json!([1]));
    assert_eq!(json["upper_bound"], 4);
    assert_eq!(
        json["radius"],
        serde_json::json!({"files": 1, "source_files": 6, "share": 0.1667})
    );
    assert_eq!(
        json["reach"],
        serde_json::json!([{
            "distance": 1,
            "files": ["lib/booking_web/controllers/insight_controller.ex"]
        }])
    );
    assert_eq!(
        json["unknown_without_tests"],
        serde_json::json!(["lib/booking/export.ex"])
    );
    assert_eq!(json["change_tests_a_dependent"], false);
    assert_eq!(
        json["direct"][0],
        serde_json::json!({
            "file": "lib/booking_web/controllers/insight_controller.ex",
            "exposure": "calls",
            "protection": "none",
            "calls": ["Insights.arrange"],
            "tests": 0,
            "tests_in_diff": 0,
            "entry_point": false
        })
    );
    assert_eq!(json["direct"][2]["exposure"], "elsewhere");
    assert_eq!(
        json["unprotected"],
        serde_json::json!(["lib/booking_web/controllers/insight_controller.ex"])
    );

    let unnarrowed = Structural::default();
    let json = serde_json::to_value(JsonStructural::from(&unnarrowed)).unwrap();
    assert_eq!(json["functions"], serde_json::Value::Null);
}

#[test]
fn the_radius_carries_on_from_the_files_the_change_concerns() {
    // The controller calls what changed, and a router uses the controller.
    let router = (
        "lib/booking_web/router.ex",
        "defmodule BookingWeb.Router do\n  def routes, do: [BookingWeb.InsightController]\nend\n",
    );
    let changes = [change(
        "lib/booking/insights.ex",
        &[line_of("Repo.insert!")],
    )];
    let radius = radius(project(&[router]), &changes);

    // The controller at distance 1, the router at 2. The export, which
    // only names the struct, and the mailer, behind a file that calls
    // elsewhere, are only in the upper bound.
    assert_eq!(radius.by_distance, [1, 1]);
    assert_eq!(radius.upper_bound, 5);
    let lines = render(&radius, 20);
    assert_eq!(
        lines[5],
        " Radius: 2 of 7 source files (29%): 1 at distance 1, 1 at distance 2"
    );
    assert_eq!(
        lines[6],
        " Not in the radius: 1 that refer to the module without calling it"
    );
    assert_eq!(
        lines[7],
        " Upper bound, whatever the function: 5 files (71%)"
    );
}

#[test]
fn without_functions_the_radius_is_the_upper_bound() {
    let changes = [change("lib/booking/insights.ex", &[line_of("defstruct")])];
    let radius = radius(project(&[]), &changes);
    assert_eq!(radius.by_distance, [3, 1]);
    assert_eq!(radius.upper_bound, radius.reached());
}

#[test]
fn a_share_below_one_percent_is_not_shown_as_zero() {
    let radius = Structural {
        origins: vec![PathBuf::from("lib/a.ex")],
        functions: Some(vec!["f".to_string()]),
        source_files: 618,
        direct: vec![Dependent {
            file: PathBuf::from("lib/b.ex"),
            exposure: Exposure::Calls,
            protection: Protection::Direct,
            calls: vec!["A.f".to_string()],
            tests: 1,
            tests_in_diff: 0,
            entry_point: false,
        }],
        by_distance: vec![1, 1],
        upper_bound: 465,
        ..Structural::default()
    };
    let lines = summary(&radius);
    assert_eq!(
        lines[3],
        " Radius: 2 of 618 source files (<1%): 1 at distance 1, 1 at distance 2"
    );
    assert_eq!(
        lines[4],
        " Upper bound, whatever the function: 465 files (75%)"
    );
}

#[test]
fn each_changed_file_is_narrowed_on_its_own() {
    // One file changes inside a function; another is new, and a third
    // changes its struct. Only the first can be narrowed.
    let report_user = (
        "lib/booking/digest.ex",
        "defmodule Booking.Digest do\n  def run, do: Booking.Report.run()\nend\n",
    );
    let mut added = change("lib/booking/mailer.ex", &[1, 2, 3]);
    added.kind = ChangeKind::Added;
    let changes = [
        change(
            "lib/booking/insights.ex",
            &[line_of("record(insight, user, :arranged)")],
        ),
        added,
        change("lib/booking/report.ex", &[1]),
    ];
    let radius = radius(project(&[report_user]), &changes);

    assert_eq!(
        radius.functions.as_deref(),
        Some(&["arrange".to_string()][..])
    );
    let narrowing: Vec<(&str, Option<Vec<&str>>, Option<&str>)> = radius
        .narrowing
        .iter()
        .map(|n| {
            (
                n.file.file_name().unwrap().to_str().unwrap(),
                n.functions
                    .as_ref()
                    .map(|f| f.iter().map(String::as_str).collect()),
                n.reason.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        narrowing,
        [
            ("insights.ex", Some(vec!["arrange"]), None),
            ("mailer.ex", None, Some("new file")),
            ("report.ex", None, Some("line 1 is outside every function")),
        ]
    );

    // The insights change still reaches only who calls arrange; the report
    // could not be narrowed, so its user counts whatever it calls.
    let direct = direct(&radius);
    assert!(direct.contains(&(
        "insight_controller.ex",
        Exposure::Calls,
        0,
        vec!["Insights.arrange"]
    )));
    assert!(direct.contains(&("digest.ex", Exposure::Calls, 0, vec!["Report.run"])));

    let lines = render(&radius, 20);
    assert_eq!(lines[4], " Functions: arrange");
    assert_eq!(
        lines[5],
        " Every use counts for: lib/booking/mailer.ex (new file), lib/booking/report.ex (line 1 is outside every function)"
    );
    assert!(lines[6].starts_with(" Radius: "), "{}", lines[6]);

    let json = serde_json::to_value(JsonStructural::from(&radius)).unwrap();
    assert_eq!(
        json["narrowing"][0],
        serde_json::json!({"file": "lib/booking/insights.ex", "functions": ["arrange"], "reason": null})
    );
    assert_eq!(json["narrowing"][1]["reason"], "new file");
}

#[test]
fn a_language_without_functions_says_so() {
    let sources = vec![
        (
            PathBuf::from("src/app.ts"),
            "import { helper } from './util';\nhelper();\n".to_string(),
        ),
        (
            PathBuf::from("src/util.ts"),
            "export const helper = () => 1;\n".to_string(),
        ),
    ];
    let radius = radius(sources, &[change("src/util.ts", &[1])]);

    assert_eq!(radius.functions, None);
    assert_eq!(
        radius.narrowing[0].reason.as_deref(),
        Some("its language does not tell functions")
    );
    assert_eq!(direct(&radius), [("app.ts", Exposure::Calls, 0, vec![])]);
}

/// A context whose function changes, called by a live view, a helper the
/// view uses, a task nobody tests, and a view tested through a namesake.
fn views() -> Vec<(PathBuf, String)> {
    let files = [
        (
            "lib/shop/orders.ex",
            "defmodule Shop.Orders do\n  def list, do: []\nend\n",
        ),
        (
            "lib/shop_web/live/orders_live/index.ex",
            "defmodule ShopWeb.OrdersLive.Index do\n  def mount, do: {Shop.Orders.list(), ShopWeb.OrdersLive.Rows.build()}\nend\n",
        ),
        (
            "lib/shop_web/live/orders_live/rows.ex",
            "defmodule ShopWeb.OrdersLive.Rows do\n  def build, do: Shop.Orders.list()\nend\n",
        ),
        (
            "lib/shop_web/live/summary_live.ex",
            "defmodule ShopWeb.SummaryLive do\n  def mount, do: ShopWeb.Totals.sum()\nend\n",
        ),
        (
            "lib/shop_web/totals.ex",
            "defmodule ShopWeb.Totals do\n  def sum, do: Shop.Orders.list()\nend\n",
        ),
        (
            "lib/mix/tasks/orders.export.ex",
            "defmodule Mix.Tasks.Orders.Export do\n  def run(_), do: Shop.Orders.list()\nend\n",
        ),
        // Both views are tested through their routes: no module is named.
        (
            "test/shop_web/live/orders_live_test.exs",
            "defmodule ShopWeb.OrdersLiveTest do\n  test \"lists\", %{conn: conn} do\n    live(conn, \"/orders\")\n  end\nend\n",
        ),
        (
            "test/shop_web/summary_live_test.exs",
            "defmodule ShopWeb.SummaryLiveTest do\n  test \"sums\", %{conn: conn} do\n    live(conn, \"/summary\")\n  end\nend\n",
        ),
    ];
    files
        .iter()
        .map(|(path, text)| (PathBuf::from(path), text.to_string()))
        .collect()
}

#[test]
fn protection_comes_in_degrees() {
    let radius = radius(views(), &[change("lib/shop/orders.ex", &[2])]);

    let protection: Vec<(&str, Protection, usize)> = radius
        .direct
        .iter()
        .map(|d| {
            (
                d.file.file_name().unwrap().to_str().unwrap(),
                d.protection,
                d.tests,
            )
        })
        .collect();
    assert_eq!(
        protection,
        [
            // Least protected first.
            ("orders.export.ex", Protection::None, 0),
            ("totals.ex", Protection::Users, 0),
            // Named after the directory the view is in.
            ("index.ex", Protection::Named, 0),
            ("rows.ex", Protection::Named, 0),
        ]
    );
    // Only the task is reported: the rest is reached by some test.
    let unprotected: Vec<&Path> = radius.unprotected().map(|d| d.file.as_path()).collect();
    assert_eq!(unprotected, [Path::new("lib/mix/tasks/orders.export.ex")]);
}

#[test]
fn render_tells_the_degrees_apart() {
    let lines = render(&radius(views(), &[change("lib/shop/orders.ex", &[2])]), 20);
    let table: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .skip_while(|l| *l != " Tests  Dependent")
        .take_while(|l| !l.starts_with('─'))
        .collect();
    assert_eq!(
        table,
        [
            " Tests  Dependent",
            "  none  lib/mix/tasks/orders.export.ex",
            "            calls Orders.list",
            " users  lib/shop_web/totals.ex",
            "            calls Orders.list",
            " named  lib/shop_web/live/orders_live/index.ex",
            "            calls Orders.list",
            " named  lib/shop_web/live/orders_live/rows.ex",
            "            calls Orders.list",
            " named: a test carries its name; users: only what uses it is tested",
        ]
    );
    assert!(lines.contains(
        &"No test reaches 1 of the files that call what changed; an integration test is probably missing:".to_string()
    ));

    // With direct tests only, there is nothing to explain.
    let direct_only = render(
        &radius(
            project(&[CONTROLLER_TEST]),
            &[change(
                "lib/booking/insights.ex",
                &[line_of("record(insight, user, :arranged)")],
            )],
        ),
        20,
    );
    assert!(
        !direct_only.iter().any(|l| l.contains("named: a test")),
        "{direct_only:?}"
    );
}

/// A project whose web layer uses what it does by convention: a view its
/// controller never names, a component only a template renders, a task.
fn web() -> Vec<(PathBuf, String)> {
    let files = [
        (
            "lib/shop/orders.ex",
            "defmodule Shop.Orders do\n  def list, do: []\nend\n",
        ),
        (
            "lib/shop_web/controllers/order_controller.ex",
            "defmodule ShopWeb.OrderController do\n  def index(conn, _), do: conn\nend\n",
        ),
        (
            "lib/shop_web/controllers/order_json.ex",
            "defmodule ShopWeb.OrderJSON do\n  def index(_), do: Shop.Orders.list()\nend\n",
        ),
        (
            "test/shop_web/controllers/order_controller_test.exs",
            "defmodule ShopWeb.OrderControllerTest do\n  test \"lists\", %{conn: conn} do\n    get(conn, \"/orders\")\n  end\nend\n",
        ),
        (
            "lib/shop_web/components/badge.ex",
            "defmodule ShopWeb.Components.Badge do\n  def count(_), do: Shop.Orders.list()\nend\n",
        ),
        (
            "lib/shop_web/live/summary_live.ex",
            "defmodule ShopWeb.SummaryLive do\n  alias ShopWeb.Components.Badge\nend\n",
        ),
        (
            "lib/shop_web/live/summary_live.html.heex",
            "<h1>Summary</h1>\n<Badge.count orders={@orders} />\n",
        ),
        (
            "test/shop_web/live/summary_live_test.exs",
            "defmodule ShopWeb.SummaryLiveTest do\n  test \"sums\", %{conn: conn} do\n    live(conn, \"/summary\")\n  end\nend\n",
        ),
        (
            "lib/mix/tasks/orders.export.ex",
            "defmodule Mix.Tasks.Orders.Export do\n  def run(_), do: Shop.Orders.list()\nend\n",
        ),
    ];
    files
        .iter()
        .map(|(path, text)| (PathBuf::from(path), text.to_string()))
        .collect()
}

fn protection_of(radius: &Structural, name: &str) -> Protection {
    let dependent = radius.direct.iter().find(|d| d.file.ends_with(name));
    dependent
        .unwrap_or_else(|| panic!("{name} is no dependent"))
        .protection
}

#[test]
fn the_test_of_a_controller_protects_the_view_it_renders() {
    let radius = radius(web(), &[change("lib/shop/orders.ex", &[2])]);
    assert_eq!(protection_of(&radius, "order_json.ex"), Protection::Users);
}

#[test]
fn a_view_whose_controller_has_no_test_stays_unprotected() {
    let untested: Vec<_> = web()
        .into_iter()
        .filter(|(path, _)| !path.ends_with("order_controller_test.exs"))
        .collect();
    let radius = radius(untested, &[change("lib/shop/orders.ex", &[2])]);
    assert_eq!(protection_of(&radius, "order_json.ex"), Protection::None);
}

#[test]
fn a_component_is_used_by_the_module_whose_template_renders_it() {
    let radius = radius(web(), &[change("lib/shop/orders.ex", &[2])]);
    assert_eq!(protection_of(&radius, "badge.ex"), Protection::Users);
    // The template is no file of the graph.
    assert_eq!(radius.source_files, 6);

    let without: Vec<_> = web()
        .into_iter()
        .filter(|(path, _)| !path.ends_with("summary_live.html.heex"))
        .collect();
    let radius = radius_of(without);
    assert_eq!(protection_of(&radius, "badge.ex"), Protection::None);
}

fn radius_of(sources: Vec<(PathBuf, String)>) -> Structural {
    radius(sources, &[change("lib/shop/orders.ex", &[2])])
}

#[test]
fn an_entry_point_with_no_test_is_told_apart() {
    let mut radius = radius_of(web());
    let unprotected =
        |r: &Structural| -> Vec<PathBuf> { r.unprotected().map(|d| d.file.clone()).collect() };
    let task = PathBuf::from("lib/mix/tasks/orders.export.ex");
    assert_eq!(unprotected(&radius), [task.clone()]);

    radius.mark_entry_points(|path| path.starts_with("lib/mix/tasks"));
    assert!(unprotected(&radius).is_empty());
    let entry: Vec<&PathBuf> = radius.untested_entry_points().map(|d| &d.file).collect();
    assert_eq!(entry, [&task]);

    let text = render(&radius, 10).join("\n");
    assert!(
        text.contains("Run rather than used, and with no test, 1: lib/mix/tasks/orders.export.ex"),
        "{text}"
    );
    assert!(
        !text.contains("an integration test is probably missing"),
        "{text}"
    );
    let json = serde_json::to_value(JsonStructural::from(&radius)).unwrap();
    assert_eq!(
        json["untested_entry_points"][0],
        "lib/mix/tasks/orders.export.ex"
    );
    assert_eq!(json["unprotected"].as_array().unwrap().len(), 0);
}
