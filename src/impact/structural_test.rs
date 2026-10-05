use super::*;
use crate::git::ChangeKind;

const INVOICES: &str = "\
defmodule Billing.Invoices do
  alias Billing.Repo

  defstruct [:id]

  def approve(invoice, user) do
    record(invoice, user, :approved)
  end

  def list, do: Repo.all(__MODULE__)

  defp record(invoice, _user, state) do
    Repo.insert!(%{invoice | state: state})
  end
end
";

/// The 1-based number of the line of `INVOICES` holding `needle`.
fn line_of(needle: &str) -> usize {
    INVOICES.lines().position(|l| l.contains(needle)).unwrap() + 1
}

fn project(extra: &[(&str, &str)]) -> Vec<(PathBuf, String)> {
    let base = [
        ("lib/billing/invoices.ex", INVOICES),
        (
            "lib/billing/repo.ex",
            "defmodule Billing.Repo do\n  def all(q), do: q\n  def insert!(x), do: x\nend\n",
        ),
        (
            "lib/billing_web/controllers/invoice_controller.ex",
            "defmodule BillingWeb.InvoiceController do\n  alias Billing.Invoices\n  def create(conn, p), do: Invoices.approve(p, conn.assigns.user)\nend\n",
        ),
        (
            "lib/billing/report.ex",
            "defmodule Billing.Report do\n  def run, do: Billing.Invoices.list()\nend\n",
        ),
        (
            "lib/billing/export.ex",
            "defmodule Billing.Export do\n  def header(%Billing.Invoices{} = i), do: i.id\nend\n",
        ),
        (
            "lib/billing/mailer.ex",
            "defmodule Billing.Mailer do\n  def send, do: Billing.Report.run()\nend\n",
        ),
        (
            "test/billing/report_test.exs",
            "defmodule Billing.ReportTest do\n  test \"run\" do\n    assert Billing.Report.run()\n  end\nend\n",
        ),
        (
            "test/support/factory.ex",
            "defmodule Billing.Factory do\n  def build, do: {Billing.Export.header(nil), BillingWeb.InvoiceController}\nend\n",
        ),
        (
            "config/config.exs",
            "import Config\nconfig :billing, Billing.Export, []\n",
        ),
        ("mix.exs", "defmodule Billing.MixProject do\nend\n"),
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
    "test/billing_web/controllers/invoice_controller_test.exs",
    "defmodule BillingWeb.InvoiceControllerTest do\n  test \"create\", %{conn: conn} do\n    assert post(conn, \"/invoices\")\n  end\nend\n",
);

#[test]
fn a_change_inside_a_function_exposes_those_who_call_it() {
    let changes = [change(
        "lib/billing/invoices.ex",
        &[line_of("record(invoice, user, :approved)")],
    )];
    let radius = radius(project(&[CONTROLLER_TEST]), &changes);

    assert_eq!(radius.origins, [PathBuf::from("lib/billing/invoices.ex")]);
    assert_eq!(
        radius.functions.as_deref(),
        Some(&["approve".to_string()][..])
    );
    assert_eq!(
        direct(&radius),
        [
            // The controller test names no module: it is at the same place.
            (
                "invoice_controller.ex",
                Exposure::Calls,
                1,
                vec!["Invoices.approve"]
            ),
            ("export.ex", Exposure::Refers, 0, vec![]),
            ("report.ex", Exposure::Elsewhere, 1, vec![]),
        ]
    );
    assert_eq!(radius.unprotected().count(), 0);
    assert_eq!(radius.exposed().count(), 2);
    // Sources only: tests, support, configuration and mix.exs are not.
    assert_eq!(radius.source_files, 6);
    // The radius goes through the two files the change concerns. The
    // report calls elsewhere, so the mailer that uses it is not reached.
    assert_eq!(radius.by_distance, [2]);
    assert_eq!(radius.reached(), 2);
    // Whatever the function: three direct, then the mailer.
    assert_eq!(radius.upper_bound, 4);
}

#[test]
fn a_change_to_a_private_function_exposes_the_callers_of_what_reaches_it() {
    let changes = [change(
        "lib/billing/invoices.ex",
        &[line_of("Repo.insert!")],
    )];
    let radius = radius(project(&[]), &changes);

    assert_eq!(
        radius.functions.as_deref(),
        Some(&["approve".to_string()][..])
    );
    // Without its test the controller is unprotected.
    assert_eq!(
        direct(&radius)[0],
        (
            "invoice_controller.ex",
            Exposure::Calls,
            0,
            vec!["Invoices.approve"]
        )
    );
    let unprotected: Vec<&Path> = radius.unprotected().map(|d| d.file.as_path()).collect();
    assert_eq!(
        unprotected,
        [Path::new(
            "lib/billing_web/controllers/invoice_controller.ex"
        )]
    );
}

#[test]
fn a_change_outside_functions_exposes_every_user_of_the_file() {
    let changes = [change("lib/billing/invoices.ex", &[line_of("defstruct")])];
    let radius = radius(project(&[]), &changes);

    assert_eq!(radius.functions, None);
    assert_eq!(
        direct(&radius),
        [
            ("export.ex", Exposure::Calls, 0, vec![]),
            (
                "invoice_controller.ex",
                Exposure::Calls,
                0,
                vec!["Invoices.approve"]
            ),
            ("report.ex", Exposure::Calls, 1, vec!["Invoices.list"]),
        ]
    );
    assert_eq!(radius.unprotected().count(), 2);
}

#[test]
fn tests_of_the_change_are_told_from_the_others() {
    let changes = [
        change("lib/billing/invoices.ex", &[line_of("def list")]),
        change("test/billing/report_test.exs", &[1]),
    ];
    let radius = radius(project(&[]), &changes);

    // The changed test is no origin; it protects the report, in the change.
    assert_eq!(radius.origins, [PathBuf::from("lib/billing/invoices.ex")]);
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
    let changes = [change("lib/billing/export.ex", &[2])];
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
        "lib/billing/invoices.ex",
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
            " Changed: lib/billing/invoices.ex",
            " Functions: approve",
            " Radius: 2 of 6 source files (33%): 2 at distance 1",
            " Upper bound, whatever the function: 4 files (67%)",
            "",
            " Tests  Dependent",
            "  none  lib/billing_web/controllers/invoice_controller.ex",
            "            calls Invoices.approve",
            "  none  lib/billing/export.ex",
            "            refers to the module without calling it",
            sep.as_str(),
            "No test refers to 1 of the files that call what changed; an integration test is probably missing:",
            "  lib/billing_web/controllers/invoice_controller.ex",
            "No test in the change exercises a file that uses what changed.",
            "1 more with no test use the module without a call that tells whether the change concerns them.",
        ]
    );
}

