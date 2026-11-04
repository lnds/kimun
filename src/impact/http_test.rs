use super::*;

const ROUTER: &str = "defmodule ShopWeb.Router do
  use ShopWeb, :router

  scope \"/api\", ShopWeb do
    get \"/v1/items\", ItemController, :index
    get \"/v1/items/:id\", ItemController, :show
    post \"/v1/orders\", OrderController, :create
  end
end
";

const CONTROLLER: &str = "defmodule ShopWeb.ItemController do
  def index(conn, _), do: conn
  def show(conn, _), do: conn
end
";

fn sources(router: &str) -> Vec<(PathBuf, String)> {
    [
        ("apps/shop/lib/shop_web/router.ex", router),
        ("apps/shop/lib/shop_web/item_controller.ex", CONTROLLER),
        (
            "apps/billing/lib/billing/shop.ex",
            "defmodule Billing.Shop do\n  def items, do: Req.get!(url: \"/api/v1/items\")\n  def item(id), do: Tesla.get(client(), \"/v1/items/#{id}\")\n  def order(b), do: Req.post!(url: \"/api/v1/orders\", json: b)\n  def any(p), do: Req.get!(url: p)\nend\n",
        ),
        ("apps/billing/test/billing/shop_test.exs", "defmodule Billing.ShopTest do\n  test \"x\", do: Req.get!(url: \"/api/v1/items\")\nend\n"),
        ("web/src/items.ts", "export const list = () => fetch('/api/v1/items');\n"),
    ]
    .iter()
    .map(|(p, t)| (PathBuf::from(p), t.to_string()))
    .collect()
}

fn project(path: &Path) -> Option<String> {
    path.components()
        .nth(1)
        .and_then(|c| c.as_os_str().to_str())
        .map(str::to_string)
}

fn router_change(
    removed: &[(usize, &str)],
    inserted: &[(usize, &str)],
    lines: &[usize],
) -> FileDiffStat {
    FileDiffStat {
        path: PathBuf::from("apps/shop/lib/shop_web/router.ex"),
        old_path: None,
        kind: ChangeKind::Modified,
        added: inserted.len(),
        deleted: removed.len(),
        lines: lines.to_vec(),
        old_lines: lines.to_vec(),
        probe: Vec::new(),
        removed: removed.iter().map(|(_, l)| l.to_string()).collect(),
        inserted: inserted.iter().map(|(_, l)| l.to_string()).collect(),
        removed_at: removed.iter().map(|(n, _)| *n).collect(),
        inserted_at: inserted.iter().map(|(n, _)| *n).collect(),
    }
}

fn measured(router: &str, change: FileDiffStat, narrowing: &[Narrowing]) -> Http {
    let sources = sources(router);
    let repository = Repository {
        sources: &sources,
        project_of: &project,
    };
    measure(&repository, &[change], narrowing)
}

#[test]
fn a_removed_route_lists_the_calls_that_still_ask_for_it() {
    let head = ROUTER.replace("    get \"/v1/items\", ItemController, :index\n", "");
    let change = router_change(
        &[(5, "    get \"/v1/items\", ItemController, :index")],
        &[],
        &[5],
    );
    let http = measured(&head, change, &[]);

    assert_eq!(http.routes.len(), 1);
    let route = &http.routes[0];
    assert_eq!(
        (
            route.verb.as_str(),
            route.path.as_str(),
            route.change,
            route.serves.as_str(),
            route.line
        ),
        (
            "get",
            "/api/v1/items",
            RouteChange::Removed,
            "ShopWeb.ItemController.index",
            5
        )
    );
    let callers: Vec<(&str, bool, bool, Option<&str>)> = route
        .callers
        .iter()
        .map(|c| {
            (
                c.file.to_str().unwrap(),
                c.exact,
                c.tested,
                c.project.as_deref(),
            )
        })
        .collect();
    // Tests are no callers; `/v1/items/#{id}` is another route.
    assert_eq!(
        callers,
        [
            (
                "apps/billing/lib/billing/shop.ex",
                true,
                true,
                Some("billing")
            ),
            ("web/src/items.ts", true, false, Some("src")),
        ]
    );
    assert_eq!(http.unresolved.len(), 1);
    assert_eq!(http.unresolved[0].line, 5);
}

#[test]
fn a_changed_action_concerns_the_routes_it_serves() {
    let narrowing = [Narrowing {
        file: PathBuf::from("apps/shop/lib/shop_web/item_controller.ex"),
        functions: Some(vec!["show".to_string()]),
        reason: None,
        attributes: Vec::new(),
    }];
    let change = FileDiffStat {
        path: PathBuf::from("apps/shop/lib/shop_web/item_controller.ex"),
        ..router_change(&[], &[], &[3])
    };
    let http = measured(ROUTER, change, &narrowing);

    assert_eq!(http.routes.len(), 1);
    assert_eq!(http.routes[0].path, "/api/v1/items/:id");
    assert_eq!(http.routes[0].change, RouteChange::Action);
    // Matched by the end of the path, past a base URL not seen.
    assert_eq!(http.routes[0].callers.len(), 1);
    assert!(!http.routes[0].callers[0].exact);
}

#[test]
fn a_route_whose_line_changed_is_changed_and_a_new_one_is_not() {
    let head = ROUTER
        .replace("OrderController, :create", "CheckoutController, :create")
        .replace(
            "  end\nend",
            "    get \"/v1/new\", NewController, :index\n  end\nend",
        );
    let change = router_change(
        &[(7, "    post \"/v1/orders\", OrderController, :create")],
        &[
            (7, "    post \"/v1/orders\", CheckoutController, :create"),
            (8, "    get \"/v1/new\", NewController, :index"),
        ],
        &[7, 8],
    );
    let http = measured(&head, change, &[]);

    let routes: Vec<(&str, RouteChange, usize)> = http
        .routes
        .iter()
        .map(|r| (r.path.as_str(), r.change, r.callers.len()))
        .collect();
    assert_eq!(routes, [("/api/v1/orders", RouteChange::Changed, 1)]);
}

#[test]
fn nothing_is_read_when_no_route_is_concerned() {
    let change = FileDiffStat {
        path: PathBuf::from("apps/billing/lib/billing/shop.ex"),
        ..router_change(&[], &[], &[2])
    };
    assert_eq!(measured(ROUTER, change, &[]), Http::default());
}

#[test]
fn a_call_matches_the_end_of_a_route() {
    assert_eq!(matches("/api/v1/items", "/api/v1/items"), Some(true));
    assert_eq!(
        matches("/api/v1/items/:id", "/v1/items/:param"),
        Some(false)
    );
    assert_eq!(matches("/api/v1/items/:id", "/v1/items/42"), Some(false));
    // One segment alone must be the whole route.
    assert_eq!(matches("/api/items", "/items"), None);
    assert_eq!(matches("/items", "/items"), Some(true));
    // Parameters alone match nothing.
    assert_eq!(matches("/a/:id", "/:param/:param"), None);
    assert!(
        verb_fits("*", Some("post")) && verb_fits("get", None) && !verb_fits("get", Some("post"))
    );
}
