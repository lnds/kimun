use super::*;
use crate::deps::layout::is_manifest;

fn source(path: &str, text: &str) -> Source {
    let language = match path.rsplit('.').next() {
        Some("ex") => "Elixir",
        Some("exs") => "Elixir Script",
        Some("ts") => "TypeScript",
        Some("py") => "Python",
        _ => "Rust",
    };
    Source {
        path: PathBuf::from(path),
        language: language.to_string(),
        text: text.to_string(),
        templates: Vec::new(),
    }
}

fn build(sources: &[Source]) -> FileGraph {
    build_with(sources, &|_, _| false)
}

fn build_with(sources: &[Source], related: Related) -> FileGraph {
    let layout = Layout {
        known: sources.iter().map(|s| s.path.clone()).collect(),
        ..Layout::default()
    };
    FileGraph::build(sources, &layout, related)
}

/// What `file` uses, as the paths used and the calls made on each.
fn uses<'a>(graph: &'a FileGraph, file: &str) -> Vec<(&'a str, Vec<&'a str>)> {
    let index = graph
        .files
        .iter()
        .position(|f| f == Path::new(file))
        .unwrap();
    graph.uses[index]
        .iter()
        .map(|u| {
            (
                graph.files[u.to].to_str().unwrap(),
                u.calls.iter().map(String::as_str).collect(),
            )
        })
        .collect()
}

#[test]
fn elixir_references_resolve_to_the_files_that_define_the_modules() {
    let graph = build(&[
        source(
            "lib/booking/insights.ex",
            "defmodule Booking.Insights do\n  def total(i), do: Booking.Tip.apply(i) + Booking.Tip.flat()\nend\n",
        ),
        source(
            "lib/booking/tip.ex",
            "defmodule Booking.Tip do\n  def apply(i), do: i\n  def flat, do: Enum.count([])\nend\n",
        ),
        source(
            "lib/web/page.ex",
            "defmodule Web.Page do\n  alias Booking.Insights\n  def show(%Booking.Tip{} = t), do: {t, Insights.total(1)}\nend\n",
        ),
    ]);

    assert_eq!(
        uses(&graph, "lib/booking/insights.ex"),
        [(
            "lib/booking/tip.ex",
            vec!["Booking.Tip.apply", "Booking.Tip.flat"]
        )]
    );
    // A module of a library, and a file's own module, are no edges.
    assert!(uses(&graph, "lib/booking/tip.ex").is_empty());
    assert_eq!(
        uses(&graph, "lib/web/page.ex"),
        [
            ("lib/booking/insights.ex", vec!["Booking.Insights.total"]),
            // A struct is a use with no call.
            ("lib/booking/tip.ex", vec![]),
        ]
    );
    assert_eq!(graph.ambiguous, 0);
}

#[test]
fn a_test_script_is_part_of_the_graph() {
    let graph = build(&[
        source("lib/a.ex", "defmodule A do\n  def f, do: 1\nend\n"),
        source(
            "test/a_test.exs",
            "defmodule ATest do\n  use ExUnit.Case\n  test \"f\" do\n    assert A.f() == 1\n  end\nend\n",
        ),
    ]);
    assert_eq!(uses(&graph, "test/a_test.exs"), [("lib/a.ex", vec!["A.f"])]);
}

fn template(project: &str) -> Vec<Source> {
    vec![
        source(
            &format!("{project}/lib/adapter.ex"),
            "defmodule Adapter do\n  def run, do: Adapter.Client.call()\nend\n",
        ),
        source(
            &format!("{project}/lib/adapter/client.ex"),
            "defmodule Adapter.Client do\n  def call, do: :ok\nend\n",
        ),
    ]
}

#[test]
fn a_module_defined_in_several_projects_resolves_within_the_project() {
    let mut sources = template("apps/one");
    sources.extend(template("apps/two"));
    let graph = build(&sources);

    assert_eq!(
        uses(&graph, "apps/one/lib/adapter.ex"),
        [(
            "apps/one/lib/adapter/client.ex",
            vec!["Adapter.Client.call"]
        )]
    );
    assert_eq!(
        uses(&graph, "apps/two/lib/adapter.ex"),
        [(
            "apps/two/lib/adapter/client.ex",
            vec!["Adapter.Client.call"]
        )]
    );
    assert_eq!(graph.ambiguous, 0);
}