#[test]
fn render_when_the_change_cannot_be_narrowed() {
    let changes = [change("lib/billing/invoices.ex", &[line_of("defstruct")])];
    let lines = render(&radius(project(&[CONTROLLER_TEST]), &changes), 2);

    assert_eq!(
        lines[2..6],
        [
            " 4 of 6 source files reached (67%), 3 direct, 1 of them with no test",
            " Changed: lib/billing/invoices.ex",
            " The change touches code outside functions: every use of the file counts.",
            " Reach: 3 at distance 1, 1 at distance 2",
        ]
    );
    // Unprotected first, and no more than `top`.
    assert_eq!(lines[8], "  none  lib/billing/export.ex");
    assert!(
        lines.contains(&" 3 files (2 shown).".to_string()),
        "{lines:?}"
    );
}

#[test]
fn render_when_nobody_calls_what_changed() {
    let sources = project(&[]);
    let changes = [change("lib/billing/mailer.ex", &[2])];
    let lines = render(&radius(sources, &changes), 20);
    assert_eq!(lines[2], " 0 of 6 source files reached.");
    assert_eq!(lines[4], " No source file measured uses what changed.");

    // The repo is used, but not through the function that changed.
    let unused = (
        "lib/billing/repo.ex",
        "defmodule Billing.Repo do\n  def all(q), do: q\n  def insert!(x), do: x\n  def never(x), do: x\nend\n",
    );
    let mut sources = project(&[]);
    sources.retain(|(path, _)| !path.ends_with("repo.ex"));
    sources.push((PathBuf::from(unused.0), unused.1.to_string()));
    let lines = render(&radius(sources, &[change("lib/billing/repo.ex", &[4])]), 20);
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
fn a_file_and_its_test_are_at_the_same_place() {
    let same = |source: &str, test: &str| place(Path::new(source)) == place(Path::new(test));

    assert!(same(
        "lib/app_web/controllers/user_controller.ex",
        "test/app_web/controllers/user_controller_test.exs"
    ));
    assert!(same("apps/a/lib/a/b.ex", "apps/a/test/a/b_test.exs"));
    assert!(same("src/ui/button.ts", "src/ui/button.test.ts"));
    assert!(same("src/ui/button.ts", "src/ui/__tests__/button.spec.ts"));
    assert!(same("pkg/parser.py", "pkg/tests/test_parser.py"));
    assert!(same("src/Parser.hs", "test/ParserSpec.hs"));
    // Another project, another directory or another name is another place.
    assert!(!same("apps/a/lib/a/b.ex", "apps/c/test/a/b_test.exs"));
    assert!(!same(
        "lib/app/users.ex",
        "test/app/accounts/users_test.exs"
    ));
    assert!(!same("lib/app/user.ex", "test/app/users_test.exs"));
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
        "lib/billing/invoices.ex",
        &[line_of("Repo.insert!")],
    )];
    let radius = radius(project(&[]), &changes);
    let json = serde_json::to_value(JsonStructural::from(&radius)).unwrap();

    assert_eq!(json["source_files"], 6);
    assert_eq!(
        json["origins"],
        serde_json::json!(["lib/billing/invoices.ex"])
    );
    assert_eq!(json["functions"], serde_json::json!(["approve"]));
    assert_eq!(json["reached"], 2);
    assert_eq!(json["by_distance"], serde_json::json!([2]));
    assert_eq!(json["upper_bound"], 4);
    assert_eq!(
        json["radius"],
        serde_json::json!({"files": 2, "source_files": 6, "share": 0.3333})
    );
    assert_eq!(
        json["reach"],
        serde_json::json!([{
            "distance": 1,
            "files": [
                "lib/billing_web/controllers/invoice_controller.ex",
                "lib/billing/export.ex"
            ]
        }])
    );
    assert_eq!(
        json["unknown_without_tests"],
        serde_json::json!(["lib/billing/export.ex"])
    );
    assert_eq!(json["change_tests_a_dependent"], false);
    assert_eq!(
        json["direct"][0],
        serde_json::json!({
            "file": "lib/billing_web/controllers/invoice_controller.ex",
            "exposure": "calls",
            "calls": ["Invoices.approve"],
            "tests": 0,
            "tests_in_diff": 0
        })
    );
    assert_eq!(json["direct"][2]["exposure"], "elsewhere");
    assert_eq!(
        json["unprotected"],
        serde_json::json!(["lib/billing_web/controllers/invoice_controller.ex"])
    );

    let unnarrowed = Structural::default();
    let json = serde_json::to_value(JsonStructural::from(&unnarrowed)).unwrap();
    assert_eq!(json["functions"], serde_json::Value::Null);
}

#[test]
fn the_radius_carries_on_from_the_files_the_change_concerns() {
    // The controller calls what changed, and a router uses the controller.
    let router = (
        "lib/billing_web/router.ex",
        "defmodule BillingWeb.Router do\n  def routes, do: [BillingWeb.InvoiceController]\nend\n",
    );
    let changes = [change(
        "lib/billing/invoices.ex",
        &[line_of("Repo.insert!")],
    )];
    let radius = radius(project(&[router]), &changes);

    // Controller and export at distance 1, the router at 2; the mailer,
    // behind a file that calls elsewhere, is only in the upper bound.
    assert_eq!(radius.by_distance, [2, 1]);
    assert_eq!(radius.upper_bound, 5);
    let lines = render(&radius, 20);
    assert_eq!(
        lines[5],
        " Radius: 3 of 7 source files (43%): 2 at distance 1, 1 at distance 2"
    );
    assert_eq!(
        lines[6],
        " Upper bound, whatever the function: 5 files (71%)"
    );
}

#[test]
fn without_functions_the_radius_is_the_upper_bound() {
    let changes = [change("lib/billing/invoices.ex", &[line_of("defstruct")])];
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
            calls: vec!["A.f".to_string()],
            tests: 1,
            tests_in_diff: 0,
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
