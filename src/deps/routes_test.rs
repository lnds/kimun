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
    assert!(serves("/orders/new", "/orders/:_"));
    assert!(serves("/", "/"));
    assert!(serves("/files/*path", "/files/a/b"));

    assert!(!serves("/orders", "/orders/42"));
    assert!(!serves("/orders/:id", "/orders"));
    assert!(!serves("/orders/:id", "/users/42"));
    assert!(!serves("/", "/orders"));
    assert!(!serves("/files/*path", "/other/a"));
}
