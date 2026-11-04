use super::*;

fn paths(language: &str, text: &str) -> Vec<(String, Option<String>, Option<String>)> {
    calls(Path::new("x"), language, text)
        .into_iter()
        .map(|c| (c.client, c.verb, c.path))
        .collect()
}

fn call(
    client: &str,
    verb: Option<&str>,
    path: Option<&str>,
) -> (String, Option<String>, Option<String>) {
    (
        client.to_string(),
        verb.map(str::to_string),
        path.map(str::to_string),
    )
}

#[test]
fn a_path_is_normalized_for_matching() {
    assert_eq!(
        normalize("https://api.example.com/v1/items?x=1").as_deref(),
        Some("/v1/items")
    );
    assert_eq!(
        normalize(":param/v1/items/:param").as_deref(),
        Some("/v1/items/:param")
    );
    assert_eq!(normalize("items").as_deref(), Some("/items"));
    assert_eq!(normalize("https://example.com"), None);
    assert_eq!(normalize(":param"), None);
}

#[test]
fn elixir_clients_are_read_with_their_verb_and_path() {
    let source = r##"defmodule Billing.Client do
  def items, do: Tesla.get(client(), "/v1/items")
  def item(id), do: Req.get!(base_url: url(), url: "/v1/items/#{id}")
  def make(body), do: HTTPoison.post("#{@base}/v1/items", body)
  def raw(req), do: Finch.build(:delete, "https://svc/v1/items/1")
  def dynamic(path), do: Tesla.get(client(), path)
  # Tesla.get(client(), "/commented")
  def other, do: Map.get(%{}, "/v1/items")
end
"##;
    assert_eq!(
        paths("Elixir", source),
        [
            call("Tesla", Some("get"), Some("/v1/items")),
            call("Req", Some("get"), Some("/v1/items/:param")),
            call("HTTPoison", Some("post"), Some("/v1/items")),
            call("Finch", Some("delete"), Some("/v1/items/1")),
            call("Tesla", Some("get"), None),
        ]
    );
}

#[test]
fn a_tesla_client_module_calls_verbs_bare() {
    let source = "defmodule Api do\n  use Tesla\n  def list, do: get(\"/orders\")\n  def map, do: Map.get(%{}, :a)\nend\n";
    assert_eq!(
        paths("Elixir", source),
        [call("Tesla", Some("get"), Some("/orders"))]
    );
    // Without `use Tesla`, a bare `get` is any function.
    assert!(paths("Elixir", "def list, do: get(\"/orders\")\n").is_empty());
}

#[test]
fn script_and_python_clients_are_read_too() {
    let script = "const a = await fetch(`/api/orders/${id}`, { method: 'DELETE' });\nconst b = axios.post('/api/orders', body);\nconst c = fetch(url);\n";
    assert_eq!(
        paths("TypeScript", script),
        [
            call("fetch", Some("delete"), Some("/api/orders/:param")),
            call("axios", Some("post"), Some("/api/orders")),
            call("fetch", None, None),
        ]
    );
    let python =
        "r = requests.get(f\"{BASE}/v1/items/{item_id}\")\nhttpx.put('/v1/items/1', json=x)\n";
    assert_eq!(
        paths("Python", python),
        [
            call("requests", Some("get"), Some("/v1/items/:param")),
            call("httpx", Some("put"), Some("/v1/items/1")),
        ]
    );
}

#[test]
fn a_parenthesis_after_a_multibyte_character_is_read() {
    let source = r##"defmodule Billing.Client do
  def items(n), do: Tesla.get(client(), "/v1/items") |> tag("N°(#{n})")
end
"##;
    assert_eq!(
        paths("Elixir", source),
        vec![call("Tesla", Some("get"), Some("/v1/items"))]
    );
}