#[test]
fn a_module_of_other_projects_resolves_to_the_one_depended_on() {
    let mut sources = template("apps/one");
    sources.extend(template("apps/two"));
    sources.push(source(
        "apps/web/lib/web.ex",
        "defmodule Web do\n  def go, do: Adapter.run()\n  def again, do: Adapter.run()\nend\n",
    ));

    // Nothing tells the two apart: the reference is left out, once.
    let graph = build(&sources);
    assert!(uses(&graph, "apps/web/lib/web.ex").is_empty());
    assert_eq!(graph.ambiguous, 1);

    // The project of `web` declares a dependency on `two`.
    let depends_on_two = |_: &Path, to: &Path| to.starts_with("apps/two");
    let graph = build_with(&sources, &depends_on_two);
    assert_eq!(
        uses(&graph, "apps/web/lib/web.ex"),
        [("apps/two/lib/adapter.ex", vec!["Adapter.run"])]
    );
    assert_eq!(graph.ambiguous, 0);

    // Depending on both does not single one out.
    let graph = build_with(&sources, &|_, _| true);
    assert!(uses(&graph, "apps/web/lib/web.ex").is_empty());
    assert_eq!(graph.ambiguous, 1);
}

#[test]
fn other_languages_go_through_their_import_resolver() {
    let graph = build(&[
        source(
            "src/app.ts",
            "import { helper } from './util';\nhelper();\n",
        ),
        source("src/util.ts", "export const helper = () => 1;\n"),
    ]);
    assert_eq!(uses(&graph, "src/app.ts"), [("src/util.ts", vec![])]);
    assert_eq!(
        graph.edges()[Path::new("src/app.ts")],
        [PathBuf::from("src/util.ts")]
    );
}

#[test]
fn changed_functions_are_told_for_elixir_and_python() {
    let elixir = source(
        "lib/a.ex",
        "defmodule A do\n  def f, do: g()\n\n  defp g, do: 1\nend\n",
    );
    assert_eq!(
        elixir
            .changed_functions(&[4])
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>(),
        ["f"]
    );
    // No line known, or a line that may concern every function.
    assert_eq!(elixir.changed_functions(&[]), Err(Unnarrowed::NoLines));
    assert_eq!(elixir.changed_functions(&[1]), Err(Unnarrowed::Outside(1)));

    let python = source(
        "shop/prices.py",
        "RATE = 2\n\ndef tax(x):\n    return x * RATE\n\nsetup()\n",
    );
    assert_eq!(
        python
            .changed_functions(&[1])
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>(),
        ["RATE", "tax"]
    );
    assert_eq!(python.changed_functions(&[]), Err(Unnarrowed::NoLines));
    assert_eq!(python.changed_functions(&[6]), Err(Unnarrowed::Outside(6)));

    let typescript = source("src/a.ts", "export function f() { return 1 }\n");
    assert_eq!(
        typescript.changed_functions(&[1]),
        Err(Unnarrowed::Language)
    );
}

#[test]
fn what_the_projects_declare_comes_before_what_is_near() {
    // `apps/one` and `libs/adapter` both define Adapter. `apps/web` is
    // nearer to the first by its path, and declares the second.
    let mut sources = template("apps/one");
    sources.extend(template("libs/adapter"));
    sources.push(source(
        "apps/web/lib/web.ex",
        "defmodule Web do\n  def go, do: Adapter.run()\nend\n",
    ));

    let declares_the_lib = |_: &Path, to: &Path| to.starts_with("libs/adapter");
    let graph = build_with(&sources, &declares_the_lib);
    assert_eq!(
        uses(&graph, "apps/web/lib/web.ex"),
        [("libs/adapter/lib/adapter.ex", vec!["Adapter.run"])]
    );

    // With nothing declared, only the path is left to go by.
    let graph = build(&sources);
    assert_eq!(
        uses(&graph, "apps/web/lib/web.ex"),
        [("apps/one/lib/adapter.ex", vec!["Adapter.run"])]
    );
}

