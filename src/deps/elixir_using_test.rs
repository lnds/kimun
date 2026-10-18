use std::path::PathBuf;

use super::*;

const WEB: &str = r#"
defmodule ShopWeb do
  alias ShopWeb.Helpers.Format

  def router do
    quote do
      use Phoenix.Router
    end
  end

  def live_view do
    quote do
      use Phoenix.LiveView
      unquote(html_helpers())
    end
  end

  defp html_helpers do
    quote do
      import ShopWeb.CoreComponents
      alias ShopWeb.Layouts
      Format.setup()
    end
  end

  defmacro __using__(which) when is_atom(which) do
    apply(__MODULE__, which, [])
  end
end
"#;

const COMPONENTS: &str = r#"
defmodule ShopWeb.CoreComponents do
  def button(assigns), do: assigns
  def input(assigns), do: assigns
  defp private_helper(x), do: x
end
"#;

const LIVE: &str = r#"
defmodule ShopWeb.OrderLive do
  use ShopWeb, :live_view

  def render(assigns) do
    ~H"""
    <Layouts.app flash={@flash}>
      <.button>Save</.button>
    </Layouts.app>
    """
  end
end
"#;

fn call(module: &str, function: &str) -> Reference {
    Reference {
        module: module.to_string(),
        function: Some(function.to_string()),
    }
}

#[test]
fn a_use_with_a_name_injects_that_function_and_what_it_splices() {
    let found = injection(WEB, Some("live_view")).unwrap();
    assert_eq!(found.whole, ["ShopWeb.CoreComponents"]);
    // Read with the aliases of the module that injects it.
    assert!(
        found
            .refs
            .contains(&call("ShopWeb.Helpers.Format", "setup"))
    );
    assert!(!found.refs.iter().any(|r| r.module == "Phoenix.Router"));
    // The aliases declared inside are the user's.
    let mut aliases = Aliases::default();
    aliases.inherit(&found.aliases);
    let (_, code) = Aliases::of("Layouts.app()");
    assert_eq!(
        references(&code, &aliases),
        [call("ShopWeb.Layouts", "app")]
    );
}

#[test]
fn a_use_needs_the_function_it_runs() {
    assert!(injection(WEB, Some("channel")).is_none());
    assert!(injection(COMPONENTS, None).is_none());
    assert!(injection(WEB, None).is_some());
}

fn sources() -> Vec<Source> {
    [
        ("lib/shop_web.ex", WEB),
        ("lib/shop_web/components/core_components.ex", COMPONENTS),
        (
            "lib/shop_web/components/layouts.ex",
            "defmodule ShopWeb.Layouts do\n  def app(a), do: a\nend\n",
        ),
        ("lib/shop_web/live/order_live.ex", LIVE),
    ]
    .into_iter()
    .map(|(path, text)| Source {
        path: PathBuf::from(path),
        language: "Elixir".to_string(),
        text: text.to_string(),
        templates: Vec::new(),
    })
    .collect()
}

#[test]
fn a_user_takes_what_the_use_injects_and_what_the_import_answers() {
    let sources = sources();
    let mut parsed: Vec<Option<ElixirFile>> = sources
        .iter()
        .map(|s| Some(elixir::parse_with(&s.text, &s.templates)))
        .collect();
    let index: HashMap<String, usize> = parsed
        .iter()
        .enumerate()
        .flat_map(|(i, p)| {
            p.iter()
                .flat_map(move |p| p.defines.iter().map(move |d| (d.clone(), i)))
        })
        .collect();
    let resolve = |_: usize, module: &str| index.get(module).copied();
    complete(&mut parsed, &sources, &resolve);

    let live = parsed[3].as_ref().unwrap();
    for expected in [
        call("ShopWeb", "live_view"),
        call("ShopWeb.CoreComponents", "button"),
        call("ShopWeb.Layouts", "app"),
    ] {
        assert!(
            live.refs.contains(&expected),
            "{expected:?} in {:?}",
            live.refs
        );
    }
    assert!(!live.refs.contains(&call("ShopWeb.CoreComponents", "input")));
    assert_eq!(live.whole, ["ShopWeb.CoreComponents"]);
}
