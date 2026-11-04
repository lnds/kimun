use super::*;

fn route(path: &str, module: &str) -> Route {
    Route {
        path: path.to_string(),
        module: module.to_string(),
    }
}

const ROUTER: &str = r#"
defmodule ShopWeb.Router do
  use ShopWeb, :router

  scope "/", ShopWeb do
    pipe_through :browser

    get "/", PageController, :home
    # get "/old", OldController, :index

    live_session :default do
      live "/orders", OrdersLive.Index, :index
      live "/orders/:id", OrdersLive.Show, :show
    end

    scope "/admin", Admin do
      live "/", DashboardLive
      resources "/users", UserController
    end
  end

  scope "/api" do
    post "/orders", ShopWeb.Api.OrderController, :create
  end

  get "/health", ShopWeb.HealthController, :check
end
"#;

#[test]
fn a_scope_adds_its_path_and_its_alias_to_what_is_inside() {
    assert_eq!(
        routes(ROUTER),
        [
            route("/", "ShopWeb.PageController"),
            route("/orders", "ShopWeb.OrdersLive.Index"),
            route("/orders/:id", "ShopWeb.OrdersLive.Show"),
            route("/admin", "ShopWeb.Admin.DashboardLive"),
            route("/admin/users", "ShopWeb.Admin.UserController"),
            route("/admin/users/new", "ShopWeb.Admin.UserController"),
            route("/admin/users/:id", "ShopWeb.Admin.UserController"),
            route("/admin/users/:id/edit", "ShopWeb.Admin.UserController"),
            route("/api/orders", "ShopWeb.Api.OrderController"),
            route("/health", "ShopWeb.HealthController"),
        ]
    );
}

#[test]
fn a_file_that_is_no_router_has_no_routes() {
    assert!(routes("defmodule Shop.Orders do\n  def get(id), do: id\nend\n").is_empty());
}

#[test]
fn a_test_requests_the_first_path_of_each_request() {
    let test = r#"
test "lists", %{conn: conn} do
  {:ok, view, _} = live(conn, ~p"/orders")
  conn = get(conn, "/orders/#{order.id}?tab=items")
  assert redirected_to(post(conn, ~p"/api/orders", %{})) == "/done"
  conn |> visit("/admin")
  assert_redirect(view, "/elsewhere")
  budget("/not/a/request")
end
"#;
    assert_eq!(
        requests(test),
        ["/admin", "/api/orders", "/orders", "/orders/:_?tab=items"]
    );
}

#[test]
fn a_request_without_a_path_literal_asks_for_nothing_known() {
    assert!(requests("live(conn, path)\nget(conn, Routes.page_path(conn, :home))\n").is_empty());
}

#[test]
fn a_route_serves_the_requests_that_match_it_segment_by_segment() {
    assert!(serves("/orders", "/orders"));
    assert!(serves("/orders", "/orders/?page=2"));
    assert!(serves("/orders/:id", "/orders/42"));
    assert!(serves("/orders/:id", "/orders/:_"));
    assert!(serves("/", "/"));
    assert!(serves("/files/*path", "/files/a/b"));

    assert!(!serves("/orders", "/orders/42"));
    assert!(!serves("/orders/new", "/orders/:_"));
    assert!(!serves("/orders", "/:_"));
    assert!(!serves("/orders/:id", "/orders"));
    assert!(!serves("/orders/:id", "/users/42"));
    assert!(!serves("/", "/orders"));
    assert!(!serves("/files/*path", "/other/a"));
}

#[test]
fn a_request_is_read_from_a_line_with_text_beyond_ascii() {
    let test = "assert html =~ \"Sesión\" and get(conn, \"/orders\")\nrevisión(conn)\n";
    assert_eq!(requests(test), ["/orders"]);
}

#[test]
fn each_route_is_declared_with_its_verb_action_and_line() {
    let found = declared(ROUTER);
    let show = |d: &Declared| {
        format!(
            "{} {} {}#{} @{}",
            d.verb,
            d.path,
            d.module.rsplit('.').next().unwrap(),
            d.action.as_deref().unwrap_or("-"),
            d.line
        )
    };
    let lines: Vec<String> = found.iter().map(show).collect();
    assert_eq!(lines[0], "get / PageController#home @8");
    assert_eq!(lines[1], "get /orders Index#index @12");
    assert_eq!(lines[3], "get /admin DashboardLive#- @17");
    assert!(lines.contains(&"delete /admin/users/:id UserController#delete @18".to_string()));
    assert!(lines.contains(&"post /api/orders OrderController#create @23".to_string()));
    let any = declared("scope \"/x\" do\n  match :*, \"/hook\", HookController, :any\nend\n");
    assert_eq!(
        (any[0].verb.as_str(), any[0].path.as_str()),
        ("*", "/x/hook")
    );
    assert_eq!(any[0].action.as_deref(), Some("any"));
}