#[test]
fn rust_files_use_what_their_paths_name() {
    let sources = [
        source("app/src/lib.rs", "pub mod orders;\npub mod cart;"),
        source("app/src/orders.rs", "pub fn total() {}\n#[test]\nfn t() {}"),
        source("app/src/cart.rs", "use crate::orders::total;"),
        source("app/tests/cart.rs", "use my_app::cart;\n#[test]\nfn t() {}"),
    ];
    let manifest = "[package]\nname = \"my-app\"\nversion = \"0.1.0\"\n";
    let layout = Layout::default().with_manifests([(Path::new("app/Cargo.toml"), manifest)]);
    let graph = FileGraph::build(&sources, &layout, &|_, _| false);

    let used = |file: usize| -> Vec<usize> { graph.uses[file].iter().map(|u| u.to).collect() };
    // Declaring the modules is not using them.
    assert!(used(0).is_empty());
    assert_eq!(used(2), [1]);
    // The integration test reaches the library by the name of its package.
    assert_eq!(used(3), [2]);
    assert_eq!(graph.own_tests, [false, true, false, true]);
    assert!(is_manifest(Path::new("app/Cargo.toml")));
    assert!(!is_manifest(Path::new("app/Cargo.lock")));
}

#[test]
fn python_imports_follow_the_roots_a_project_declares() {
    let sources = [
        source("svc/python/shop/cart.py", "def add(): ...\n"),
        source("svc/python/shop/orders.py", "from shop import cart\n"),
        source("svc/tests/test_cart.py", "from shop.cart import add\n"),
    ];
    let manifest = "[tool.setuptools.packages.find]\nwhere = [\"python\"]\n";
    let layout = Layout {
        known: sources.iter().map(|s| s.path.clone()).collect(),
        ..Layout::default()
    }
    .with_manifests([(Path::new("svc/pyproject.toml"), manifest)]);
    let graph = FileGraph::build(&sources, &layout, &|_, _| false);

    assert_eq!(
        uses(&graph, "svc/tests/test_cart.py"),
        [("svc/python/shop/cart.py", vec!["cart.add"])]
    );
    assert_eq!(
        uses(&graph, "svc/python/shop/orders.py"),
        [("svc/python/shop/cart.py", vec![])]
    );
    assert!(is_manifest(Path::new("svc/pyproject.toml")));
}

#[test]
fn python_uses_tell_the_names_taken_from_each_module() {
    let prices = "RATE = 2\n\ndef tax(x):\n    return x\n\ndef total(x):\n    return x\n";
    let sources = [
        source("shop/__init__.py", "from .prices import total\n"),
        source("shop/prices.py", prices),
        source(
            "shop/named.py",
            "from shop.prices import tax as vat, RATE\n",
        ),
        source(
            "shop/module.py",
            "import shop.prices\n\nX = shop.prices.tax(1)\n",
        ),
        source(
            "shop/alias.py",
            "import shop.prices as p\n\nX = p.total(1)\n",
        ),
        source(
            "shop/sub.py",
            "from shop import prices\n\nX = prices.RATE\n",
        ),
        source("shop/star.py", "from .prices import *\n\nX = tax(RATE)\n"),
        source(
            "shop/value.py",
            "from . import prices\n\nHANDLERS = [prices]\n",
        ),
        source("shop/idle.py", "import shop.prices\n"),
        source("shop/package.py", "from shop import total\n"),
    ];
    let graph = build(&sources);
    let prices = |calls: &[&'static str]| vec![("shop/prices.py", calls.to_vec())];

    assert_eq!(
        uses(&graph, "shop/named.py"),
        prices(&["prices.RATE", "prices.tax"])
    );
    assert_eq!(uses(&graph, "shop/module.py"), prices(&["prices.tax"]));
    assert_eq!(uses(&graph, "shop/alias.py"), prices(&["prices.total"]));
    // A name taken from a package that is one of its modules.
    assert_eq!(uses(&graph, "shop/sub.py"), prices(&["prices.RATE"]));
    // `*` takes the names the file goes on to mention.
    assert_eq!(
        uses(&graph, "shop/star.py"),
        prices(&["prices.RATE", "prices.tax"])
    );
    // A module handed over as a value may give any of its names.
    assert_eq!(
        uses(&graph, "shop/value.py"),
        prices(&["prices.RATE", "prices.tax", "prices.total"])
    );
    // Imported and never read.
    assert_eq!(uses(&graph, "shop/idle.py"), prices(&[]));
    // A name a package re-exports is taken from the package.
    assert_eq!(
        uses(&graph, "shop/package.py"),
        [("shop/__init__.py", vec!["shop.total"])]
    );
    assert_eq!(uses(&graph, "shop/__init__.py"), prices(&["prices.total"]));
}
